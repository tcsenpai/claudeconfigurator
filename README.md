# ClaudeConfigurator

Minimal desktop GUI to configure Claude Code by editing `~/.claude`.
Tauri 2 + Svelte 5 + CodeMirror 6. Rust owns the filesystem, path-jailed to `~/.claude`.

## Features (v1)

- **Files** — edit `CLAUDE.md` and adjacent root `.md` files.
- **Skills / Commands / Agents** — browse cards from frontmatter, edit `SKILL.md` / command / agent files.
- **Settings** — edit `settings.json` with JSON validation on save.
- **`@`-references** — highlighted in markdown; ctrl/cmd+click follows the target (file path, or `@skill`/`@command`/`@agent` name) into the right view.
- **Generic frontmatter editor** — every YAML key becomes a field (text / chip-list); unknown/nested keys shown read-only.
- **Rotating backups** — each save copies the file to `~/.claude/backups/…` (keeps newest 5). Writes are atomic.

## Develop

```sh
bun install
bun run tauri dev      # run the app
```

## Build

```sh
bun run tauri build    # bundled app
```

Rust tests:

```sh
cd src-tauri && cargo test
```

## Add a module

Adding a view = drop a folder under `src/views/` and add one line to
`src/views/registry.ts`. Editing files reuses `src/lib/DocEditor.svelte`
(frontmatter editor + CodeMirror pane + save). Domain data comes from Rust
commands in `src-tauri/src/fs_cmds.rs`.

## Layout

```
src/
  App.svelte              shell + sidebar
  views/registry.ts       the extensibility contract
  views/*View.svelte      one per sidebar entry
  lib/
    api.ts                typed invoke() wrappers
    DocEditor.svelte      frontmatter + editor + save
    CatalogView.svelte    shared skills/commands/agents list
    FrontmatterEditor.svelte
    EditorPane.svelte     CodeMirror wrapper
    editor.ts             CodeMirror setup + @-ref decorations/click
    nav.svelte.ts         cross-view navigation store
src-tauri/src/
  jail.rs                 path jail (security boundary)
  frontmatter.rs          YAML split / parse / round-trip
  backup.rs               rotating backups
  index.rs                skills/commands/agents catalog
  refs.rs                 @-ref scan + resolve
  fs_cmds.rs              Tauri command surface
```
