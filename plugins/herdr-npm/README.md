# herdr-npm

Column of npm/pnpm scripts for the current package, as a Herdr plugin.

## Prerequisites

- Herdr 0.9.1
- Rust 1.89+ (edition 2024)
- macOS or Linux
- npm or pnpm on PATH for later lots; the skeleton only opens an empty column

Supported recipe shells later: `sh`, `bash`, `zsh`.

## Install from source (linked checkout)

```sh
git clone https://github.com/massdo/herdr-npm
cd herdr-npm
sh plugins/herdr-npm/scripts/build.sh
herdr plugin link "$PWD/plugins/herdr-npm"
herdr config check
```

Do not add keybindings to your personal `config.toml` while developing. Use a recipe profile, or invoke:

```sh
herdr --session <recipe-session> plugin action invoke --plugin herdr-npm toggle
```

Bindings to document when you opt in:

```toml
# macOS / Ghostty
[[keys.command]]
key = "cmd+shift+s"
type = "plugin_action"
command = "herdr-npm.toggle"

# Linux (Herdr prefix then Shift+S)
[[keys.command]]
key = "prefix+shift+s"
type = "plugin_action"
command = "herdr-npm.toggle"
```

## Usage (V1 skeleton)

`herdr-npm.toggle` opens an empty `npm` column on the left of the working pane (preferred outer width 32 columns, height of that pane). Press `q` in the column to close it.

Catalogue, launch, two-state toggle and e2e recipe are later lots.

## Limits

- After a Herdr restart, a restored column is inert (session tokens are gone). Close it with Herdr, then toggle again. There is no automatic replacement.
- Workspaces, yarn/bun as managers, Windows, marketplace and prebuilt binaries are out of V1.
