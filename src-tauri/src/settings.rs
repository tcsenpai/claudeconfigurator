//! Structured access to settings.json for the Hooks and Plugins tabs.
//! Read the whole file, edit a top-level key, write back (validated + backed up).

use crate::backup;
use crate::jail;
use serde_json::Value;
use std::fs;

const SETTINGS: &str = "settings.json";

fn load() -> Result<Value, String> {
    let abs = jail::resolve(SETTINGS)?;
    let raw = fs::read_to_string(&abs).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| format!("settings.json parse error: {e}"))
}

/// The JSON Schema for settings.json, vendored at build time from
/// json.schemastore.org. Used to drive typed form widgets + inline help.
const SCHEMA: &str = include_str!("../assets/claude-code-settings.schema.json");

/// Return the bundled settings JSON Schema (parsed).
#[tauri::command]
pub fn settings_schema() -> Result<Value, String> {
    serde_json::from_str(SCHEMA).map_err(|e| format!("bundled schema parse error: {e}"))
}

/// Read a top-level key (returns `null` if absent).
#[tauri::command]
pub fn settings_get(key: String) -> Result<Value, String> {
    Ok(load()?.get(&key).cloned().unwrap_or(Value::Null))
}

/// Set `outer.inner = value`, creating `outer` as an object if needed. Used to
/// toggle one plugin inside `enabledPlugins`. Backs up + validates.
pub fn settings_set_nested(outer: &str, inner: &str, value: Value) -> Result<(), String> {
    let mut doc = load()?;
    let obj = doc.as_object_mut().ok_or("settings.json is not an object")?;
    let nested = obj
        .entry(outer.to_string())
        .or_insert_with(|| Value::Object(Default::default()));
    nested
        .as_object_mut()
        .ok_or_else(|| format!("settings.json `{outer}` is not an object"))?
        .insert(inner.to_string(), value);
    persist(&doc)
}

fn persist(doc: &Value) -> Result<(), String> {
    let out = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())? + "\n";
    let abs = jail::resolve(SETTINGS)?;
    backup::rotate(&abs)?;
    let tmp = abs.with_extension("cctmp");
    fs::write(&tmp, out).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &abs).map_err(|e| e.to_string())
}

/// Replace a top-level key and persist. Preserves key order of existing keys
/// (serde_json Map is insertion-ordered with the `preserve_order` feature; even
/// without it, we only touch the one key). Backs up first.
#[tauri::command]
pub fn settings_set(key: String, value: Value) -> Result<(), String> {
    let mut doc = load()?;
    let obj = doc.as_object_mut().ok_or("settings.json is not an object")?;
    obj.insert(key, value);
    persist(&doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::with_claude;

    #[test]
    fn nested_set_creates_and_preserves_order() {
        with_claude(|claude| {
            fs::write(
                claude.join(SETTINGS),
                r#"{"a":1,"enabledPlugins":{"x@m":true},"z":9}"#,
            )
            .unwrap();
            settings_set_nested("enabledPlugins", "y@m", Value::Bool(false)).unwrap();

            let doc = load().unwrap();
            let ep = doc.get("enabledPlugins").unwrap();
            assert_eq!(ep.get("x@m").unwrap(), &Value::Bool(true));
            assert_eq!(ep.get("y@m").unwrap(), &Value::Bool(false));
            // Top-level order preserved: a, enabledPlugins, z.
            let keys: Vec<_> = doc.as_object().unwrap().keys().cloned().collect();
            assert_eq!(keys, vec!["a", "enabledPlugins", "z"]);
        });
    }

    #[test]
    fn set_backs_up_before_write() {
        with_claude(|claude| {
            fs::write(claude.join(SETTINGS), r#"{"a":1}"#).unwrap();
            settings_set("a".into(), Value::from(2)).unwrap();
            // A backup of the pre-write content must exist.
            let bak = claude.join("backups").join("settings.json.0.bak");
            assert!(bak.exists());
            assert_eq!(fs::read_to_string(bak).unwrap(), r#"{"a":1}"#);
            assert_eq!(load().unwrap().get("a").unwrap(), &Value::from(2));
        });
    }
}
