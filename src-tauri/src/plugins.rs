//! Plugins tab: list installed plugins + marketplaces, toggle enabled state in
//! settings.json, and shell out to the `claude` CLI for install/remove (we do
//! NOT reimplement Claude Code's plugin lifecycle).

use crate::jail;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::process::Command;

#[derive(Serialize)]
pub struct Plugin {
    /// Full id `name@marketplace`.
    pub id: String,
    pub name: String,
    pub marketplace: String,
    pub version: String,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct Marketplace {
    pub name: String,
    pub repo: String,
    pub source: String,
}

#[derive(Serialize)]
pub struct PluginData {
    pub plugins: Vec<Plugin>,
    pub marketplaces: Vec<Marketplace>,
}

fn read_json(rel: &str) -> Value {
    jail::resolve(rel)
        .and_then(|p| fs::read_to_string(&p).map_err(|e| e.to_string()))
        .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        .unwrap_or(Value::Null)
}

#[tauri::command]
pub fn plugins_list() -> Result<PluginData, String> {
    let installed = read_json("plugins/installed_plugins.json");
    let markets = read_json("plugins/known_marketplaces.json");
    let settings = read_json("settings.json");
    let enabled = settings.get("enabledPlugins").cloned().unwrap_or(Value::Null);

    let mut plugins = vec![];
    if let Some(map) = installed.get("plugins").and_then(Value::as_object) {
        for (id, entries) in map {
            let (name, marketplace) = split_id(id);
            let version = entries
                .as_array()
                .and_then(|a| a.first())
                .and_then(|e| e.get("version"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let is_enabled = enabled.get(id).and_then(Value::as_bool).unwrap_or(false);
            plugins.push(Plugin {
                id: id.clone(),
                name,
                marketplace,
                version,
                enabled: is_enabled,
            });
        }
    }
    plugins.sort_by(|a, b| a.id.to_lowercase().cmp(&b.id.to_lowercase()));

    let mut marketplaces = vec![];
    if let Some(map) = markets.as_object() {
        for (name, m) in map {
            let src = m.get("source");
            marketplaces.push(Marketplace {
                name: name.clone(),
                repo: src.and_then(|s| s.get("repo")).and_then(Value::as_str).unwrap_or("").into(),
                source: src.and_then(|s| s.get("source")).and_then(Value::as_str).unwrap_or("").into(),
            });
        }
    }
    marketplaces.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(PluginData { plugins, marketplaces })
}

/// Toggle a plugin's enabled flag in settings.json.enabledPlugins.
#[tauri::command]
pub fn plugin_set_enabled(id: String, enabled: bool) -> Result<(), String> {
    crate::settings::settings_set_nested("enabledPlugins", &id, Value::Bool(enabled))
}

/// Install a plugin via the `claude` CLI. `id` is `name@marketplace`.
#[tauri::command]
pub fn plugin_install(id: String) -> Result<String, String> {
    run_claude(&["plugin", "install", &id])
}

/// Remove a plugin via the `claude` CLI.
#[tauri::command]
pub fn plugin_remove(id: String) -> Result<String, String> {
    run_claude(&["plugin", "uninstall", &id])
}

/// Add a marketplace via the `claude` CLI. `repo` is e.g. `owner/name`.
#[tauri::command]
pub fn marketplace_add(repo: String) -> Result<String, String> {
    run_claude(&["plugin", "marketplace", "add", &repo])
}

fn split_id(id: &str) -> (String, String) {
    match id.split_once('@') {
        Some((n, m)) => (n.to_string(), m.to_string()),
        None => (id.to_string(), String::new()),
    }
}

fn run_claude(args: &[&str]) -> Result<String, String> {
    let out = Command::new("claude")
        .args(args)
        .output()
        .map_err(|e| format!("failed to run claude: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::with_claude;

    #[test]
    fn split_id_parses_name_and_market() {
        assert_eq!(split_id("beads@beads-marketplace"), ("beads".into(), "beads-marketplace".into()));
        assert_eq!(split_id("bare"), ("bare".into(), "".into()));
    }

    #[test]
    fn plugins_list_merges_installed_and_enabled() {
        with_claude(|claude| {
            fs::create_dir_all(claude.join("plugins")).unwrap();
            fs::write(
                claude.join("plugins/installed_plugins.json"),
                r#"{"version":2,"plugins":{"foo@mk":[{"version":"1.2.3"}]}}"#,
            )
            .unwrap();
            fs::write(
                claude.join("plugins/known_marketplaces.json"),
                r#"{"mk":{"source":{"source":"github","repo":"o/r"}}}"#,
            )
            .unwrap();
            fs::write(
                claude.join("settings.json"),
                r#"{"enabledPlugins":{"foo@mk":true}}"#,
            )
            .unwrap();

            let d = plugins_list().unwrap();
            assert_eq!(d.plugins.len(), 1);
            assert_eq!(d.plugins[0].name, "foo");
            assert_eq!(d.plugins[0].marketplace, "mk");
            assert_eq!(d.plugins[0].version, "1.2.3");
            assert!(d.plugins[0].enabled);
            assert_eq!(d.marketplaces[0].repo, "o/r");
        });
    }
}
