//! Create new config entries (skill / command / agent / root .md) from a name
//! + template, or import existing files/folders from anywhere on disk into
//! ~/.claude. Destinations are always jailed; sources for import are read-only
//! and may live outside the jail. Never overwrites: an existing target errors.

use crate::jail;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Skill,
    Command,
    Agent,
    File,
}

/// Compute the destination path (relative to ~/.claude) for a new entry.
/// `namespace` only applies to commands (optional sub-dir).
fn dest_rel(kind: Kind, name: &str, namespace: Option<&str>) -> Result<String, String> {
    let name = sanitize(name)?;
    Ok(match kind {
        Kind::Skill => format!("skills/{name}/SKILL.md"),
        Kind::Agent => format!("agents/{name}.md"),
        Kind::File => format!("{name}.md"),
        Kind::Command => match namespace.map(str::trim).filter(|s| !s.is_empty()) {
            Some(ns) => format!("commands/{}/{name}.md", sanitize(ns)?),
            None => format!("commands/{name}.md"),
        },
    })
}

/// Resolve a new-entry relative path to an absolute path under ~/.claude,
/// without requiring it (or its parents) to exist yet. Safe because `rel` is
/// composed only of fixed prefixes plus sanitized single segments, so it cannot
/// contain `..` or absolute components; we still assert containment.
fn dest_abs(rel: &str) -> Result<PathBuf, String> {
    let root = jail::root()?;
    let abs = root.join(rel);
    // rel is sanitized; this is defense in depth.
    if abs.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err("invalid path".into());
    }
    Ok(abs)
}

/// Reject names that would escape or produce junk paths. Allows a single
/// path segment (letters, digits, `-`, `_`, `.`); no slashes, no `..`.
fn sanitize(name: &str) -> Result<String, String> {
    let n = name.trim();
    if n.is_empty() {
        return Err("name is empty".into());
    }
    if n.contains('/') || n.contains('\\') || n.contains("..") {
        return Err("name must be a single path segment".into());
    }
    if !n.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) {
        return Err("name may only contain letters, digits, '-', '_', '.'".into());
    }
    Ok(n.to_string())
}

fn template(kind: Kind, name: &str) -> String {
    match kind {
        Kind::Skill => format!("---\nname: {name}\ndescription: \n---\n\n# {name}\n"),
        Kind::Agent => format!("---\nname: {name}\ndescription: \n---\n\n# {name}\n"),
        Kind::Command => format!("---\ndescription: \n---\n\n# {name}\n"),
        Kind::File => String::new(),
    }
}

/// Create a new entry from a template. Returns the created path (relative to
/// ~/.claude) so the UI can open it. Errors if the target already exists.
#[tauri::command]
pub fn create_entry(kind: Kind, name: String, namespace: Option<String>) -> Result<String, String> {
    let rel = dest_rel(kind, &name, namespace.as_deref())?;
    let abs = dest_abs(&rel)?;
    if abs.exists() {
        return Err(format!("{rel} already exists"));
    }
    write_new(&abs, template(kind, &sanitize(&name)?).as_bytes())?;
    Ok(rel)
}

/// Import a single file from `src` (any path on disk) into ~/.claude as the
/// given kind/name. A skill wraps the file as skills/<name>/SKILL.md.
#[tauri::command]
pub fn import_file(
    kind: Kind,
    name: String,
    namespace: Option<String>,
    src: String,
) -> Result<String, String> {
    let src = Path::new(&src);
    if !src.is_file() {
        return Err("source is not a file".into());
    }
    let rel = dest_rel(kind, &name, namespace.as_deref())?;
    let abs = dest_abs(&rel)?;
    if abs.exists() {
        return Err(format!("{rel} already exists"));
    }
    let bytes = fs::read(src).map_err(|e| e.to_string())?;
    write_new(&abs, &bytes)?;
    Ok(rel)
}

/// Import a whole skill folder from `src` into skills/<name>/ (recursive copy).
#[tauri::command]
pub fn import_skill_dir(name: String, src: String) -> Result<String, String> {
    let src = Path::new(&src);
    if !src.is_dir() {
        return Err("source is not a directory".into());
    }
    let name = sanitize(&name)?;
    let rel = format!("skills/{name}");
    let abs = dest_abs(&rel)?;
    if abs.exists() {
        return Err(format!("{rel} already exists"));
    }
    copy_dir(src, &abs)?;
    // Open SKILL.md if present, else the folder path.
    let skill_md = format!("{rel}/SKILL.md");
    Ok(if abs.join("SKILL.md").is_file() { skill_md } else { rel })
}

/// Write a brand-new file, creating parent dirs. Caller has already confirmed
/// the target does not exist. No backup (nothing to back up for a new file).
fn write_new(abs: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(abs, bytes).map_err(|e| e.to_string())
}

/// Recursive directory copy (files + subdirs). Skips nothing; the source is
/// trusted user selection.
fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::with_claude;

    #[test]
    fn dest_paths_per_kind() {
        assert_eq!(dest_rel(Kind::Skill, "foo", None).unwrap(), "skills/foo/SKILL.md");
        assert_eq!(dest_rel(Kind::Agent, "foo", None).unwrap(), "agents/foo.md");
        assert_eq!(dest_rel(Kind::File, "foo", None).unwrap(), "foo.md");
        assert_eq!(dest_rel(Kind::Command, "foo", None).unwrap(), "commands/foo.md");
        assert_eq!(
            dest_rel(Kind::Command, "foo", Some("dev")).unwrap(),
            "commands/dev/foo.md"
        );
        assert_eq!(dest_rel(Kind::Command, "foo", Some("  ")).unwrap(), "commands/foo.md");
    }

    #[test]
    fn sanitize_rejects_escapes() {
        assert!(sanitize("../etc").is_err());
        assert!(sanitize("a/b").is_err());
        assert!(sanitize("").is_err());
        assert!(sanitize("ok-name_1.2").is_ok());
    }

    #[test]
    fn create_makes_file_and_refuses_dup() {
        with_claude(|claude| {
            let rel = create_entry(Kind::Skill, "mytool".into(), None).unwrap();
            assert_eq!(rel, "skills/mytool/SKILL.md");
            let content = fs::read_to_string(claude.join(&rel)).unwrap();
            assert!(content.contains("name: mytool"));
            // Second create must refuse.
            assert!(create_entry(Kind::Skill, "mytool".into(), None).is_err());
        });
    }

    #[test]
    fn import_file_copies_in() {
        with_claude(|claude| {
            let src = claude.parent().unwrap().join("outside.md");
            fs::write(&src, "hello").unwrap();
            let rel = import_file(Kind::File, "brought".into(), None, src.to_string_lossy().into())
                .unwrap();
            assert_eq!(rel, "brought.md");
            assert_eq!(fs::read_to_string(claude.join(&rel)).unwrap(), "hello");
        });
    }

    #[test]
    fn import_skill_dir_recursive() {
        with_claude(|claude| {
            let src = claude.parent().unwrap().join("srcskill");
            fs::create_dir_all(src.join("scripts")).unwrap();
            fs::write(src.join("SKILL.md"), "s").unwrap();
            fs::write(src.join("scripts/x.sh"), "echo").unwrap();
            let rel = import_skill_dir("imported".into(), src.to_string_lossy().into()).unwrap();
            assert_eq!(rel, "skills/imported/SKILL.md");
            assert!(claude.join("skills/imported/scripts/x.sh").is_file());
        });
    }
}
