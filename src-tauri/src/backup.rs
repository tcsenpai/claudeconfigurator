//! Rotating backups. Before every write we copy the current file to
//! `~/.claude/backups/<relpath>.<seq>.bak`, keeping the newest N and deleting
//! older ones. Backups live under `backups/` (not beside the source) so the
//! `.md` scanners never see them and there is no rescan loop.

use crate::jail;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const KEEP: usize = 5;

#[derive(Serialize)]
pub struct BackupInfo {
    pub index: usize,
    pub size: u64,
    pub modified_ms: u64,
}

pub fn resolve_backup_base(target: &Path) -> Result<PathBuf, String> {
    let root = jail::root()?;
    let backups_dir = root.join("backups");
    let base = match target.strip_prefix(&root) {
        Ok(rel) => backups_dir.join(rel),
        Err(_) => backups_dir
            .join("_root")
            .join(target.file_name().ok_or("invalid target")?),
    };
    Ok(base)
}

/// Back up `target` (an already-jailed absolute path under root). No-op if the
/// file doesn't exist yet (first save of a new file).
pub fn rotate(target: &Path) -> Result<(), String> {
    if !target.exists() {
        return Ok(());
    }
    let base = resolve_backup_base(target)?;
    if let Some(parent) = base.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let target_mtime = fs::metadata(target)
        .and_then(|m| m.modified())
        .unwrap_or_else(|_| SystemTime::now());

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
    let content = fs::read(target).map_err(|e| e.to_string())?;
    let dest = bak_path(&base, 0);
    fs::write(&dest, content).map_err(|e| e.to_string())?;
    if let Ok(f) = fs::OpenOptions::new().write(true).open(&dest) {
        let _ = f.set_modified(target_mtime);
    }
    Ok(())
}

fn bak_path(base: &Path, n: usize) -> PathBuf {
    let mut s = base.as_os_str().to_owned();
    s.push(format!(".{n}.bak"));
    PathBuf::from(s)
}

/// List backups for a given jailed file path.
#[tauri::command]
pub fn backup_list(path: String) -> Result<Vec<BackupInfo>, String> {
    let abs = jail::resolve_any(&path)?;
    let base = resolve_backup_base(&abs)?;
    let mut out = vec![];
    for n in 0..KEEP {
        let p = bak_path(&base, n);
        if p.is_file() {
            let meta = fs::metadata(&p).map_err(|e| e.to_string())?;
            let modified = meta
                .modified()
                .map_err(|e| e.to_string())?
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            out.push(BackupInfo {
                index: n,
                size: meta.len(),
                modified_ms: modified,
            });
        }
    }
    // Sort by modified_ms descending so the newest backup is always first
    out.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms));
    Ok(out)
}

/// Read a backup file's raw content by index.
#[tauri::command]
pub fn backup_read(path: String, index: usize) -> Result<String, String> {
    if index >= KEEP {
        return Err("invalid backup index".into());
    }
    let abs = jail::resolve_any(&path)?;
    let base = resolve_backup_base(&abs)?;
    let p = bak_path(&base, index);
    if !p.is_file() {
        return Err("backup file not found".into());
    }
    fs::read_to_string(&p).map_err(|e| e.to_string())
}

/// Restore a backup file by index, returning its content.
#[tauri::command]
pub fn backup_restore(path: String, index: usize) -> Result<String, String> {
    if index >= KEEP {
        return Err("invalid backup index".into());
    }
    let abs = jail::resolve_any(&path)?;
    let base = resolve_backup_base(&abs)?;
    let p = bak_path(&base, index);
    if !p.is_file() {
        return Err("backup file not found".into());
    }
    rotate(&abs)?;
    let content = fs::read(&p).map_err(|e| e.to_string())?;
    fs::write(&abs, content).map_err(|e| e.to_string())?;
    fs::read_to_string(&abs).map_err(|e| e.to_string())
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::with_claude;

    #[test]
    fn keeps_newest_n_and_drops_older() {
        with_claude(|claude| {
            let f = claude.join("CLAUDE.md");
            // jail::resolve returns non-canonical paths rooted at ~/.claude, so
            // exercise rotate the same way (no canonicalize).
            let base = claude.join("backups").join("CLAUDE.md");

            // 7 saves; only KEEP backups should survive.
            for i in 0..7 {
                fs::write(&f, format!("v{i}")).unwrap();
                rotate(&f).unwrap();
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
