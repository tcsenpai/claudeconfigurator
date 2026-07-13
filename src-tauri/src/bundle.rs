//! Export / restore the whole Claude configuration as a portable archive.
//!
//! The bundle captures only the real config (instruction files, skills,
//! commands, agents, settings.json, MCP servers from ~/.claude.json, and a
//! plugins manifest), NOT the multi-GB runtime junk under ~/.claude. Symlinked
//! skills are dereferenced so the archive is self-contained. Secrets are
//! optionally redacted. A standalone restore.sh is embedded.

use crate::jail;
use crate::plugins;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::Serialize;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const BUNDLE_VERSION: u32 = 1;
const REDACTED: &str = "__REDACTED__";

/// Unique-per-call temp dir (pid + monotonic counter) so preview and restore in
/// the same process don't collide on a shared path.
fn unique_temp(prefix: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let id = N.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("{prefix}-{}-{id}", std::process::id()))
}

/// Top-level config dirs/files to include (relative to ~/.claude). Root `.md`
/// files are gathered separately.
const CONFIG_DIRS: [&str; 3] = ["skills", "commands", "agents"];

// ---------------------------------------------------------------------------
// Secret detection
// ---------------------------------------------------------------------------

/// A secret-ish value found during the pre-flight scan.
#[derive(Serialize, Clone)]
pub struct SecretHit {
    /// Human-readable location, e.g. "settings.json: env.OPENAI_API_KEY".
    pub location: String,
}

fn looks_secret(key: &str) -> bool {
    let k = key.to_uppercase();
    k.contains("KEY") || k.contains("TOKEN") || k.contains("SECRET") || k.contains("PASSWORD")
}

/// Scan settings.json + ~/.claude.json mcpServers for likely secrets.
#[tauri::command]
pub fn bundle_scan_secrets() -> Result<Vec<SecretHit>, String> {
    let root = jail::root()?;
    let mut hits = vec![];

    if let Ok(v) = read_json(&root.join("settings.json")) {
        for key in ["apiKeyHelper", "awsCredentialExport"] {
            if v.get(key).is_some() {
                hits.push(SecretHit { location: format!("settings.json: {key}") });
            }
        }
        if let Some(env) = v.get("env").and_then(Value::as_object) {
            for k in env.keys().filter(|k| looks_secret(k)) {
                hits.push(SecretHit { location: format!("settings.json: env.{k}") });
            }
        }
    }
    for (name, cfg) in mcp_servers()? {
        if let Some(env) = cfg.get("env").and_then(Value::as_object) {
            for k in env.keys().filter(|k| looks_secret(k)) {
                hits.push(SecretHit { location: format!("mcp: {name}.env.{k}") });
            }
        }
        if let Some(h) = cfg.get("headers").and_then(Value::as_object) {
            for k in h.keys().filter(|k| looks_secret(k) || k.eq_ignore_ascii_case("authorization")) {
                hits.push(SecretHit { location: format!("mcp: {name}.headers.{k}") });
            }
        }
        // args / url can smuggle credentials (e.g. --api-key sk-…, ?token=…).
        // Flag them so the user knows (these are NOT auto-redacted — the value's
        // location within a string is ambiguous — but the warning is honest).
        if value_has_secretish(cfg.get("url")) {
            hits.push(SecretHit { location: format!("mcp: {name}.url (not auto-redacted)") });
        }
        if let Some(args) = cfg.get("args").and_then(Value::as_array) {
            if args.iter().any(|a| value_has_secretish(Some(a))) {
                hits.push(SecretHit { location: format!("mcp: {name}.args (not auto-redacted)") });
            }
        }
    }
    Ok(hits)
}

/// Heuristic: does this string value look like it carries a credential?
fn value_has_secretish(v: Option<&Value>) -> bool {
    let s = match v.and_then(Value::as_str) {
        Some(s) => s.to_lowercase(),
        None => return false,
    };
    s.contains("token=") || s.contains("key=") || s.contains("apikey")
        || s.contains("secret") || s.contains("sk-") || s.contains("password")
}

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct Manifest {
    version: u32,
    created_at: String,
    redacted: bool,
    redactions: Vec<String>,
    /// Skill name -> original symlink target (for reference only).
    dereferenced_symlinks: Map<String, Value>,
    plugins: Value,
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

/// Export the config bundle to `dest` (a .tar.gz path). `redact` strips secrets.
/// `timestamp` is supplied by the caller (Rust can't read wall-clock here).
#[tauri::command]
pub fn bundle_export(dest: String, redact: bool, timestamp: String) -> Result<(), String> {
    let root = jail::root()?;
    let staging = unique_temp("ccbundle");
    let _ = fs::remove_dir_all(&staging);
    let cfg = staging.join("config");
    fs::create_dir_all(&cfg).map_err(|e| e.to_string())?;

    let mut symlinks = Map::new();

    // Root .md files.
    for entry in read_dir(&root) {
        if entry.is_file() && entry.extension().is_some_and(|e| e == "md") {
            copy_file(&entry, &cfg.join(entry.file_name().unwrap()))?;
        }
    }
    // skills / commands / agents (dereferencing symlinks).
    for dir in CONFIG_DIRS {
        let src = root.join(dir);
        if src.is_dir() {
            copy_tree_deref(&src, &cfg.join(dir), dir, &mut symlinks)?;
        }
    }
    // settings.json (redacted).
    let mut redactions = vec![];
    if let Ok(mut settings) = read_json(&root.join("settings.json")) {
        if redact {
            redact_settings(&mut settings, &mut redactions);
        }
        write_json(&cfg.join("settings.json"), &settings)?;
    }
    // mcp.json from ~/.claude.json (redacted).
    let mut mcp = Map::new();
    for (name, mut c) in mcp_servers()? {
        if redact {
            redact_mcp(&name, &mut c, &mut redactions);
        }
        mcp.insert(name, c);
    }
    write_json(&cfg.join("mcp.json"), &Value::Object(mcp))?;

    // manifest + restore.sh
    let plugins = plugins::plugins_list().map(|p| serde_json::to_value(p).unwrap_or(Value::Null))
        .unwrap_or(Value::Null);
    let manifest = Manifest {
        version: BUNDLE_VERSION,
        created_at: timestamp,
        redacted: redact,
        redactions,
        dereferenced_symlinks: symlinks,
        plugins,
    };
    write_json(&staging.join("manifest.json"), &serde_json::to_value(&manifest).unwrap())?;
    fs::write(staging.join("restore.sh"), RESTORE_SH).map_err(|e| e.to_string())?;

    // tar.gz to a temp file, then rename onto dest — so a mid-write failure
    // can't truncate a pre-existing good archive at the chosen path.
    let dest_path = Path::new(&dest);
    let tmp = dest_path.with_extension("cctmp");
    if let Err(e) = tar_gz(&staging, &tmp) {
        let _ = fs::remove_file(&tmp);
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }
    fs::rename(&tmp, dest_path).map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(&staging);
    Ok(())
}

fn redact_settings(settings: &mut Value, out: &mut Vec<String>) {
    if let Some(obj) = settings.as_object_mut() {
        for key in ["apiKeyHelper", "awsCredentialExport"] {
            if obj.contains_key(key) {
                obj.insert(key.into(), Value::String(REDACTED.into()));
                out.push(format!("settings.json:{key}"));
            }
        }
        if let Some(env) = obj.get_mut("env").and_then(Value::as_object_mut) {
            for (k, v) in env.iter_mut() {
                if looks_secret(k) {
                    *v = Value::String(REDACTED.into());
                    out.push(format!("settings.json:env.{k}"));
                }
            }
        }
    }
}

fn redact_mcp(name: &str, cfg: &mut Value, out: &mut Vec<String>) {
    if let Some(env) = cfg.get_mut("env").and_then(Value::as_object_mut) {
        for (k, v) in env.iter_mut() {
            if looks_secret(k) {
                *v = Value::String(REDACTED.into());
                out.push(format!("mcp:{name}.env.{k}"));
            }
        }
    }
    if let Some(h) = cfg.get_mut("headers").and_then(Value::as_object_mut) {
        for (k, v) in h.iter_mut() {
            if looks_secret(k) || k.eq_ignore_ascii_case("authorization") {
                *v = Value::String(REDACTED.into());
                out.push(format!("mcp:{name}.headers.{k}"));
            }
        }
    }
    // url / args can carry credentials too. Redact the whole value when it looks
    // secretish, plus the arg that FOLLOWS a secret-named flag (e.g. --api-key X).
    if value_has_secretish(cfg.get("url")) {
        cfg.as_object_mut().unwrap().insert("url".into(), Value::String(REDACTED.into()));
        out.push(format!("mcp:{name}.url"));
    }
    if let Some(args) = cfg.get_mut("args").and_then(Value::as_array_mut) {
        let mut redact_next = false;
        let mut changed = false;
        for a in args.iter_mut() {
            let s = a.as_str().unwrap_or("");
            let this_secret = value_has_secretish(Some(a));
            // Only a secret-named FLAG (starts with '-') marks the next arg as
            // its value; a plain positional containing "key" does not.
            let flag_secret = s.starts_with('-') && looks_secret(s);
            if this_secret || redact_next {
                *a = Value::String(REDACTED.into());
                changed = true;
            }
            redact_next = flag_secret;
        }
        if changed {
            out.push(format!("mcp:{name}.args"));
        }
    }
}

// ---------------------------------------------------------------------------
// Restore
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct RestorePreview {
    pub version: u32,
    pub created_at: String,
    pub redacted: bool,
    pub redactions: Vec<String>,
    /// Files that already exist locally and differ from the archive.
    pub conflicts: Vec<String>,
    /// Files present in the archive but not locally.
    pub new_files: Vec<String>,
    pub plugin_install_cmds: Vec<String>,
}

/// Inspect an archive without applying it: returns manifest info + a diff of
/// config/ files vs the current ~/.claude.
#[tauri::command]
pub fn bundle_preview(archive: String) -> Result<RestorePreview, String> {
    let dir = extract_to_temp(Path::new(&archive))?;
    let manifest = read_json(&dir.join("manifest.json"))?;
    let version = manifest.get("version").and_then(Value::as_u64).unwrap_or(0) as u32;
    if version > BUNDLE_VERSION {
        return Err(format!("archive version {version} is newer than supported {BUNDLE_VERSION}"));
    }
    let root = jail::root()?;
    let cfg = dir.join("config");
    let (mut conflicts, mut new_files) = (vec![], vec![]);
    for rel in list_config_files(&cfg)? {
        let local = root.join(&rel);
        let bundled = cfg.join(&rel);
        if !local.exists() {
            new_files.push(rel);
        } else if fs::read(&local).ok() != fs::read(&bundled).ok() {
            conflicts.push(rel);
        }
    }
    conflicts.sort();
    new_files.sort();

    let plugin_install_cmds = plugin_cmds(&manifest);
    let _ = fs::remove_dir_all(&dir);
    Ok(RestorePreview {
        version,
        created_at: manifest.get("created_at").and_then(Value::as_str).unwrap_or("").into(),
        redacted: manifest.get("redacted").and_then(Value::as_bool).unwrap_or(false),
        redactions: str_list(&manifest, "redactions"),
        conflicts,
        new_files,
        plugin_install_cmds,
    })
}

/// Apply an archive. `mode` = "replace" (overlay: write every archive file,
/// overwriting matches) or "merge" (overwrite only the conflicting files named
/// in `replace_files`). NEITHER mode deletes local files absent from the archive
/// — a restore never removes config, only adds/overwrites. Always snapshots the
/// current config (including ~/.claude.json) first. `timestamp` labels it.
#[tauri::command]
pub fn bundle_restore(
    archive: String,
    mode: String,
    replace_files: Vec<String>,
    timestamp: String,
) -> Result<(), String> {
    let dir = extract_to_temp(Path::new(&archive))?;
    let manifest = read_json(&dir.join("manifest.json"))?;
    let version = manifest.get("version").and_then(Value::as_u64).unwrap_or(0) as u32;
    if version > BUNDLE_VERSION {
        return Err(format!("archive version {version} newer than supported {BUNDLE_VERSION}"));
    }
    let root = jail::root()?;
    let cfg = dir.join("config");

    // Snapshot current config first — and FAIL CLOSED if it didn't work, so we
    // never overwrite live config without a verified backup.
    snapshot_config(&root, &timestamp)?;
    verify_snapshot(&root, &timestamp)?;

    // From here on, config may be mutated. If any step fails, tell the user
    // where the pre-restore snapshot is so they can recover.
    let result = apply_restore(&root, &cfg, &mode, replace_files);
    let _ = fs::remove_dir_all(&dir);
    result.map_err(|e| {
        format!("{e} — your previous config is backed up at ~/.claude/backups/config-{timestamp}")
    })
}

/// The mutating half of restore, run after a verified snapshot exists.
fn apply_restore(
    root: &Path,
    cfg: &Path,
    mode: &str,
    replace_files: Vec<String>,
) -> Result<(), String> {
    // Patch ~/.claude.json FIRST: it is the only whole-file/atomic op and the
    // most likely to reject (malformed file). Doing it before touching config
    // files means a failure here aborts with the fewest files changed. Redacted
    // placeholders are merged, preserving the user's live secret values.
    let replace: std::collections::HashSet<String> = replace_files.into_iter().collect();
    let merge = mode == "merge";

    let mcp = read_json(&cfg.join("mcp.json")).unwrap_or(Value::Object(Map::new()));
    if let Some(map) = mcp.as_object() {
        // In merge mode, only ADD servers absent locally — never overwrite an
        // existing same-named server the user didn't opt into. Replace mode
        // overwrites. Neither mode removes local servers.
        patch_claude_json_mcp(map, merge)?;
    }

    for rel in list_config_files(cfg)? {
        if rel == "mcp.json" {
            continue; // handled above
        }
        let local = root.join(&rel);
        let bundled = cfg.join(&rel);
        let exists = local.exists();
        let differs = exists && fs::read(&local).ok() != fs::read(&bundled).ok();
        // In merge mode, only overwrite a differing file if the user chose it.
        if merge && differs && !replace.contains(&rel) {
            continue;
        }
        if let Some(parent) = local.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        // settings.json may contain __REDACTED__ placeholders from a redacted
        // export; merge those away so restoring never blanks live secrets.
        if rel == "settings.json" {
            restore_settings_merged(&bundled, &local)?;
        } else {
            copy_file(&bundled, &local)?;
        }
    }
    Ok(())
}

/// Write bundled settings.json to `local`, but for any top-level or `env` value
/// equal to `__REDACTED__`, keep the existing local value instead of the
/// placeholder. Prevents a redacted bundle from wiping live secrets.
fn restore_settings_merged(bundled: &Path, local: &Path) -> Result<(), String> {
    let mut incoming = read_json(bundled)?;
    let existing = read_json(local).unwrap_or(Value::Object(Map::new()));
    unredact(&mut incoming, &existing);
    write_json(local, &incoming)
}

/// Resolve REDACTED placeholders: fill each from `fallback` (matched by object
/// key / array index) where a live value exists, then drop any placeholder that
/// had no fallback. A bogus literal "__REDACTED__" never lands in live config —
/// on a fresh-machine restore the key is simply absent (unset).
fn unredact(target: &mut Value, fallback: &Value) {
    fill_from_fallback(target, fallback);
    strip_redacted(target);
}

/// Replace REDACTED strings with the fallback's value at the same location.
/// Leaves a REDACTED literal in place only where the fallback has nothing.
fn fill_from_fallback(target: &mut Value, fallback: &Value) {
    match target {
        Value::String(s) if s == REDACTED => {
            if !fallback.is_null() {
                *target = fallback.clone();
            }
        }
        Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                fill_from_fallback(v, fallback.get(k).unwrap_or(&Value::Null));
            }
        }
        Value::Array(arr) => {
            let fb = fallback.as_array();
            for (i, v) in arr.iter_mut().enumerate() {
                fill_from_fallback(v, fb.and_then(|a| a.get(i)).unwrap_or(&Value::Null));
            }
        }
        _ => {}
    }
}

/// Remove any remaining REDACTED placeholder: drop the object key, prune the
/// array element. (Top-level scalar placeholders are left to the caller, which
/// never passes a bare scalar.)
fn strip_redacted(target: &mut Value) {
    match target {
        Value::Object(map) => {
            map.retain(|_, v| v.as_str() != Some(REDACTED));
            for v in map.values_mut() {
                strip_redacted(v);
            }
        }
        Value::Array(arr) => {
            arr.retain(|v| v.as_str() != Some(REDACTED));
            for v in arr.iter_mut() {
                strip_redacted(v);
            }
        }
        _ => {}
    }
}

/// Confirm the pre-restore snapshot actually captured the files that exist, so
/// a silently-failed backup can't precede a destructive restore.
fn verify_snapshot(root: &Path, ts: &str) -> Result<(), String> {
    let dest = root.join("backups").join(format!("config-{ts}"));
    if !dest.is_dir() {
        return Err("snapshot directory was not created; aborting restore".into());
    }
    // Every config file/dir that exists on disk must exist in the snapshot.
    for name in ["CLAUDE.md", "settings.json"] {
        let src = root.join(name);
        if src.is_file() && !dest.join(name).is_file() {
            return Err(format!("snapshot missing {name}; aborting restore"));
        }
    }
    for dir in CONFIG_DIRS {
        if root.join(dir).is_dir() && !dest.join(dir).is_dir() {
            return Err(format!("snapshot missing {dir}/; aborting restore"));
        }
    }
    if let Ok(cj) = claude_json_path() {
        if cj.exists() && !dest.join("claude.json.bak").is_file() {
            return Err("snapshot missing ~/.claude.json backup; aborting restore".into());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers: filesystem
// ---------------------------------------------------------------------------

fn read_dir(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect()
}

fn copy_file(src: &Path, dst: &Path) -> Result<(), String> {
    if let Some(p) = dst.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    fs::copy(src, dst).map(|_| ()).map_err(|e| format!("copy {}: {e}", src.display()))
}

/// Recursively copy `src` -> `dst`, following symlinks (so a symlinked skill's
/// real content lands in the archive). Top-level symlinks are recorded.
fn copy_tree_deref(
    src: &Path,
    dst: &Path,
    group: &str,
    symlinks: &mut Map<String, Value>,
) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let from = entry.path();
        let to = dst.join(&name);
        // Record top-level symlinks (e.g. skills/<name> -> external).
        if from.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
            if let Ok(target) = fs::read_link(&from) {
                symlinks.insert(
                    format!("{group}/{}", name.to_string_lossy()),
                    Value::String(target.to_string_lossy().into_owned()),
                );
            }
        }
        // metadata() follows symlinks -> deref. A dangling symlink errors here;
        // skip it rather than aborting the whole export.
        let meta = match fs::metadata(&from) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            copy_tree_deref(&from, &to, group, symlinks)?;
        } else {
            copy_file(&from, &to)?;
        }
    }
    Ok(())
}

/// List every file under `config/` as a path relative to config/.
fn list_config_files(cfg: &Path) -> Result<Vec<String>, String> {
    let mut out = vec![];
    walk(cfg, cfg, &mut out)?;
    Ok(out)
}
fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let p = entry.map_err(|e| e.to_string())?.path();
        // Never follow symlinks in an extracted archive: only process real
        // files/dirs that were genuinely unpacked (defense in depth vs a crafted
        // archive whose entries link out to arbitrary content).
        let ft = match fs::symlink_metadata(&p) {
            Ok(m) => m.file_type(),
            Err(_) => continue,
        };
        if ft.is_symlink() {
            continue;
        }
        if ft.is_dir() {
            walk(base, &p, out)?;
        } else {
            out.push(p.strip_prefix(base).unwrap().to_string_lossy().into_owned());
        }
    }
    Ok(())
}

/// Copy the current config surface into ~/.claude/backups/config-<ts>/ so a
/// restore is undoable. Includes ~/.claude.json (whose mcpServers the restore
/// overwrites) so the snapshot fully covers everything a restore can change.
fn snapshot_config(root: &Path, ts: &str) -> Result<(), String> {
    let dest = root.join("backups").join(format!("config-{ts}"));
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    for entry in read_dir(root) {
        let name = entry.file_name().unwrap().to_string_lossy().into_owned();
        let is_md = entry.is_file() && entry.extension().is_some_and(|e| e == "md");
        let is_cfg_dir = entry.is_dir() && CONFIG_DIRS.contains(&name.as_str());
        let is_settings = name == "settings.json";
        // Propagate copy errors: a snapshot that silently half-fails must abort
        // the restore, not let it proceed over an incomplete backup.
        if is_md || is_settings {
            copy_file(&entry, &dest.join(&name))?;
        } else if is_cfg_dir {
            let mut ignore = Map::new();
            copy_tree_deref(&entry, &dest.join(&name), &name, &mut ignore)?;
        }
    }
    // ~/.claude.json lives outside root; back it up too since restore patches it.
    if let Ok(cj) = claude_json_path() {
        if cj.exists() {
            copy_file(&cj, &dest.join("claude.json.bak"))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers: json + ~/.claude.json
// ---------------------------------------------------------------------------

fn read_json(path: &Path) -> Result<Value, String> {
    let s = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|e| format!("{}: {e}", path.display()))
}
fn write_json(path: &Path, v: &Value) -> Result<(), String> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let s = serde_json::to_string_pretty(v).map_err(|e| e.to_string())? + "\n";
    fs::write(path, s).map_err(|e| e.to_string())
}

fn claude_json_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME not set")?;
    Ok(PathBuf::from(home).join(".claude.json"))
}

/// Read the mcpServers map from ~/.claude.json as (name, config) pairs.
fn mcp_servers() -> Result<Vec<(String, Value)>, String> {
    let path = claude_json_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let doc = read_json(&path)?;
    Ok(doc
        .get("mcpServers")
        .and_then(Value::as_object)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default())
}

/// Merge the bundle's mcpServers into ~/.claude.json (preserve every other key).
/// Starts from the existing servers so local-only servers are never removed.
/// REDACTED placeholders are merged against the existing same-named server so a
/// redacted bundle never blanks live secrets. When `merge_only`, an existing
/// same-named server is left untouched (only genuinely new servers are added).
fn patch_claude_json_mcp(servers: &Map<String, Value>, merge_only: bool) -> Result<(), String> {
    let path = claude_json_path()?;
    let mut doc = if path.exists() { read_json(&path)? } else { Value::Object(Map::new()) };
    let obj = doc.as_object_mut().ok_or("~/.claude.json is not an object")?;
    let existing = obj.get("mcpServers").cloned().unwrap_or(Value::Null);
    let mut merged = existing.as_object().cloned().unwrap_or_default();
    for (name, cfg) in servers {
        // Merge mode: don't overwrite a server the user already has.
        if merge_only && merged.contains_key(name) {
            continue;
        }
        let mut c = cfg.clone();
        unredact(&mut c, existing.get(name).unwrap_or(&Value::Null));
        // Skip a server left with no usable endpoint after redaction stripping
        // (e.g. a url-only server whose url was redacted with no fallback) —
        // don't write a launch-less server Claude can't use.
        let usable = c.get("command").is_some() || c.get("url").is_some();
        if usable {
            merged.insert(name.clone(), c);
        }
    }
    obj.insert("mcpServers".into(), Value::Object(merged));
    let out = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())? + "\n";
    let tmp = path.with_extension("cctmp");
    fs::write(&tmp, out).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

fn str_list(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

/// Build `claude plugin install` / `marketplace add` commands from the manifest.
fn plugin_cmds(manifest: &Value) -> Vec<String> {
    let mut cmds = vec![];
    let plugins = manifest.get("plugins");
    if let Some(markets) = plugins.and_then(|p| p.get("marketplaces")).and_then(Value::as_array) {
        for m in markets {
            if let Some(repo) = m.get("repo").and_then(Value::as_str).filter(|r| !r.is_empty()) {
                cmds.push(format!("claude plugin marketplace add {repo}"));
            }
        }
    }
    if let Some(list) = plugins.and_then(|p| p.get("plugins")).and_then(Value::as_array) {
        for p in list {
            let id = p.get("id").and_then(Value::as_str).unwrap_or("");
            let enabled = p.get("enabled").and_then(Value::as_bool).unwrap_or(false);
            if !id.is_empty() && enabled {
                cmds.push(format!("claude plugin install {id}"));
            }
        }
    }
    cmds
}

// ---------------------------------------------------------------------------
// Helpers: tar.gz
// ---------------------------------------------------------------------------

fn tar_gz(src_dir: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::create(dest).map_err(|e| e.to_string())?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all(".", src_dir).map_err(|e| e.to_string())?;
    tar.into_inner().map_err(|e| e.to_string())?.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn extract_to_temp(archive: &Path) -> Result<PathBuf, String> {
    let out = unique_temp("ccrestore");
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let file = fs::File::open(archive).map_err(|e| e.to_string())?;
    let mut ar = tar::Archive::new(GzDecoder::new(file));
    ar.unpack(&out).map_err(|e| e.to_string())?;
    Ok(out)
}

// A minimal, self-contained restore for when the app isn't available.
const RESTORE_SH: &str = r#"#!/usr/bin/env bash
# Standalone restore for a ClaudeConfigurator config bundle.
# Run from the extracted bundle dir (where this script + config/ live).
# Route: replace (overwrite) with a timestamped backup of the current config.
set -euo pipefail
cd "$(dirname "$0")"
DEST="$HOME/.claude"
TS="$(date +%Y%m%d-%H%M%S)"
BK="$DEST/backups/config-$TS"
echo "==> backing up current config to $BK"
mkdir -p "$BK"
for p in CLAUDE.md skills commands agents settings.json; do
  [ -e "$DEST/$p" ] && cp -R "$DEST/$p" "$BK/" || true
done
echo "==> restoring config into $DEST"
mkdir -p "$DEST"
# copy root .md + dirs + settings.json
cp -R config/. "$DEST/" 2>/dev/null || true
rm -f "$DEST/mcp.json"  # merged into ~/.claude.json below, not a real config file
# patch mcpServers into ~/.claude.json (requires jq)
if [ -f config/mcp.json ] && command -v jq >/dev/null; then
  CJ="$HOME/.claude.json"
  [ -f "$CJ" ] || echo '{}' > "$CJ"
  cp "$CJ" "$BK/claude.json.bak"
  jq --slurpfile mcp config/mcp.json '.mcpServers = $mcp[0]' "$CJ" > "$CJ.tmp" && mv "$CJ.tmp" "$CJ"
  echo "==> patched mcpServers in ~/.claude.json"
fi
echo
echo "==> plugins: run these to reinstall (see manifest.json)"
jq -r '.plugins.marketplaces[]? | "claude plugin marketplace add " + .repo' manifest.json 2>/dev/null || true
jq -r '.plugins.plugins[]? | select(.enabled) | "claude plugin install " + .id' manifest.json 2>/dev/null || true
echo "==> done. Redacted secrets (if any) must be re-entered; see manifest.json."
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::with_claude;

    fn seed(claude: &Path) {
        fs::write(claude.join("CLAUDE.md"), "root instructions\n").unwrap();
        fs::create_dir_all(claude.join("skills/foo")).unwrap();
        fs::write(claude.join("skills/foo/SKILL.md"), "---\nname: foo\n---\n").unwrap();
        fs::create_dir_all(claude.join("commands")).unwrap();
        fs::write(claude.join("commands/c.md"), "cmd\n").unwrap();
        fs::write(
            claude.join("settings.json"),
            r#"{"includeCoAuthoredBy":true,"apiKeyHelper":"secret-cmd","env":{"OPENAI_API_KEY":"sk-xxx","FOO":"bar"}}"#,
        )
        .unwrap();
        // ~/.claude.json with mcpServers + another key.
        let home = std::env::var_os("HOME").unwrap();
        fs::write(
            PathBuf::from(home).join(".claude.json"),
            r#"{"otherKey":1,"mcpServers":{"ctx":{"command":"npx","env":{"API_TOKEN":"t"}}}}"#,
        )
        .unwrap();
    }

    #[test]
    fn export_then_restore_round_trips() {
        with_claude(|claude| {
            seed(claude);
            let dest = claude.parent().unwrap().join("bundle.tar.gz");
            bundle_export(dest.to_string_lossy().into(), false, "20260713-000000".into()).unwrap();
            assert!(dest.is_file());

            // Wipe a config file, then restore (replace).
            fs::remove_file(claude.join("CLAUDE.md")).unwrap();
            bundle_restore(dest.to_string_lossy().into(), "replace".into(), vec![], "20260713-000001".into())
                .unwrap();
            assert_eq!(fs::read_to_string(claude.join("CLAUDE.md")).unwrap(), "root instructions\n");
            // mcpServers survived + other key preserved.
            let home = std::env::var_os("HOME").unwrap();
            let cj = read_json(&PathBuf::from(home).join(".claude.json")).unwrap();
            assert_eq!(cj.get("otherKey").unwrap(), &Value::from(1));
            assert!(cj["mcpServers"].get("ctx").is_some());
        });
    }

    #[test]
    fn redaction_replaces_and_records() {
        with_claude(|claude| {
            seed(claude);
            let dest = claude.parent().unwrap().join("b.tar.gz");
            bundle_export(dest.to_string_lossy().into(), true, "ts".into()).unwrap();
            let ex = extract_to_temp(&dest).unwrap();
            let settings = read_json(&ex.join("config/settings.json")).unwrap();
            assert_eq!(settings["apiKeyHelper"], Value::String(REDACTED.into()));
            assert_eq!(settings["env"]["OPENAI_API_KEY"], Value::String(REDACTED.into()));
            assert_eq!(settings["env"]["FOO"], Value::String("bar".into())); // non-secret kept
            let manifest = read_json(&ex.join("manifest.json")).unwrap();
            assert!(manifest["redacted"].as_bool().unwrap());
            assert!(!manifest["redactions"].as_array().unwrap().is_empty());
        });
    }

    #[test]
    fn symlinked_skill_is_dereferenced() {
        with_claude(|claude| {
            seed(claude);
            // External skill target outside ~/.claude, symlinked in.
            let ext = claude.parent().unwrap().join("ext-skill");
            fs::create_dir_all(&ext).unwrap();
            fs::write(ext.join("SKILL.md"), "external\n").unwrap();
            std::os::unix::fs::symlink(&ext, claude.join("skills/linked")).unwrap();

            let dest = claude.parent().unwrap().join("b2.tar.gz");
            bundle_export(dest.to_string_lossy().into(), false, "ts".into()).unwrap();
            let ex = extract_to_temp(&dest).unwrap();
            // Real content copied in (not a dangling link).
            assert_eq!(
                fs::read_to_string(ex.join("config/skills/linked/SKILL.md")).unwrap(),
                "external\n"
            );
            let manifest = read_json(&ex.join("manifest.json")).unwrap();
            assert!(manifest["dereferenced_symlinks"].get("skills/linked").is_some());
        });
    }

    #[test]
    fn preview_reports_conflicts_and_new() {
        with_claude(|claude| {
            seed(claude);
            let dest = claude.parent().unwrap().join("b3.tar.gz");
            bundle_export(dest.to_string_lossy().into(), false, "ts".into()).unwrap();
            // Change a file (conflict) + remove another (new on restore).
            fs::write(claude.join("CLAUDE.md"), "changed\n").unwrap();
            fs::remove_file(claude.join("commands/c.md")).unwrap();
            let pv = bundle_preview(dest.to_string_lossy().into()).unwrap();
            assert!(pv.conflicts.contains(&"CLAUDE.md".to_string()));
            assert!(pv.new_files.contains(&"commands/c.md".to_string()));
        });
    }

    #[test]
    fn restore_snapshots_claude_json() {
        with_claude(|claude| {
            seed(claude);
            let dest = claude.parent().unwrap().join("b4.tar.gz");
            bundle_export(dest.to_string_lossy().into(), false, "ts".into()).unwrap();
            bundle_restore(dest.to_string_lossy().into(), "replace".into(), vec![], "20260713-1".into())
                .unwrap();
            // ~/.claude.json backed up before the mcpServers patch.
            let snap = claude.join("backups/config-20260713-1/claude.json.bak");
            assert!(snap.exists(), "~/.claude.json must be snapshotted before restore");
            let backed = read_json(&snap).unwrap();
            assert_eq!(backed.get("otherKey").unwrap(), &Value::from(1));
        });
    }

    #[test]
    fn dangling_symlink_does_not_abort_export() {
        with_claude(|claude| {
            seed(claude);
            // A skill symlink whose target does not exist.
            let missing = claude.parent().unwrap().join("gone");
            std::os::unix::fs::symlink(&missing, claude.join("skills/broken")).unwrap();
            let dest = claude.parent().unwrap().join("b5.tar.gz");
            // Must still succeed, skipping the broken link.
            bundle_export(dest.to_string_lossy().into(), false, "ts".into()).unwrap();
            assert!(dest.is_file());
            let ex = extract_to_temp(&dest).unwrap();
            assert!(ex.join("config/skills/foo/SKILL.md").is_file());
        });
    }

    #[test]
    fn redact_strips_mcp_args_and_url() {
        with_claude(|claude| {
            seed(claude);
            let home = std::env::var_os("HOME").unwrap();
            fs::write(
                PathBuf::from(&home).join(".claude.json"),
                r#"{"mcpServers":{"s":{"command":"x","args":["--api-key","sk-abc","--flag"],"url":"https://h?token=z"}}}"#,
            )
            .unwrap();
            let dest = claude.parent().unwrap().join("r.tar.gz");
            bundle_export(dest.to_string_lossy().into(), true, "ts".into()).unwrap();
            let ex = extract_to_temp(&dest).unwrap();
            let mcp = read_json(&ex.join("config/mcp.json")).unwrap();
            assert_eq!(mcp["s"]["url"], Value::String(REDACTED.into()));
            // value following the --api-key flag is redacted; --flag preserved.
            let args = mcp["s"]["args"].as_array().unwrap();
            assert_eq!(args[1], Value::String(REDACTED.into()));
            assert_eq!(args[2], Value::String("--flag".into()));
        });
    }

    #[test]
    fn restoring_redacted_bundle_keeps_live_secrets() {
        with_claude(|claude| {
            seed(claude); // settings has real apiKeyHelper + OPENAI_API_KEY
            let dest = claude.parent().unwrap().join("red.tar.gz");
            bundle_export(dest.to_string_lossy().into(), true, "ts".into()).unwrap();
            // Restore the redacted bundle over the live (real-secret) config.
            bundle_restore(dest.to_string_lossy().into(), "replace".into(), vec![], "20260713-2".into())
                .unwrap();
            let settings = read_json(&claude.join("settings.json")).unwrap();
            // Live secret preserved, NOT stomped with __REDACTED__.
            assert_eq!(settings["apiKeyHelper"], Value::String("secret-cmd".into()));
            assert_eq!(settings["env"]["OPENAI_API_KEY"], Value::String("sk-xxx".into()));
            assert_eq!(settings["includeCoAuthoredBy"], Value::Bool(true));
        });
    }

    #[test]
    fn restore_keeps_local_mcp_servers_absent_from_bundle() {
        with_claude(|claude| {
            seed(claude); // bundle will contain server "ctx"
            let dest = claude.parent().unwrap().join("mcpkeep.tar.gz");
            bundle_export(dest.to_string_lossy().into(), false, "ts".into()).unwrap();
            // Add a NEW local server not in the bundle.
            let home = std::env::var_os("HOME").unwrap();
            fs::write(
                PathBuf::from(&home).join(".claude.json"),
                r#"{"otherKey":1,"mcpServers":{"ctx":{"command":"npx"},"local_only":{"command":"y"}}}"#,
            )
            .unwrap();
            bundle_restore(dest.to_string_lossy().into(), "replace".into(), vec![], "20260713-4".into())
                .unwrap();
            let cj = read_json(&PathBuf::from(&home).join(".claude.json")).unwrap();
            // Bundle server present AND local-only server preserved.
            assert!(cj["mcpServers"].get("ctx").is_some());
            assert!(cj["mcpServers"].get("local_only").is_some(), "local server must survive");
            assert_eq!(cj.get("otherKey").unwrap(), &Value::from(1));
        });
    }

    #[test]
    fn merge_mode_does_not_overwrite_existing_mcp_server() {
        with_claude(|claude| {
            seed(claude); // bundle contains server "ctx" = {command:"npx",...}
            let dest = claude.parent().unwrap().join("mergemcp.tar.gz");
            bundle_export(dest.to_string_lossy().into(), false, "ts".into()).unwrap();
            // User's live "ctx" now differs from the bundle's.
            let home = std::env::var_os("HOME").unwrap();
            fs::write(
                PathBuf::from(&home).join(".claude.json"),
                r#"{"mcpServers":{"ctx":{"command":"MINE"}}}"#,
            )
            .unwrap();
            bundle_restore(dest.to_string_lossy().into(), "merge".into(), vec![], "20260713-5".into())
                .unwrap();
            let cj = read_json(&PathBuf::from(&home).join(".claude.json")).unwrap();
            // Merge mode left the existing ctx untouched.
            assert_eq!(cj["mcpServers"]["ctx"]["command"], Value::String("MINE".into()));
        });
    }

    #[test]
    fn fresh_restore_drops_placeholders_not_literal() {
        with_claude(|claude| {
            seed(claude);
            let dest = claude.parent().unwrap().join("fresh.tar.gz");
            bundle_export(dest.to_string_lossy().into(), true, "ts".into()).unwrap();
            // Simulate a fresh machine: remove local settings + claude.json.
            fs::remove_file(claude.join("settings.json")).unwrap();
            let home = std::env::var_os("HOME").unwrap();
            fs::write(PathBuf::from(&home).join(".claude.json"), "{}").unwrap();

            bundle_restore(dest.to_string_lossy().into(), "replace".into(), vec![], "20260713-3".into())
                .unwrap();

            let settings = read_json(&claude.join("settings.json")).unwrap();
            // Non-secret restored; redacted keys ABSENT (not literal __REDACTED__).
            assert_eq!(settings["includeCoAuthoredBy"], Value::Bool(true));
            assert!(settings.get("apiKeyHelper").is_none(), "placeholder must be dropped");
            assert!(settings["env"].get("OPENAI_API_KEY").is_none(), "placeholder dropped");
            assert_eq!(settings["env"]["FOO"], Value::String("bar".into()));
            // No literal __REDACTED__ anywhere.
            assert!(!serde_json::to_string(&settings).unwrap().contains(REDACTED));
        });
    }

    #[test]
    fn scan_flags_secretish_args_and_url() {
        with_claude(|claude| {
            seed(claude);
            let home = std::env::var_os("HOME").unwrap();
            fs::write(
                PathBuf::from(home).join(".claude.json"),
                r#"{"mcpServers":{"s":{"command":"x","args":["--api-key","sk-abc"],"url":"https://h?token=z"}}}"#,
            )
            .unwrap();
            let hits = bundle_scan_secrets().unwrap();
            assert!(hits.iter().any(|h| h.location.contains("args")));
            assert!(hits.iter().any(|h| h.location.contains("url")));
        });
    }
}
