//! MCP server configuration. Claude Code stores global MCP servers in
//! ~/.claude.json under the top-level `mcpServers` key — a big file holding all
//! of Claude Code's state, so every write is SURGICAL: read the whole file,
//! replace only the MCP-related keys, re-serialize preserving order, back up
//! the whole file first. Disabling a server moves it to `_disabledMcpServers`
//! (a stash this app owns) so the config is not lost.

use serde::Serialize;
use serde_json::{Map, Value};
use std::fs;
use std::path::PathBuf;

const ACTIVE: &str = "mcpServers";
const DISABLED: &str = "_disabledMcpServers";

fn claude_json() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME not set")?;
    Ok(PathBuf::from(home).join(".claude.json"))
}

fn load() -> Result<Value, String> {
    let path = claude_json()?;
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| format!("~/.claude.json parse error: {e}"))
}

#[derive(Serialize)]
pub struct McpServer {
    pub name: String,
    pub enabled: bool,
    /// The raw server config object (command/args/env or type/url/headers).
    pub config: Value,
}

/// List active + disabled MCP servers.
#[tauri::command]
pub fn mcp_list() -> Result<Vec<McpServer>, String> {
    let doc = load()?;
    let mut out = vec![];
    if let Some(m) = doc.get(ACTIVE).and_then(Value::as_object) {
        for (name, cfg) in m {
            out.push(McpServer { name: name.clone(), enabled: true, config: cfg.clone() });
        }
    }
    if let Some(m) = doc.get(DISABLED).and_then(Value::as_object) {
        for (name, cfg) in m {
            out.push(McpServer { name: name.clone(), enabled: false, config: cfg.clone() });
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

/// Add or update a server (in the active map). `config` is the raw server
/// object. Errors on invalid name.
#[tauri::command]
pub fn mcp_upsert(name: String, config: Value, enabled: bool) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("server name is empty".into());
    }
    if !config.is_object() {
        return Err("server config must be an object".into());
    }
    let mut doc = load()?;
    // Remove from both maps first so an edit that flips enabled doesn't dup.
    remove_from(&mut doc, ACTIVE, &name);
    remove_from(&mut doc, DISABLED, &name);
    let target = if enabled { ACTIVE } else { DISABLED };
    map_mut(&mut doc, target).insert(name, config);
    persist(&doc)
}

/// Remove a server entirely (from whichever map holds it).
#[tauri::command]
pub fn mcp_remove(name: String) -> Result<(), String> {
    let mut doc = load()?;
    let a = remove_from(&mut doc, ACTIVE, &name);
    let d = remove_from(&mut doc, DISABLED, &name);
    if !a && !d {
        return Err(format!("{name} not found"));
    }
    persist(&doc)
}

/// Enable/disable: move a server between the active and disabled maps.
#[tauri::command]
pub fn mcp_set_enabled(name: String, enabled: bool) -> Result<(), String> {
    let mut doc = load()?;
    let (from, to) = if enabled { (DISABLED, ACTIVE) } else { (ACTIVE, DISABLED) };
    let cfg = take_from(&mut doc, from, &name)
        .or_else(|| take_from(&mut doc, to, &name)) // already in target: no-op move
        .ok_or_else(|| format!("{name} not found"))?;
    map_mut(&mut doc, to).insert(name, cfg);
    persist(&doc)
}

fn map_mut<'a>(doc: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    let obj = doc.as_object_mut().expect("claude.json root is an object");
    obj.entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .expect("mcp map is an object")
}

fn remove_from(doc: &mut Value, key: &str, name: &str) -> bool {
    doc.get_mut(key)
        .and_then(Value::as_object_mut)
        .map(|m| m.remove(name).is_some())
        .unwrap_or(false)
}

fn take_from(doc: &mut Value, key: &str, name: &str) -> Option<Value> {
    doc.get_mut(key).and_then(Value::as_object_mut).and_then(|m| m.remove(name))
}

/// Back up the whole ~/.claude.json into ~/.claude/backups/, then write.
fn persist(doc: &Value) -> Result<(), String> {
    let path = claude_json()?;
    backup_claude_json(&path)?;
    let out = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())? + "\n";
    let tmp = path.with_extension("cctmp");
    fs::write(&tmp, out).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// Rotating backup of ~/.claude.json into ~/.claude/backups/ (keep newest 5).
fn backup_claude_json(src: &std::path::Path) -> Result<(), String> {
    if !src.exists() {
        return Ok(());
    }
    let home = std::env::var_os("HOME").ok_or("HOME not set")?;
    let backups = PathBuf::from(&home).join(".claude").join("backups");
    fs::create_dir_all(&backups).map_err(|e| e.to_string())?;
    let base = backups.join("claude.json");
    const KEEP: usize = 5;
    for n in (0..KEEP).rev() {
        let from = bak(&base, n);
        if !from.exists() {
            continue;
        }
        if n + 1 >= KEEP {
            let _ = fs::remove_file(&from);
        } else {
            let _ = fs::rename(&from, bak(&base, n + 1));
        }
    }
    fs::copy(src, bak(&base, 0)).map_err(|e| e.to_string())?;
    Ok(())
}

fn bak(base: &std::path::Path, n: usize) -> PathBuf {
    let mut s = base.as_os_str().to_owned();
    s.push(format!(".{n}.bak"));
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::with_claude;

    fn write_claude_json(v: Value) {
        let home = std::env::var_os("HOME").unwrap();
        let p = PathBuf::from(home).join(".claude.json");
        fs::write(p, serde_json::to_string(&v).unwrap()).unwrap();
    }

    #[test]
    fn list_active_and_disabled() {
        with_claude(|_| {
            write_claude_json(serde_json::json!({
                "otherKey": 1,
                "mcpServers": {"ctx": {"command": "npx"}},
                "_disabledMcpServers": {"old": {"command": "x"}}
            }));
            let l = mcp_list().unwrap();
            assert_eq!(l.len(), 2);
            let ctx = l.iter().find(|s| s.name == "ctx").unwrap();
            assert!(ctx.enabled);
            assert!(!l.iter().find(|s| s.name == "old").unwrap().enabled);
        });
    }

    #[test]
    fn upsert_preserves_other_keys() {
        with_claude(|_| {
            write_claude_json(serde_json::json!({"otherKey": 42, "mcpServers": {}}));
            mcp_upsert("new".into(), serde_json::json!({"command": "run"}), true).unwrap();
            let doc = load().unwrap();
            assert_eq!(doc.get("otherKey").unwrap(), &Value::from(42));
            assert!(doc["mcpServers"].get("new").is_some());
        });
    }

    #[test]
    fn toggle_moves_between_maps_and_backs_up() {
        with_claude(|claude| {
            write_claude_json(serde_json::json!({"mcpServers": {"s": {"command": "c"}}}));
            mcp_set_enabled("s".into(), false).unwrap();
            let doc = load().unwrap();
            assert!(doc["mcpServers"].get("s").is_none());
            assert!(doc["_disabledMcpServers"].get("s").is_some());
            // whole-file backup exists
            assert!(claude.join("backups/claude.json.0.bak").exists());
        });
    }

    #[test]
    fn remove_deletes_and_errors_when_missing() {
        with_claude(|_| {
            write_claude_json(serde_json::json!({"mcpServers": {"s": {"command": "c"}}}));
            mcp_remove("s".into()).unwrap();
            assert!(load().unwrap()["mcpServers"].get("s").is_none());
            assert!(mcp_remove("ghost".into()).is_err());
        });
    }
}
