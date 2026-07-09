//! Path jail: every filesystem path must resolve inside `~/.claude`.
//! This is the single security boundary — all fs commands go through `resolve`.

use std::path::{Path, PathBuf};

/// Root of the config surface we're allowed to touch: `~/.claude`.
pub fn root() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME not set")?;
    Ok(Path::new(&home).join(".claude"))
}

/// Resolve a caller-supplied path and guarantee it stays inside `~/.claude`.
///
/// Accepts either an absolute path already under root, or a path relative to
/// root. Rejects anything that escapes (via `..`, symlinks, or absolute paths
/// pointing elsewhere). Returns the canonical path on success.
///
/// For not-yet-existing paths (new files) we canonicalize the parent dir and
/// re-append the final component, so writes to new files still get jailed.
pub fn resolve(input: &str) -> Result<PathBuf, String> {
    let root = root()?;
    let root_c = canonical_existing(&root)?;

    let raw = {
        let p = Path::new(input);
        if p.is_absolute() { p.to_path_buf() } else { root.join(p) }
    };

    // Reject `..` components outright — cheap defense before touching the fs.
    if raw.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err("path traversal rejected".into());
    }

    let canon = match raw.canonicalize() {
        Ok(c) => c,
        Err(_) => {
            // Path may not exist yet (new file): jail the parent instead.
            let parent = raw.parent().ok_or("invalid path")?;
            let name = raw.file_name().ok_or("invalid path")?;
            let parent_c = parent
                .canonicalize()
                .map_err(|e| format!("cannot resolve parent: {e}"))?;
            parent_c.join(name)
        }
    };

    if canon.starts_with(&root_c) {
        Ok(canon)
    } else {
        Err("path escapes ~/.claude".into())
    }
}

fn canonical_existing(p: &Path) -> Result<PathBuf, String> {
    p.canonicalize().map_err(|e| format!("~/.claude not accessible: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::with_claude;
    use std::fs;

    #[test]
    fn accepts_relative_inside() {
        with_claude(|claude| {
            fs::write(claude.join("CLAUDE.md"), "x").unwrap();
            assert!(resolve("CLAUDE.md").is_ok());
        });
    }

    #[test]
    fn rejects_parent_traversal() {
        with_claude(|_| {
            assert!(resolve("../secret").is_err());
            assert!(resolve("sub/../../etc/passwd").is_err());
        });
    }

    #[test]
    fn rejects_absolute_outside() {
        with_claude(|_| {
            assert!(resolve("/etc/passwd").is_err());
        });
    }

    #[test]
    fn allows_new_file_in_root() {
        with_claude(|_| {
            // File doesn't exist yet but parent (root) does.
            assert!(resolve("newfile.md").is_ok());
        });
    }
}
