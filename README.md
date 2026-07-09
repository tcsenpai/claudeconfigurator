# ClaudeConfigurator

A minimal desktop application for configuring Claude Code by editing the files
under `~/.claude`. It provides a focused GUI over the configuration surface:
the entrypoint `CLAUDE.md`, adjacent instruction files, skills, commands,
agents, hooks, plugins, and `settings.json`.

Built with Tauri 2, Svelte 5, and CodeMirror 6. The Rust core owns all
filesystem access and is path-jailed to `~/.claude`. Every write is backed up
and atomic.

## Features

- **CLAUDE.md**: dedicated tab for the entrypoint instruction file.
- **Files**: edit the other root-level `.md` files in `~/.claude`.
- **Skills, Commands, Agents**: browse entries as cards built from their YAML
  frontmatter, then edit the underlying file.
- **Hooks**: a per-event editor for the hooks defined in `settings.json`
  (matcher, command or HTTP hook, timeout, async).
- **Plugins**: list installed plugins and marketplaces, enable or disable them,
  and install, remove, or add a marketplace. Lifecycle operations are delegated
  to the `claude` CLI rather than reimplemented.
- **Settings**: a recursive JSON form that exposes every nested key, with a raw
  JSON mode. Both modes share the same in-memory model, so unsaved edits carry
  across when switching. Content is validated before saving.
- **Markdown preview**: toggle any markdown file between source and a rendered
  view.
- **`@` references**: references such as `@RTK.md`, `@~/.claude/file.md`, or a
  bare `@skill` / `@command` / `@agent` name are highlighted in the editor.
  Ctrl or Cmd click follows a resolved reference and opens its target in the
  correct tab.
- **Generic frontmatter editor**: each YAML key becomes an input (text field or
  chip list). Unknown or nested keys are shown read only and edited through the
  file body.
- **Rotating backups**: each save copies the previous file content to
  `~/.claude/backups/`, keeping the five most recent versions. Writes are
  performed atomically (temp file plus rename).

## Safety

- All filesystem paths are canonicalized and rejected if they resolve outside
  `~/.claude`. This is the single security boundary.
- `settings.json` is parsed and validated before any write; invalid JSON is
  refused.
- Backups are written before every save.

This application edits your live Claude Code configuration. Changes take effect
for your actual setup.

## Requirements

- [Bun](https://bun.sh)
- Rust toolchain (stable)
- Tauri 2 system dependencies for your platform. See the
  [Tauri prerequisites](https://tauri.app/start/prerequisites/).
- The `claude` CLI on `PATH` for plugin install, remove, and marketplace
  operations.

## Quick install

Clone, build the native bundle, and install it for your platform:

```sh
git clone https://github.com/tcsenpai/claudeconfigurator.git
cd claudeconfigurator
./build.sh      # macOS: .app + .dmg    Linux: .deb + .appimage
./install.sh    # macOS: -> /Applications    Linux: dpkg/rpm or ~/.local/bin
```

`build.sh` installs the frontend dependencies and produces the bundle;
`install.sh` places the built bundle onto your system. Both detect the platform
automatically. See [Build](#build) and [Install](#install) for the individual
flags.

## Development

```sh
bun install
bun run tauri dev
```

## Build

```sh
./build.sh              # platform default (macOS: app + dmg, Linux: deb + appimage)
./build.sh --dmg        # macOS installer
./build.sh --app        # macOS app bundle only
./build.sh --appimage   # Linux AppImage
./build.sh --deb        # Linux .deb
```

Or invoke Tauri directly: `bun run tauri build`.

## Install

After building, install the bundle onto the current system:

```sh
./install.sh
```

On macOS this copies the `.app` into `/Applications`. On Linux it installs the
`.deb` or `.rpm` (with sudo), or copies the AppImage into `~/.local/bin`.

## Tests

```sh
cd src-tauri && cargo test
```

The Rust tests cover the security-critical logic: the path jail, frontmatter
round-tripping, backup rotation, reference resolution, and settings writes.

## Extending

Adding a view is intentionally cheap. Create a component under `src/views/` and
add one line to `src/views/registry.ts`. File editing reuses
`src/lib/DocEditor.svelte`, which composes the frontmatter editor, the
CodeMirror pane, and the save logic. Domain data comes from Tauri commands in
`src-tauri/src/fs_cmds.rs`.

## Project layout

```
src/
  App.svelte                shell and sidebar
  views/registry.ts         the extensibility contract
  views/*View.svelte        one component per sidebar entry
  lib/
    api.ts                  typed invoke() wrappers
    DocEditor.svelte        frontmatter editor, CodeMirror pane, save
    CatalogView.svelte      shared skills/commands/agents list
    FrontmatterEditor.svelte
    EditorPane.svelte       CodeMirror wrapper
    editor.ts               CodeMirror setup, reference decorations and clicks
    JsonNode.svelte         recursive JSON form node
    nav.svelte.ts           cross-view navigation store
src-tauri/src/
  jail.rs                   path jail (security boundary)
  frontmatter.rs            YAML split, parse, round-trip
  backup.rs                 rotating backups
  index.rs                  skills/commands/agents catalog
  refs.rs                   reference scan and resolution
  settings.rs               structured settings.json access
  plugins.rs                plugin listing, toggle, CLI delegation
  fs_cmds.rs                Tauri command surface
```

## License

MIT. See [LICENSE](LICENSE).
