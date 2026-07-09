//! Catalog of skills / commands / agents. Used for cards in the views and for
//! resolving `@name` references. Cheap to rebuild; the frontend caches it.

use crate::frontmatter;
use crate::jail;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub description: String,
    /// Path relative to `~/.claude`, e.g. `skills/foo/SKILL.md`.
    pub path: String,
    /// For commands: the namespace dir (e.g. `dev`), else "".
    pub group: String,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Catalog {
    pub skills: Vec<Entry>,
    pub commands: Vec<Entry>,
    pub agents: Vec<Entry>,
}

pub fn build() -> Result<Catalog, String> {
    let root = jail::root()?;
    Ok(Catalog {
        skills: scan_skills(&root),
        commands: scan_commands(&root),
        agents: scan_agents(&root),
    })
}

/// skills/<name>/SKILL.md
fn scan_skills(root: &Path) -> Vec<Entry> {
    let dir = root.join("skills");
    let mut out = vec![];
    for sub in list_dirs(&dir) {
        let skill_md = sub.join("SKILL.md");
        if skill_md.is_file() {
            out.push(entry_from(root, &skill_md, dir_name(&sub), ""));
        }
    }
    sort(&mut out);
    out
}

/// commands/*.md and commands/<ns>/*.md
fn scan_commands(root: &Path) -> Vec<Entry> {
    let dir = root.join("commands");
    let mut out = vec![];
    for f in list_md(&dir) {
        out.push(entry_from(root, &f, file_stem(&f), ""));
    }
    for ns in list_dirs(&dir) {
        let group = dir_name(&ns);
        for f in list_md(&ns) {
            out.push(entry_from(root, &f, file_stem(&f), &group));
        }
    }
    sort(&mut out);
    out
}

/// agents/*.md
fn scan_agents(root: &Path) -> Vec<Entry> {
    let dir = root.join("agents");
    let mut out: Vec<Entry> = list_md(&dir)
        .iter()
        .map(|f| entry_from(root, f, file_stem(f), ""))
        .collect();
    sort(&mut out);
    out
}

/// Build an Entry, pulling name/description from frontmatter when present,
/// falling back to `fallback_name` and an empty description.
fn entry_from(root: &Path, file: &Path, fallback_name: String, group: &str) -> Entry {
    let content = fs::read_to_string(file).unwrap_or_default();
    let (fm, _) = frontmatter::split(&content);
    let (mut name, mut description) = (fallback_name.clone(), String::new());
    if let Some(fm) = fm {
        for field in frontmatter::parse_fields(fm) {
            if let frontmatter::Field::Scalar { key, value } = field {
                match key.as_str() {
                    "name" if !value.is_empty() => name = value,
                    "description" => description = value,
                    _ => {}
                }
            }
        }
    }
    Entry { name, description, path: rel(root, file), group: group.to_string() }
}

fn list_dirs(dir: &Path) -> Vec<PathBuf> {
    read(dir).filter(|p| p.is_dir()).collect()
}
fn list_md(dir: &Path) -> Vec<PathBuf> {
    read(dir)
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect()
}
fn read(dir: &Path) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
}
fn dir_name(p: &Path) -> String {
    p.file_name().unwrap_or_default().to_string_lossy().into_owned()
}
fn file_stem(p: &Path) -> String {
    p.file_stem().unwrap_or_default().to_string_lossy().into_owned()
}
fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).to_string_lossy().into_owned()
}
fn sort(v: &mut [Entry]) {
    v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
}
