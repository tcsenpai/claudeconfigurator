//! Test-only helper: run a closure with `HOME` pointed at a fresh temp
//! `~/.claude`, serialized across threads (env is process-global).

#![cfg(test)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Create an isolated `~/.claude`, set `HOME`, run `f(claude_dir)`, clean up.
/// Serialized so parallel tests don't clobber each other's `HOME`.
pub fn with_claude<T>(f: impl FnOnce(&PathBuf) -> T) -> T {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!("cctest-{}-{}", std::process::id(), id));
    let claude = tmp.join(".claude");
    std::fs::create_dir_all(&claude).unwrap();
    std::env::set_var("HOME", &tmp);
    let r = f(&claude);
    let _ = std::fs::remove_dir_all(&tmp);
    r
}
