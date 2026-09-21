# herdr-npm

Column of npm/pnpm scripts for the current package, as a Herdr plugin.

## Prerequisites

- Herdr **0.9.1** (protocol 22)
- Rust **1.89** (edition 2024) to build from source
- macOS or Linux
- `npm` or `pnpm` on `PATH`
- Recipe shells: `sh`, `bash`, `zsh` (the tab Herdr creates uses the configured shell)

Heartbeat, marketplace listing, Windows, yarn/bun as managers, and prebuilt binaries are out of V1.

## Install from a git SHA

The V1 commit is installable by SHA without a GitHub release:

```sh
herdr plugin install massdo/herdr-npm/plugins/herdr-npm --ref <SHA> --yes
```

`sh plugins/herdr-npm/scripts/install-smoke.sh <SHA>` does that in a throwaway Herdr profile (it does not edit your personal `config.toml`).

## Linked checkout (development)

```sh
git clone https://github.com/massdo/herdr-npm
cd herdr-npm
sh plugins/herdr-npm/scripts/build.sh
herdr plugin link "$PWD/plugins/herdr-npm" --enabled
```

## Keybindings

Do not write these into a personal `config.toml` from the recipe scripts. Opt in yourself:

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

If CI or a headless PTY cannot reproduce Ghostty's `cmd+shift+s`, invoke `herdr-npm.toggle` from the CLI or use `prefix+shift+s`. Confirm `cmd+shift+s` once in a real Ghostty window on macOS.

## Usage

`herdr-npm.toggle` opens or closes the `npm` column on the left of the working pane. Preferred outer width is **32** columns, clamped to about 15–50% of the local split. Height follows the target pane; a pane already at half height keeps that height. The herdr-sidebar explorer is never the split target.

The header shows the package name and `npm`/`pnpm`. Each script row starts with a play icon. `j`/`k` (and arrows) move the selection without wrapping. The mouse wheel scrolls the list without moving the selection. `h`/`l` scroll the full command on the footer. A too-small pane (under 12 inner columns or 4 rows) shows `Terminal too small` and blocks launch.

Enter or a left mouse down on a row's play icon starts `npm run -- <script>` or `pnpm run -- <script>` in a **new tab**, `focus: false`, cwd = package root. Clicking the name or command only selects the script. The sidebar keeps focus and the frozen catalogue. Closing that tab stops an ordinary recipe process; a normal exit leaves the tab and its output readable. `q` typed in a script tab is not eaten by the sidebar.

`/`, Ctrl+F or the header search icon opens a fuzzy search on script names. Enter applies the filter and selects its first result; another Enter launches it. Esc clears the search before closing the pane. While editing, letters such as `q`, `j` and `k` are query text.

`HERDR_NPM_ICONS=ascii` or `nerd` forces the glyph set. By default, the plugin checks for an installed Nerd Font; it cannot determine which font the terminal uses.

## Errors

| Message | Meaning |
|---|---|
| `Cannot determine project directory` | Origin pane has no cwd |
| `No package.json found` | Walk-up found none |
| `package.json is not valid JSON` | Parse error |
| `Cannot read package.json` | Unreadable file |
| `This package.json has no scripts` | Missing or empty `scripts` |
| `package.json scripts must be an object of strings` | Bad `scripts` shape |
| `Terminal too small` | Resize the pane |
| `Script launch not confirmed` | Tab create/send did not ack; inspect the layout, no automatic retry |

## After a Herdr restart

The restored column is inert (session token `herdr_npm_sidebar=v1` is gone). Close that pane with Herdr, then toggle again. There is no automatic replacement.

## Validate

```sh
sh plugins/herdr-npm/scripts/check.sh all   # Rust + Cucumber offline
sh plugins/herdr-npm/scripts/e2e.sh         # isolated Herdr + PTY, macOS or Linux
```
