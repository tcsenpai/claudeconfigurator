//! Tauri commands: the frontend's entire API surface. Every path goes through
//! `jail::resolve` first. Writes back up (rotating) then write atomically.

use crate::backup;
use crate::frontmatter::{self, Field};
use crate::index::{self, Catalog};
use crate::jail;
use crate::refs::{self, Ref};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Sentinel prefix for a whitelisted project-root file (CLAUDE.md / .mcp.json),
/// which lives OUTSIDE the config dir in project scope. `@root/CLAUDE.md`.
const ROOT_PREFIX: &str = "@root/";

/// Resolve either a normal in-config-dir path or a `@root/<name>` root file.
fn resolve_any(path: &str) -> Result<PathBuf, String> {
    if let Some(name) = path.strip_prefix(ROOT_PREFIX) {
        // .mcp.json / ~/.claude.json must only be mutated via the surgical mcp
        // commands, never whole-file overwritten through the generic editor.
        if name == ".mcp.json" {
            return Err("edit MCP servers via the MCP tab, not as a raw file".into());
        }
        jail::resolve_root_file(name)
    } else {
        jail::resolve(path)
    }
}

/// A file listing entry for the Files view.
#[derive(Serialize)]
pub struct FileItem {
    pub name: String,
    pub path: String,
}

/// Full parsed file for the editor.
#[derive(Serialize)]
pub struct FileDoc {
    pub path: String,
    pub fields: Vec<Field>,
    pub body: String,
    pub raw: String,
    /// dir of this file relative to root, for @-ref resolution.
    pub dir: String,
}

/// List the root-level `.md` files (CLAUDE.md + adjacent) for the Files view.
/// In project scope the project-root `CLAUDE.md` (which lives above the config
/// dir) is included as `@root/CLAUDE.md`.
#[tauri::command]
pub fn list_root_md() -> Result<Vec<FileItem>, String> {
    let root = jail::root()?;
    let mut items: Vec<FileItem> = fs::read_dir(&root)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "md"))
        .map(|p| FileItem {
            name: p.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            path: p.strip_prefix(&root).unwrap_or(&p).to_string_lossy().into_owned(),
        })
        .collect();
    // Project-root CLAUDE.md (outside the config dir) via the root-file route.
    if let Ok(claude) = jail::resolve_root_file("CLAUDE.md") {
        if claude.is_file() && claude.parent() != Some(root.as_path()) {
            items.push(FileItem {
                name: "CLAUDE.md (project root)".into(),
                path: format!("{ROOT_PREFIX}CLAUDE.md"),
            });
        }
    }
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(items)
}

/// Read + parse a file into fields + body.
#[tauri::command]
pub fn read_file(path: String) -> Result<FileDoc, String> {
    let abs = resolve_any(&path)?;
    let raw = fs::read_to_string(&abs).map_err(|e| e.to_string())?;
    let (fm, body) = frontmatter::split(&raw);
    let fields = fm.map(frontmatter::parse_fields).unwrap_or_default();
    let root = jail::root()?;
    let dir = abs
        .parent()
        .and_then(|p| p.strip_prefix(&root).ok())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(FileDoc { path, fields, body: body.to_string(), raw, dir })
}

/// Write a file assembled from edited fields + body. Backs up first.
/// `validate_json` = true parses the result as JSON and refuses bad writes
/// (used for settings.json).
#[tauri::command]
pub fn write_file(
    path: String,
    fields: Vec<Field>,
    body: String,
    validate_json: bool,
) -> Result<(), String> {
    let abs = resolve_any(&path)?;
    let content = if fields.is_empty() {
        body
    } else {
        let yaml = frontmatter::fields_to_yaml(&fields)?;
        frontmatter::join(&yaml, &body)
    };
    if validate_json {
        serde_json::from_str::<serde_json::Value>(&content)
            .map_err(|e| format!("invalid JSON, not saved: {e}"))?;
    }
    backup::rotate(&abs)?;
    atomic_write(&abs, &content)
}

/// Write already-assembled raw content (settings.json / plain files).
#[tauri::command]
pub fn write_raw(path: String, content: String, validate_json: bool) -> Result<(), String> {
    write_file(path, vec![], content, validate_json)
}

/// Build the skills/commands/agents catalog.
#[tauri::command]
pub fn catalog() -> Result<Catalog, String> {
    index::build()
}

/// Scan a body for @-references given the editing file's dir. Rebuilds the
/// catalog server-side so the frontend need not round-trip it.
#[tauri::command]
pub fn scan_refs(body: String, dir: String) -> Result<Vec<Ref>, String> {
    let cat = index::build()?;
    refs::scan(&body, &dir, &cat)
}

fn atomic_write(target: &Path, content: &str) -> Result<(), String> {
    let tmp = target.with_extension("cctmp");
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    fs::rename(&tmp, target).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::with_claude;

    #[test]
    fn writes_project_root_claude_md_and_backs_up() {
        with_claude(|claude| {
            let proj = claude.parent().unwrap().join("wproj");
            fs::create_dir_all(proj.join(".claude")).unwrap();
            fs::write(proj.join("CLAUDE.md"), "old\n").unwrap();
            crate::scope::set_project_for_test(&proj);

            write_file("@root/CLAUDE.md".into(), vec![], "new\n".into(), false).unwrap();
            // Wrote the PROJECT ROOT file, not <proj>/.claude/CLAUDE.md.
            assert_eq!(fs::read_to_string(proj.join("CLAUDE.md")).unwrap(), "new\n");
            // Backup of the prior content landed under backups/_root/.
            let bak = proj.join(".claude/backups/_root/CLAUDE.md.0.bak");
            assert_eq!(fs::read_to_string(bak).unwrap(), "old\n");

            crate::scope::set_global_for_test();
        });
    }

    #[test]
    fn resolve_any_rejects_escape_and_raw_mcp() {
        with_claude(|_| {
            assert!(resolve_any("@root/../../etc/passwd").is_err());
            assert!(resolve_any("@root/secrets.env").is_err());
            assert!(resolve_any("@root/.mcp.json").is_err()); // must use MCP tab
            assert!(resolve_any("../escape").is_err());
        });
    }
}
