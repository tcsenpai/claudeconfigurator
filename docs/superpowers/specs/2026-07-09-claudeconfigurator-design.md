# ClaudeConfigurator — v1 Design

**Date:** 2026-07-09
**Goal:** Lean, modular desktop GUI to configure Claude Code by editing `~/.claude`. Small footprint, fast build, readable code, best practices. Start basic, expand by dropping in modules.

## Project values (apply to every change)

1. Lean code — minimum that works; no speculative abstractions.
2. Ultra efficiency — small footprint, fast build/runtime; native/stdlib/installed deps over new ones.
3. Best practices — correctness first, path-jailed fs, validate before writing user config.
4. Smart tricks — elegant minimal solutions, never at the cost of readability.
5. Readable — code reads like its surroundings; comment only non-obvious logic.

## Stack (locked)

- **Tauri 2** — Rust core + web frontend. ~600KB-class bundle vs Electron's ~100MB.
- **Svelte 5** — matches user's other apps.
- **CodeMirror 6** — small, extensible editor; custom decorations for `@`-refs + clickable links.
- **bun** — frontend package manager / dev.
- **Rust owns all filesystem access**, path-jailed to `~/.claude`.

Scope v1: **global `~/.claude` only**. Per-project `.claude/` is a later module.

## Architecture

```
Svelte frontend (webview)
  Sidebar (view registry)  |  Active view module  →  <EditorPane> (CodeMirror)
        │ invoke()                 │ events
Rust core (src-tauri)
  fs      — read/write/list, path-jail to ~/.claude
  backup  — rotating .bak on write (cap N)
  parse   — frontmatter split + @-ref scan
  index   — skills/commands/agents catalog (built once, cached)
```

Rust is the single trust boundary. Frontend is presentation only.

## Modules (views)

Extensibility contract = one array `src/views/registry.ts`. Add a module = drop `src/views/<name>/` + one registry line.

```ts
export const views = [
  { id: 'files',    label: 'Files',    component: FilesView },
  { id: 'skills',   label: 'Skills',   component: SkillsView },
  { id: 'commands', label: 'Commands', component: CommandsView },
  { id: 'agents',   label: 'Agents',   component: AgentsView },
  { id: 'settings', label: 'Settings', component: SettingsView },
]
```

v1 views:

1. **Files** (primary) — root `.md` files (CLAUDE.md + adjacent). List left, editor right. `@`-highlight + ctrl+click follow.
2. **Skills** — cards from `skills/*/SKILL.md` frontmatter; click → edit SKILL.md.
3. **Commands** — grouped by namespace dir; click → edit `.md`.
4. **Agents** — cards from `agents/*.md` frontmatter; click → edit.
5. **Settings** — `settings.json`, CodeMirror JSON mode, validate (parse) on save; block bad write.

Two shared editing primitives (only two):
- `<EditorPane>` — CodeMirror wrapper: dirty-tracking, save, `@`-decorations. Reused by all views.
- `<FrontmatterEditor>` — generic YAML key/value editor. Reused by Skills/Commands/Agents.

## Flags / args / tweakable stuff

**Finding:** tweakable config lives in **YAML frontmatter**, heterogeneous across files (no fixed schema). Observed keys: `description`, `name`, `argument-hint`, `args`, `category`, `complexity`, `personas`, `mcp-servers`, `allowed-tools`, `model`, `metadata`, ... Body uses `$ARGUMENTS`/`$1` placeholders (documentation, not typed params).

**Approach — generic frontmatter panel, zero per-key schema:**

- `parse` module returns frontmatter as an ordered map key → typed value (`scalar | string-list | raw`).
- `<FrontmatterEditor>` renders:
  - scalar → text input
  - string-list (e.g. `allowed-tools`) → chip/tag input
  - unknown/nested → read-only raw; edit falls through to body editor
- Round-trip: edit fields → Rust re-serializes frontmatter + untouched body → backup → write.

Skipped (YAGNI): per-command typed schemas, "run with these flags" preview, `$ARGUMENTS` template UI. Generic editor covers ~90% at ~5% cost. Add typed schema only if a specific command demands it.

## Data flow

**Read:**
```
view mounts → invoke('list_<kind>') → Rust list dir + parse frontmatter → cards
click → invoke('read_file', path) → jail-check → {frontmatter, body, raw}
render → FrontmatterEditor(fields) + EditorPane(body)
```

**Write:**
```
save → invoke('write_file', {path, frontmatter, body})
Rust: jail-check → re-serialize frontmatter+body → rotating backup → atomic write (temp+rename) → emit 'saved'
```

## Safety

- **Path jail:** every path canonicalized; reject if not under `~/.claude`. Single guard fn; all fs commands call it. Non-negotiable.
- **Rotating backup:** before write, copy to `~/.claude/backups/<relpath>.<ts>.bak`; keep newest N (default 5), delete older. Backups in `backups/` (already exists), not alongside source — no `.md` re-scan pollution, no loop.
- **Atomic write:** temp file + rename.
- **Settings validation:** `settings.json` parse-checked before write; invalid JSON blocks the write with an error toast.

## @-ref resolution

Scan body for `@<token>`. Resolve order:
1. path-like (`@RTK.md`, `@~/.claude/x.md`, `@./rel`) → resolve vs file-dir, `~`, `~/.claude`.
2. else name-like → look up in skills/commands/agents index.

Decorate: resolved = clickable link; unresolved = dim/warn underline (non-blocking). ctrl/cmd+click → open target in-app (switch view + load); missing → toast. Name-resolution is best-effort, never blocks rendering.

## Testing (lean)

Rust unit tests on logic that fails silently:
- path-jail: escape attempts (`../`, symlink, absolute outside) rejected.
- frontmatter round-trip: parse→serialize preserves body byte-for-byte.
- backup rotation: keeps N, drops N+1.
- @-ref resolver: path / name / missing cases.

No frontend test framework in v1. Manual smoke via `bun tauri dev`.

## Explicitly out of scope (v1)

- Per-project `.claude/` and project CLAUDE.md.
- settings.json form UI (raw JSON edit only).
- Hooks visual builder.
- Typed per-command flag schemas.
- Frontend test framework.

Add each when the lean version proves insufficient.
