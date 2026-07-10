<h1>
  <img src="https://github.com/user-attachments/assets/1891123a-b215-44d9-b86c-1d0ae4e5921d" alt="icon" width="40" height="40" align="top" />
  ClaudeConfigurator
</h1>

A minimal desktop application for configuring Claude Code by editing the files
under `~/.claude`. It provides a focused GUI over the configuration surface:
the entrypoint `CLAUDE.md`, adjacent instruction files, skills, commands,
agents, hooks, plugins, and `settings.json`.

Built with Tauri 2, Svelte 5, and CodeMirror 6. The Rust core owns all
filesystem access and is path-jailed to `~/.claude`. Every write is backed up
and atomic.

<img width="1118" height="735" alt="CleanShot 2026-07-09 at 09 41 25" src="https://github.com/user-attachments/assets/703dc82a-36e2-4ea1-9dba-8f19d5047cf3" />


## Features

- **CLAUDE.md**: dedicated tab for the entrypoint instruction file.
- **Files**: edit the other root-level `.md` files in `~/.claude`.
- **Skills, Commands, Agents**: browse entries as cards built from their YAML
  frontmatter, then edit the underlying file.
- **Create and import**: a `+` button in the Skills, Commands, Agents, and
  Files views creates a new entry from a name and a starter template, or
  imports an existing file (or a whole skill folder) from anywhere on disk into
  `~/.claude`. Commands accept an optional namespace. Existing targets are never
  overwritten.
- **Delete**: a red `x` on each list row and a Delete button in the editor
  toolbar remove an entry after a confirmation. Deleting a skill removes its
  whole folder. The item is copied into `~/.claude/backups/` before removal, so
  it is recoverable.
- **Hooks**: a per-event editor for the hooks defined in `settings.json`
  (matcher, command or HTTP hook, timeout, async).
- **Plugins**: list installed plugins and marketplaces, enable or disable them,
  and install, remove, or add a marketplace. Lifecycle operations are delegated
  to the `claude` CLI rather than reimplemented.
- **Graph**: a dependency graph of the config. Nodes are files (plus
  `settings.json` and the hook scripts it invokes); edges are `@`-references and
  hook script invocations. It shows an ego-graph around a focused file (default
  `CLAUDE.md`): click a node to re-center, double-click to open it. Answers "what
  does this reference" and "what references this".
- **Settings**: a schema-driven form grouped by intent (Model & behavior,
  Permissions & safety, Environment, Git & attribution, Appearance, Plugins).
  Each known key renders as a proper widget (toggle, number stepper, 0..1
  slider, enum dropdown, chip list) with its JSON Schema description as inline
  help; complex objects with a dedicated tab link there, and anything unknown
  falls back to a generic JSON editor. A raw JSON mode shares the same in-memory
  model, so unsaved edits carry across. Content is validated before saving.
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
- **Preferences**: a `Preferences` tab holds the app's own settings (stored
  outside `~/.claude`, in the platform app-config directory). It includes
  autosave: when on, an edited file saves automatically after a configurable
  inactivity delay (default 5s, off by default). Autosave uses the same
  validation and backup as a manual save, and applies to the markdown editors
  (not the Claude `settings.json` editor).

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

### macOS: "app is damaged" on first launch

The macOS builds are ad-hoc signed but not signed with an Apple Developer
certificate or notarized. On Apple Silicon a downloaded, unsigned app can be
quarantined by Gatekeeper and refuse to open with "ClaudeConfigurator is
damaged and can't be opened". Remove the quarantine attribute to allow it:

```sh
xattr -dr com.apple.quarantine /Applications/ClaudeConfigurator.app
```

Then open it normally. Building from source locally avoids this entirely.

## Releases

Pushing a `v*` tag triggers the `release` GitHub Actions workflow
(`.github/workflows/release.yml`), which builds native bundles for macOS
(Apple Silicon and Intel), Linux, and Windows and attaches them to a draft
GitHub release for that tag. Review the draft, then publish it.

```sh
git tag v0.2.0
git push origin v0.2.0
```

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
    AddDialog.svelte        create / import new entries
    nav.svelte.ts           cross-view navigation store
src-tauri/src/
  jail.rs                   path jail (security boundary)
  frontmatter.rs            YAML split, parse, round-trip
  backup.rs                 rotating backups
  index.rs                  skills/commands/agents catalog
  refs.rs                   reference scan and resolution
  graph.rs                  dependency graph (nodes + edges)
  settings.rs               structured settings.json access
  plugins.rs                plugin listing, toggle, CLI delegation
  create.rs                 create / import / delete entries
  appconfig.rs              the app's own preferences (autosave)
  fs_cmds.rs                Tauri command surface
```

## License

MIT. See [LICENSE](LICENSE).
