//! Tauri commands: the frontend's entire API surface. Every path goes through
//! `jail::resolve` first. Writes back up (rotating) then write atomically.

use crate::backup;
use crate::frontmatter::{self, Field};
use crate::index::{self, Catalog};
use crate::jail;
use crate::refs::{self, Ref};
use serde::Serialize;
use std::fs;
use std::path::Path;

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
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(items)
}

/// Read + parse a file into fields + body.
#[tauri::command]
pub fn read_file(path: String) -> Result<FileDoc, String> {
    let abs = jail::resolve(&path)?;
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
    let abs = jail::resolve(&path)?;
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
