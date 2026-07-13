# Export / Restore Claude configuration

**Date:** 2026-07-13

Export the whole Claude configuration as a portable, self-contained archive and
restore it on the same or another machine. In-app buttons plus an embedded
standalone `restore.sh`.

## Scope: what is captured

`~/.claude` is ~2.8GB but almost all of it is runtime junk. The bundle captures
only the real config:

- Root instruction files: `CLAUDE.md` and adjacent `*.md`
- `skills/` (symlinked skills dereferenced to real content)
- `commands/`, `agents/`
- `settings.json` (secrets redacted per user choice)
- MCP servers extracted from `~/.claude.json` -> `mcp.json`
- A plugins manifest (name@marketplace + enabled + marketplace repos) so restore
  can re-run `claude plugin install` / `marketplace add` instead of copying the
  511MB plugin cache

Excluded: `plugins/` caches, `projects/`, `file-history/`, `context-mode/`,
`backups/`, `shell-snapshots/`, other runtime state.

## Archive layout (`.tar.gz`)

```
claude-config-<timestamp>/
  manifest.json     version, created_at, redactions[], symlink notes, plugins[]
  config/
    *.md
    skills/  commands/  agents/
    settings.json
    mcp.json
  restore.sh        standalone restore, works without the app
```

## Symlinked skills

9 skills are symlinks pointing outside `~/.claude` (e.g. `~/.agents/skills/...`).
Export dereferences them: the real files are copied in so the restored config is
self-contained anywhere. `manifest.json` records "was a symlink to X".

## Secrets

Pre-flight scan before export:

- `settings.json` keys `apiKeyHelper`, `awsCredentialExport`, and `env` values
  matching `*KEY*|*TOKEN*|*SECRET*`
- `mcpServers[].env` (same pattern) and `headers` with `Authorization`/`token`
- Files `.sofa/credentials.json`, `.credentials*` -> excluded by default

Redaction replaces the value with `"__REDACTED__"` and records the key path in
`manifest.json > redactions[]`. UI offers Redact all (default) / Keep all. Restore
surfaces what was redacted so the user re-enters it.

## Export flow (Rust `backup_bundle::export`)

1. Walk the curated set into a temp dir, dereferencing symlinks.
2. Extract `mcpServers` from `~/.claude.json` -> `config/mcp.json`.
3. Build plugins manifest (reuse plugins/mcp reads).
4. Apply redactions; write `manifest.json`.
5. Emit `restore.sh`.
6. `tar.gz` to a user-chosen path (native save dialog).

## Restore flow (Rust `backup_bundle::restore`) — two routes

Both routes snapshot the current config first (rotating whole-config backup), so
a restore is itself undoable.

- **Route A - Replace:** overwrite `~/.claude` config with the archive contents.
- **Route B - Merge (ask):** for each differing file, prompt keep/replace; leave
  extra local files alone.

Both: MCP servers are written back into `~/.claude.json` surgically (only the
`mcpServers` key changes); plugins are offered as `claude plugin install ...`
commands; redacted values are flagged for the user to re-enter.

`restore.sh` (standalone) performs Route A: `tar xzf`, copy config into
`~/.claude`, patch `~/.claude.json` mcpServers via `jq`, echo plugin-install
commands. Transparent, works without the app.

## Safety

- Path-jailed writes into `~/.claude` (the surgical `~/.claude.json` patch is the
  single documented exception, same as the MCP tab).
- Snapshot-before-restore.
- Archive validated (manifest present + supported version) before applying.
- Atomic writes where possible (temp + rename).

## UI

Buttons in the **Preferences** tab (app-level actions):

- "Export configuration..." -> secret pre-flight -> save dialog.
- "Restore configuration..." -> open dialog -> Route A/B choice -> (merge:
  conflict list) -> progress + summary (restored, redacted, plugin commands).

## Testing (Rust)

- Export produces the expected tree + `manifest.json`.
- Symlinked skill dereferenced to real content.
- Redaction replaces the value and records the path.
- Restore Route A round-trips: export -> wipe -> restore == identical.
- Merge (Route B) skips vs replaces per decision.
- `~/.claude.json` mcpServers survives the surgical patch; other keys preserved.

## Out of scope (v1)

- Cloud sync / remote storage (local archive only).
- Restoring `projects/`, history, or plugin caches (reinstall via manifest).
- Per-secret encryption (redact-or-keep only).
