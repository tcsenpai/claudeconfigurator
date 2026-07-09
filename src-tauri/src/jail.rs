//! Path jail: every filesystem path must resolve inside `~/.claude`.
//! This is the single security boundary — all fs commands go through `resolve`.

use std::path::{Path, PathBuf};

/// Root of the config surface we're allowed to touch: `~/.claude`.
pub fn root() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME not set")?;
    Ok(Path::new(&home).join(".claude"))
}

/// Resolve a caller-supplied path and guarantee its *entry point* stays inside
/// `~/.claude`. Returns the path rooted at `~/.claude` WITHOUT following
/// symlinks, so a symlink legitimately placed under `~/.claude` (e.g. a skill
/// symlinked to a plugin cache or an external repo) can still be read and
/// written through. Traversal via `..` is rejected.
///
/// Trust model: anyone who can plant a symlink under `~/.claude` already
/// controls the config directory, so following such symlinks is the user's
/// intent — the jail exists to stop *caller-supplied* paths (from the UI) from
/// escaping, which `..` rejection + root-prefix on the pre-symlink path covers.
pub fn resolve(input: &str) -> Result<PathBuf, String> {
    let root = root()?;

    let raw = {
        let p = Path::new(input);
        if p.is_absolute() { p.to_path_buf() } else { root.join(p) }
    };

    // Reject `..` components — the only lexical way to escape root.
    if raw.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err("path traversal rejected".into());
    }

    // The lexical path must sit under root (guaranteed for relative inputs;
    // enforced for absolute ones). Symlinks are intentionally not followed.
    if raw.starts_with(&root) {
        Ok(raw)
    } else {
        Err("path escapes ~/.claude".into())
    }
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
