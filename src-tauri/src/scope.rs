//! Active editing scope. The app edits either the global `~/.claude` or a
//! specific project's Claude config (`<proj>/.claude` plus the project-root
//! `CLAUDE.md` / `.mcp.json`). Held in a process-global lock that `jail` reads,
//! so switching scope transparently retargets every filesystem command.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[derive(Clone, PartialEq)]
pub enum Kind {
    Global,
    Project,
}

#[derive(Clone)]
pub struct Scope {
    pub kind: Kind,
    /// The jail root: the config dir every relative path resolves under.
    /// Global -> ~/.claude ; Project -> <proj>/.claude.
    pub config_dir: PathBuf,
    /// Project directory (parent of `.claude`), only in Project scope. Used to
    /// reach the two whitelisted project-root files.
    pub project_dir: Option<PathBuf>,
}

impl Scope {
    fn global() -> Result<Self, String> {
        Ok(Self { kind: Kind::Global, config_dir: global_config_dir()?, project_dir: None })
    }
}

fn global_config_dir() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME not set")?;
    Ok(PathBuf::from(home).join(".claude"))
}

// Process-global active scope. Lazily initialised to Global on first read.
static SCOPE: RwLock<Option<Scope>> = RwLock::new(None);

/// The active scope's config dir (the jail root).
pub fn config_dir() -> Result<PathBuf, String> {
    Ok(current()?.config_dir)
}

/// The MCP config file for the active scope: global `~/.claude.json`, or the
/// project's root `.mcp.json`.
pub fn mcp_file() -> Result<PathBuf, String> {
    let s = current()?;
    match (&s.kind, &s.project_dir) {
        (Kind::Project, Some(proj)) => Ok(proj.join(".mcp.json")),
        _ => {
            let home = std::env::var_os("HOME").ok_or("HOME not set")?;
            Ok(PathBuf::from(home).join(".claude.json"))
        }
    }
}

/// Absolute path for one of the whitelisted project-root files. `name` is
/// `CLAUDE.md` or `.mcp.json`. Global scope maps these to `~/.claude/CLAUDE.md`
/// and `~/.claude.json` respectively (the global equivalents).
pub fn root_file(name: &str) -> Result<PathBuf, String> {
    if name != "CLAUDE.md" && name != ".mcp.json" {
        return Err("not a whitelisted root file".into());
    }
    let s = current()?;
    match (&s.kind, &s.project_dir) {
        (Kind::Project, Some(proj)) => Ok(proj.join(name)),
        _ if name == ".mcp.json" => mcp_file(),
        _ => Ok(s.config_dir.join("CLAUDE.md")),
    }
}

fn current() -> Result<Scope, String> {
    {
        let g = SCOPE.read().map_err(|_| "scope lock poisoned")?;
        if let Some(s) = g.as_ref() {
            return Ok(s.clone());
        }
    }
    let s = Scope::global()?;
    *SCOPE.write().map_err(|_| "scope lock poisoned")? = Some(s.clone());
    Ok(s)
}

fn set(s: Scope) -> Result<(), String> {
    *SCOPE.write().map_err(|_| "scope lock poisoned")? = Some(s);
    Ok(())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ScopeInfo {
    /// "global" | "project"
    pub kind: String,
    /// Human label: "Global (~/.claude)" or the project folder name.
    pub label: String,
    /// The config dir path (for display).
    pub path: String,
}

fn info(s: &Scope) -> ScopeInfo {
    match &s.kind {
        Kind::Global => ScopeInfo {
            kind: "global".into(),
            label: "Global (~/.claude)".into(),
            path: s.config_dir.to_string_lossy().into_owned(),
        },
        Kind::Project => {
            let name = s
                .project_dir
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "project".into());
            ScopeInfo {
                kind: "project".into(),
                label: name,
                path: s.config_dir.to_string_lossy().into_owned(),
            }
        }
    }
}

#[tauri::command]
pub fn scope_get() -> Result<ScopeInfo, String> {
    Ok(info(&current()?))
}

#[tauri::command]
pub fn scope_set_global() -> Result<ScopeInfo, String> {
    let s = Scope::global()?;
    set(s.clone())?;
    Ok(info(&s))
}

/// Result of trying to open a project folder.
#[derive(Serialize)]
pub struct OpenResult {
    /// "opened" (scope switched) or "no-claude" (folder valid but has no .claude)
    pub status: String,
    pub info: Option<ScopeInfo>,
}

/// Open a project folder. If it has a `.claude/` dir, switch to Project scope.
/// Otherwise report "no-claude" so the UI can offer to create it.
#[tauri::command]
pub fn scope_open_project(path: String) -> Result<OpenResult, String> {
    let proj = PathBuf::from(&path);
    if !proj.is_dir() {
        return Err("not a directory".into());
    }
    let claude = proj.join(".claude");
    if !claude.is_dir() {
        return Ok(OpenResult { status: "no-claude".into(), info: None });
    }
    let s = project_scope(&proj);
    set(s.clone())?;
    Ok(OpenResult { status: "opened".into(), info: Some(info(&s)) })
}

/// Create `<path>/.claude` and switch to Project scope there.
#[tauri::command]
pub fn scope_create_claude(path: String) -> Result<ScopeInfo, String> {
    let proj = PathBuf::from(&path);
    if !proj.is_dir() {
        return Err("not a directory".into());
    }
    std::fs::create_dir_all(proj.join(".claude")).map_err(|e| e.to_string())?;
    let s = project_scope(&proj);
    set(s.clone())?;
    Ok(info(&s))
}

fn project_scope(proj: &Path) -> Scope {
    Scope {
        kind: Kind::Project,
        config_dir: proj.join(".claude"),
        project_dir: Some(proj.to_path_buf()),
    }
}

#[cfg(test)]
pub(crate) fn set_global_for_test() {
    set(Scope::global().unwrap()).unwrap();
}

#[cfg(test)]
pub(crate) fn set_project_for_test(proj: &Path) {
    set(project_scope(proj)).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::with_claude;

    #[test]
    fn open_project_detects_claude_or_not() {
        with_claude(|claude| {
            let base = claude.parent().unwrap();
            // A project WITH .claude/.
            let with = base.join("proj_with");
            std::fs::create_dir_all(with.join(".claude")).unwrap();
            let r = scope_open_project(with.to_string_lossy().into()).unwrap();
            assert_eq!(r.status, "opened");
            assert_eq!(config_dir().unwrap(), with.join(".claude"));

            // A project WITHOUT .claude/.
            let without = base.join("proj_bare");
            std::fs::create_dir_all(&without).unwrap();
            let r = scope_open_project(without.to_string_lossy().into()).unwrap();
            assert_eq!(r.status, "no-claude");
            set_global_for_test();
        });
    }

    #[test]
    fn scope_switches_mcp_and_root_files() {
        with_claude(|claude| {
            let base = claude.parent().unwrap();
            let proj = base.join("proj_mcp");
            std::fs::create_dir_all(proj.join(".claude")).unwrap();
            set_project_for_test(&proj);
            assert_eq!(mcp_file().unwrap(), proj.join(".mcp.json"));
            assert_eq!(root_file("CLAUDE.md").unwrap(), proj.join("CLAUDE.md"));
            assert_eq!(root_file(".mcp.json").unwrap(), proj.join(".mcp.json"));

            set_global_for_test();
            assert_eq!(mcp_file().unwrap(), base.join(".claude.json"));
            assert_eq!(root_file("CLAUDE.md").unwrap(), claude.join("CLAUDE.md"));
        });
    }

    #[test]
    fn create_claude_makes_dir_and_switches() {
        with_claude(|claude| {
            let proj = claude.parent().unwrap().join("proj_new");
            std::fs::create_dir_all(&proj).unwrap();
            scope_create_claude(proj.to_string_lossy().into()).unwrap();
            assert!(proj.join(".claude").is_dir());
            assert_eq!(config_dir().unwrap(), proj.join(".claude"));
            set_global_for_test();
        });
    }
}
