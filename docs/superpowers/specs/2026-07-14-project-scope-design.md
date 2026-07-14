# Project scope: edit any project's Claude config

**Date:** 2026-07-14

Today the app only edits the global `~/.claude`. Add a scope switcher so the user
can point the configurator at any project folder and edit *that project's* Claude
config instead.

## Scopes

A `Scope` is the active editing target:

- **Global** (default): config dir `~/.claude`, MCP file `~/.claude.json`.
- **Project**: config dir `<proj>/.claude`, MCP file `<proj>/.mcp.json`, plus the
  project-root files `<proj>/CLAUDE.md` and `<proj>/.mcp.json`.

Claude actually reads a project's root `CLAUDE.md` and `.mcp.json` (not only
`.claude/`), so project scope must reach those two root files in addition to
`.claude/**`.

## The jail (the security-critical change)

`jail::root()` currently returns a fixed `~/.claude`. It becomes the ACTIVE
scope's config dir, held in a process-global `RwLock<Scope>` in a new `scope`
module that `jail` reads.

- Global: `root()` = `~/.claude`. Every existing relative path (`skills/...`,
  `settings.json`) resolves exactly as before — no caller changes.
- Project: `root()` = `<proj>/.claude`. The same relative paths now resolve under
  the project's `.claude/`, for free.

`jail::resolve` keeps rejecting `..` and requiring the path to sit under `root()`,
WITH one scoped exception: in project scope, the two project-root files
(`../CLAUDE.md` and `../.mcp.json` relative to `.claude/`, i.e. `<proj>/CLAUDE.md`
and `<proj>/.mcp.json`) are allowed. This is expressed not as a `..` path (still
rejected) but via an explicit allowlist: `resolve` recognizes the sentinel rels
`ROOT:CLAUDE.md` / `ROOT:.mcp.json` (or a dedicated `resolve_root_file` helper)
and maps them to the project dir. Nothing else outside `.claude/` is reachable.

Precise boundary: project scope may touch `<proj>/.claude/**` plus exactly
`<proj>/CLAUDE.md` and `<proj>/.mcp.json`. Never `src/` or anything else.

## Rust

New `scope.rs`:
- `Scope { kind: Global|Project, config_dir: PathBuf, project_dir: Option<PathBuf> }`.
- Process-global `RwLock<Scope>`, default Global.
- `scope_get() -> {kind, label, path}`.
- `scope_set_global()`.
- `scope_open_project(path)`: if `<path>/.claude` exists -> set Project scope;
  else return a distinct "no-claude" result so the UI can offer to create it.
- `scope_create_claude(path)`: mkdir `<path>/.claude`, then set Project scope.

`jail`:
- `root()` reads `scope.config_dir`.
- `resolve` unchanged for normal rels; add `resolve_root_file(name)` for the two
  whitelisted project-root files (global scope: these live at `~/.claude/` and
  `~/.claude.json` respectively, so map accordingly).

`mcp.rs`: target `scope`'s MCP file — global `~/.claude.json` (key `mcpServers`)
vs project `<proj>/.mcp.json` (also key `mcpServers`, but the file is MCP-only so
it's the whole doc). Reads/writes go through the scope.

Backups: project scope writes rotating backups under `<proj>/.claude/backups/`
(same relative logic, new root). Global unchanged.

Global-only features stay global: `plugins.rs` (no project plugins) and the
bundle export/restore (whole-global) remain bound to `~/.claude`; the UI hides
those tabs in project scope.

## Frontend

- `scope.svelte.ts` store: `{kind, label, path}`, loaded at startup, updated on
  switch. All views re-fetch when it changes (they already `$effect` on load).
- Sidebar scope selector above the tabs: shows active scope
  ("Global (~/.claude)" or the project name). Actions: "Global", "Open
  project..." (folder picker via tauri dialog `directory:true`), and a short
  recent-projects list (persisted in appconfig).
- No-`.claude` flow: picker returns a folder without `.claude/` -> confirm dialog
  "No .claude here - create one?" -> `scope_create_claude` -> switch.
- Registry filters tabs by scope: project scope shows Files, Skills, Commands,
  Agents, Settings, MCP, Hooks; hides Plugins and the global export/restore
  section of Preferences. `CLAUDE.md` in project scope is edited via Files (it's
  a project-root file) rather than the dedicated global CLAUDE.md tab.
- Switching scope resets the active tab if the current one is hidden.

## Data flow

Switch scope -> `scope_set_*` (Rust updates the RwLock) -> frontend store updates
-> every view's load `$effect` re-runs against the new `jail::root()` -> lists +
editors now show the project's config. No path logic changes in the views.

## Safety

- The jail stays the single boundary; project scope only widens it to the
  project's `.claude/` + the two whitelisted root files. `..` still rejected.
- Switching scope never writes anything; it only changes where reads/writes go.
- Backups still precede every write, now under the active scope's `backups/`.
- Creating `.claude/` in a project is an explicit user action (confirm dialog).

## Testing (Rust)

- `resolve` under a project scope maps `skills/x` -> `<proj>/.claude/skills/x`.
- `resolve_root_file("CLAUDE.md")` -> `<proj>/CLAUDE.md` in project scope,
  `~/.claude/CLAUDE.md` in global.
- `resolve` still rejects `..` and paths outside the scope root in both scopes.
- Project-root `.mcp.json` MCP read/write via the scoped mcp file; global still
  uses `~/.claude.json`.
- `scope_open_project` distinguishes has-.claude vs no-.claude.
- Switching scope changes what `list_root_md` / catalog return.

## Out of scope (v1)

- Editing arbitrary project files beyond the Claude surface.
- Project-level plugins (Claude has none) or a project export/restore bundle.
- settings.local.json special handling beyond showing it as a file.
