//! `@reference` scanning + resolution for the editor's clickable links.
//!
//! Resolution order for `@token`:
//!   1. path-like (`@RTK.md`, `@~/.claude/x.md`, `@./rel`, `@dir/file`)
//!      -> resolve against the current file's dir, then `~`, then `~/.claude`.
//!   2. name-like -> look up in the skills/commands/agents catalog.
//! Unresolved refs are still returned (marked) so the UI can dim them.

use crate::index::Catalog;
use crate::jail;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Ref {
    /// Byte offset of the `@` in the source.
    pub start: usize,
    /// Byte offset just past the token.
    pub end: usize,
    /// The token text without the leading `@`.
    pub token: String,
    /// Resolved path relative to `~/.claude`, or None if unresolved.
    pub target: Option<String>,
}

/// Scan `body` for `@` references, resolving each against `file_dir`
/// (the dir of the file being edited, relative to root) and the catalog.
pub fn scan(body: &str, file_dir: &str, cat: &Catalog) -> Result<Vec<Ref>, String> {
    let root = jail::root()?;
    let bytes = body.as_bytes();
    let mut refs = vec![];
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' && at_token_boundary(bytes, i) {
            let start = i;
            let mut j = i + 1;
            while j < bytes.len() && is_token_byte(bytes[j]) {
                j += 1;
            }
            let scan_end = j;
            // Trailing sentence punctuation ('.'/':') is prose, not part of the
            // ref (e.g. "see @config.md." -> token `config.md`, dot excluded).
            let mut end = j;
            while end > start + 1 && matches!(bytes[end - 1], b'.' | b':') {
                end -= 1;
            }
            if end > start + 1 {
                let token = &body[start + 1..end];
                let target = resolve(token, file_dir, &root, cat);
                refs.push(Ref { start, end, token: token.to_string(), target });
            }
            i = scan_end;
        } else {
            i += 1;
        }
    }
    Ok(refs)
}

/// `@` only starts a ref at the start of input or after whitespace/`(`—avoids
/// matching emails and `foo@bar`.
fn at_token_boundary(b: &[u8], i: usize) -> bool {
    i == 0 || matches!(b[i - 1], b' ' | b'\t' | b'\n' | b'\r' | b'(' | b'[')
}

fn is_token_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'.' | b'/' | b'_' | b'-' | b'~' | b':')
}

fn resolve(token: &str, file_dir: &str, root: &Path, cat: &Catalog) -> Option<String> {
    if looks_like_path(token) {
        resolve_path(token, file_dir, root)
    } else {
        resolve_name(token, cat)
    }
}

fn looks_like_path(t: &str) -> bool {
    t.contains('/') || t.contains('.') || t.starts_with('~')
}

fn resolve_path(token: &str, file_dir: &str, root: &Path) -> Option<String> {
    let candidates: Vec<PathBuf> = if let Some(rest) = token.strip_prefix("~/.claude/") {
        vec![root.join(rest)]
    } else if let Some(rest) = token.strip_prefix('~') {
        let home = crate::scope::home_dir().ok()?;
        vec![home.join(rest.trim_start_matches('/'))]
    } else {
        // Relative to the file's dir, then to root.
        vec![root.join(file_dir).join(token), root.join(token)]
    };
    for c in candidates {
        if c.is_file() {
            return rel_to_root(&c, root);
        }
    }
    None
}

fn resolve_name(token: &str, cat: &Catalog) -> Option<String> {
    let hit = cat
        .skills
        .iter()
        .chain(&cat.commands)
        .chain(&cat.agents)
        .find(|e| e.name == token);
    hit.map(|e| e.path.clone())
}

/// Only return targets that stay inside root (defense in depth).
fn rel_to_root(p: &Path, root: &Path) -> Option<String> {
    let canon = p.canonicalize().ok()?;
    let root_c = root.canonicalize().ok()?;
    canon
        .strip_prefix(&root_c)
        .ok()
        .map(|r| r.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Entry;
    use crate::testutil::with_claude;
    use std::fs;

    #[test]
    fn resolves_paths_and_names_and_marks_missing() {
        with_claude(|claude| {
            fs::write(claude.join("RTK.md"), "x").unwrap();
            let cat = Catalog {
                agents: vec![Entry {
                    name: "backend".into(),
                    description: String::new(),
                    path: "agents/backend.md".into(),
                    group: String::new(),
                }],
                ..Default::default()
            };
            let body = "see @RTK.md and @backend and @nope.md";
            let refs = scan(body, "", &cat).unwrap();
            assert_eq!(refs.len(), 3);
            assert_eq!(refs[0].target.as_deref(), Some("RTK.md"));
            assert_eq!(refs[1].target.as_deref(), Some("agents/backend.md"));
            assert_eq!(refs[2].target, None);
        });
    }

    #[test]
    fn ignores_email_like() {
        with_claude(|_| {
            let cat = Catalog::default();
            let refs = scan("mail me at foo@bar.com", "", &cat).unwrap();
            assert!(refs.is_empty(), "foo@bar should not match (no boundary)");
        });
    }

    #[test]
    fn trailing_punctuation_excluded() {
        with_claude(|claude| {
            fs::write(claude.join("RTK.md"), "x").unwrap();
            let cat = Catalog::default();
            let refs = scan("see @RTK.md.", "", &cat).unwrap();
            assert_eq!(refs.len(), 1);
            assert_eq!(refs[0].token, "RTK.md", "trailing '.' must be excluded");
            assert_eq!(refs[0].target.as_deref(), Some("RTK.md"));
            // The decoration range must not cover the trailing dot.
            assert_eq!(refs[0].end, refs[0].start + 1 + "RTK.md".len());
        });
    }
}
