//! Rotating backups. Before every write we copy the current file to
//! `~/.claude/backups/<relpath>.<seq>.bak`, keeping the newest N and deleting
//! older ones. Backups live under `backups/` (not beside the source) so the
//! `.md` scanners never see them and there is no rescan loop.

use crate::jail;
use std::fs;
use std::path::{Path, PathBuf};

const KEEP: usize = 5;

/// Back up `target` (an already-jailed absolute path under root). No-op if the
/// file doesn't exist yet (first save of a new file).
pub fn rotate(target: &Path) -> Result<(), String> {
    if !target.exists() {
        return Ok(());
    }
    let root = jail::root()?.canonicalize().map_err(|e| e.to_string())?;
    let rel = target.strip_prefix(&root).map_err(|_| "target outside root")?;

    let backups_dir = root.join("backups");
    let base = backups_dir.join(rel); // e.g. backups/skills/foo/SKILL.md
    if let Some(parent) = base.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Shift existing .N.bak up by one, dropping anything at or beyond KEEP.
    for n in (0..KEEP).rev() {
        let from = bak_path(&base, n);
        if !from.exists() {
            continue;
        }
        if n + 1 >= KEEP {
            let _ = fs::remove_file(&from);
        } else {
            let _ = fs::rename(&from, bak_path(&base, n + 1));
        }
    }
    fs::copy(target, bak_path(&base, 0)).map_err(|e| e.to_string())?;
    Ok(())
}

fn bak_path(base: &Path, n: usize) -> PathBuf {
    let mut s = base.as_os_str().to_owned();
    s.push(format!(".{n}.bak"));
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::with_claude;

    #[test]
    fn keeps_newest_n_and_drops_older() {
        with_claude(|claude| {
            let f = claude.join("CLAUDE.md");
            let root = claude.canonicalize().unwrap();
            let base = root.join("backups").join("CLAUDE.md");

            // 7 saves; only KEEP backups should survive.
            for i in 0..7 {
                fs::write(&f, format!("v{i}")).unwrap();
                rotate(&f.canonicalize().unwrap()).unwrap();
            }
            let existing: Vec<_> = (0..10).filter(|n| bak_path(&base, *n).exists()).collect();
            assert_eq!(existing.len(), KEEP);
            // Newest backup (.0.bak) is the content from the last save (v6).
            let newest = fs::read_to_string(bak_path(&base, 0)).unwrap();
            assert_eq!(newest, "v6");
        });
    }

    #[test]
    fn noop_for_missing_file() {
        with_claude(|claude| {
            let f = claude.join("ghost.md");
            assert!(rotate(&f).is_ok());
        });
    }
}
