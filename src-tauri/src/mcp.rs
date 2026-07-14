//! MCP server configuration. The active scope decides the file: global servers
//! live in ~/.claude.json (a big file holding all of Claude Code's state), a
//! project's live in <proj>/.mcp.json. Either way writes are SURGICAL — read
//! the whole file, replace only the MCP-related keys, re-serialize preserving
//! order, back up the whole file first. Disabling a server moves it to
//! `_disabledMcpServers` (a stash this app owns) so the config is not lost.

use crate::scope;
use serde::Serialize;
use serde_json::{Map, Value};
use std::fs;
use std::path::PathBuf;

const ACTIVE: &str = "mcpServers";
const DISABLED: &str = "_disabledMcpServers";

fn claude_json() -> Result<PathBuf, String> {
    scope::mcp_file()
}

fn load() -> Result<Value, String> {
    let path = claude_json()?;
    let mut doc = if path.exists() {
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).map_err(|e| format!("{} parse error: {e}", path.display()))?
    } else {
        // A project may not have a .mcp.json yet; treat as empty.
        Value::Object(Map::new())
    };
    // In project scope the disabled stash lives in an app-owned sidecar so the
    // committed .mcp.json stays clean (only `mcpServers`). Merge it in for the
    // in-memory model, which uses the DISABLED key uniformly.
    if let Some(sidecar) = disabled_sidecar()? {
        if sidecar.exists() {
            let raw = fs::read_to_string(&sidecar).map_err(|e| e.to_string())?;
            let stash: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
            if stash.is_object() {
                if let Some(obj) = doc.as_object_mut() {
                    obj.insert(DISABLED.into(), stash);
                }
            }
        }
    }
    Ok(doc)
}

/// The disabled-server sidecar path in project scope (None in global, where the
/// DISABLED key lives directly in the app-owned ~/.claude.json).
fn disabled_sidecar() -> Result<Option<PathBuf>, String> {
    use crate::scope;
    let mcp = scope::mcp_file()?;
    // Global mcp file is ~/.claude.json; project is <proj>/.mcp.json.
    if mcp.file_name().is_some_and(|n| n == ".mcp.json") {
        Ok(Some(scope::config_dir()?.join("mcp-disabled.json")))
    } else {
        Ok(None)
    }
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

/// Persist the in-memory model. In project scope the DISABLED stash is split
/// out to a sidecar so the committed .mcp.json holds only real config.
fn persist(doc: &Value) -> Result<(), String> {
    let path = claude_json()?;
    let mut doc = doc.clone();

    if let Some(sidecar) = disabled_sidecar()? {
        // Move DISABLED out of the main doc into the sidecar.
        let stash = doc.as_object_mut().and_then(|o| o.remove(DISABLED));
        let stash = stash.unwrap_or(Value::Object(Map::new()));
        let empty = stash.as_object().map(|m| m.is_empty()).unwrap_or(true);
        if empty {
            let _ = fs::remove_file(&sidecar); // no disabled servers -> no sidecar
        } else {
            let out = serde_json::to_string_pretty(&stash).map_err(|e| e.to_string())? + "\n";
            let tmp = sidecar.with_extension("cctmp");
            fs::write(&tmp, out).map_err(|e| e.to_string())?;
            fs::rename(&tmp, &sidecar).map_err(|e| e.to_string())?;
        }
    }

    backup_claude_json(&path)?;
    let out = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())? + "\n";
    let tmp = path.with_extension("cctmp");
    fs::write(&tmp, out).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// Rotating backup of the active MCP file into the active scope's backups/ dir
/// (keep newest 5). Global -> ~/.claude/backups/claude.json.*.bak ; project ->
/// <proj>/.claude/backups/mcp.json.*.bak.
fn backup_claude_json(src: &std::path::Path) -> Result<(), String> {
    if !src.exists() {
        return Ok(());
    }
    let backups = scope::config_dir()?.join("backups");
    fs::create_dir_all(&backups).map_err(|e| e.to_string())?;
    let label = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or("mcp".into());
    let base = backups.join(label);
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
        let home = crate::scope::home_dir().unwrap();
        let p = home.join(".claude.json");
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
            assert!(claude.join("backups/.claude.json.0.bak").exists());
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

    #[test]
    fn project_disabled_goes_to_sidecar_not_mcp_json() {
        with_claude(|claude| {
            let proj = claude.parent().unwrap().join("mproj");
            fs::create_dir_all(proj.join(".claude")).unwrap();
            fs::write(proj.join(".mcp.json"), r#"{"mcpServers":{"s":{"command":"c"}}}"#).unwrap();
            crate::scope::set_project_for_test(&proj);

            mcp_set_enabled("s".into(), false).unwrap();

            // .mcp.json (committed) must NOT carry the app-private stash key.
            let raw = fs::read_to_string(proj.join(".mcp.json")).unwrap();
            assert!(!raw.contains("_disabledMcpServers"), ".mcp.json must stay clean");
            let doc: Value = serde_json::from_str(&raw).unwrap();
            assert!(doc["mcpServers"].get("s").is_none());
            // Sidecar holds the disabled server; list still shows it as disabled.
            assert!(proj.join(".claude/mcp-disabled.json").exists());
            let l = mcp_list().unwrap();
            let s = l.iter().find(|x| x.name == "s").unwrap();
            assert!(!s.enabled);

            // Re-enable: server returns to .mcp.json; sidecar cleared.
            mcp_set_enabled("s".into(), true).unwrap();
            let raw = fs::read_to_string(proj.join(".mcp.json")).unwrap();
            let doc: Value = serde_json::from_str(&raw).unwrap();
            assert!(doc["mcpServers"].get("s").is_some());
            assert!(!proj.join(".claude/mcp-disabled.json").exists(), "empty sidecar removed");

            crate::scope::set_global_for_test();
        });
    }
}
