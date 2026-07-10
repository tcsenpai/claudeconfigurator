//! ClaudeConfigurator's OWN preferences (not Claude's config). Stored outside
//! ~/.claude so it never appears in the file/config views. Uses the platform
//! app-config dir: ~/Library/Application Support/<id>/ on macOS,
//! $XDG_CONFIG_HOME (or ~/.config)/<id>/ on Linux.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const APP_ID: &str = "sh.discus.claudeconfigurator";
const FILE: &str = "config.json";

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Autosave a dirty document after a period of inactivity.
    pub autosave: bool,
    /// Inactivity delay before autosave fires, in milliseconds.
    pub autosave_delay_ms: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        // Off by default: this app edits the user's live config, so opt-in.
        Self { autosave: false, autosave_delay_ms: 5000 }
    }
}

fn config_dir() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME not set")?;
    let home = PathBuf::from(home);
    let base = if cfg!(target_os = "macos") {
        home.join("Library").join("Application Support")
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
    };
    Ok(base.join(APP_ID))
}

fn config_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join(FILE))
}

/// Load app preferences, falling back to defaults if the file is absent or bad.
#[tauri::command]
pub fn app_config_get() -> Result<AppConfig, String> {
    let path = config_path()?;
    match fs::read_to_string(&path) {
        Ok(s) => Ok(serde_json::from_str(&s).unwrap_or_default()),
        Err(_) => Ok(AppConfig::default()),
    }
}

/// Persist app preferences (creates the config dir if needed).
#[tauri::command]
pub fn app_config_set(config: AppConfig) -> Result<(), String> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(config_path()?, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_off_5s() {
        let c = AppConfig::default();
        assert!(!c.autosave);
        assert_eq!(c.autosave_delay_ms, 5000);
    }

    #[test]
    fn roundtrip_via_serde() {
        let c = AppConfig { autosave: true, autosave_delay_ms: 3000 };
        let s = serde_json::to_string(&c).unwrap();
        let back: AppConfig = serde_json::from_str(&s).unwrap();
        assert!(back.autosave);
        assert_eq!(back.autosave_delay_ms, 3000);
    }

    #[test]
    fn partial_json_uses_defaults() {
        // Missing autosave_delay_ms -> default 5000 (serde(default)).
        let back: AppConfig = serde_json::from_str(r#"{"autosave":true}"#).unwrap();
        assert!(back.autosave);
        assert_eq!(back.autosave_delay_ms, 5000);
    }
}
