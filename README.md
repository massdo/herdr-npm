# herdr-npm

Column of npm/pnpm scripts for the current package, as a Herdr plugin.

## Prerequisites

- Herdr **0.9.1** (protocol 22)
- Rust **1.89** (edition 2024) when the install builds from source (see below)
- macOS or Linux
- `npm` or `pnpm` on `PATH`
- Recipe shells: `sh`, `bash`, `zsh` (the tab Herdr creates uses the configured shell)

Heartbeat, marketplace listing, Windows, and yarn/bun as managers are out of V1.

## Install from GitHub

Install a published release tag (see
[Releases](https://github.com/massdo/herdr-npm/releases)) and confirm that Herdr
enabled the plugin:

```sh
herdr plugin install massdo/herdr-npm --ref v<version> --yes
herdr plugin list
```

On macOS arm64, macOS x86_64 and Linux x86_64, the build then downloads the
binary published with that release, with `curl` or GNU `wget`, and no Rust. It
keeps the binary only when the release's `SOURCE_COMMIT` is the installed
commit, no tracked file of the checkout is modified, and the file matches its
single `SHA256SUMS` entry (`sha256sum` or `shasum`). These two files detect
corruption and a commit mismatch; they come from the same GitHub release, so
they do not authenticate the binary independently.

Any other case builds from source and prints the reason on stderr: another
platform, a commit that no release was built from (such as `main` after the
tag, even when the Cargo version is unchanged), or a failed download or check.
That build needs Cargo **1.89** or newer; `$HOME/.cargo/env` is sourced when
present. If Homebrew's older Cargo still takes precedence over an installed
rustup toolchain, run `export PATH="$HOME/.cargo/bin:$PATH"` in your shell
first.

Herdr 0.9.1 shows build output only when the build fails. To keep the status
lines of a successful install, set `HERDR_NPM_BUILD_LOG` to a file path before
installing.

To install the current `main` branch or an exact commit instead:

```sh
herdr plugin install massdo/herdr-npm --yes
herdr plugin install massdo/herdr-npm --ref <SHA> --yes
```

In Herdr, open a project with a `package.json`, then run
`herdr plugin action invoke herdr-npm.toggle` from a shell to open the scripts
column. This plugin repository itself does not contain a `package.json`.

`sh scripts/install-smoke.sh <SHA>` does that in a throwaway Herdr profile (it does not edit your personal `config.toml`).

`sh scripts/install-smoke.sh <SHA> --prebuilt` validates a release install
instead; pass the full SHA of a published tag. It runs in a disposable
environment without Rust (empty `HOME`, and a `PATH` limited to Herdr, Git,
Node, npm, Python and the system directories) and checks that the installed
commit, the tag and `SOURCE_COMMIT` agree, that the binary matches
`SHA256SUMS`, that `herdr-plugin.toml` is unchanged, that the build reported
the download, and that nothing was compiled (no `target/release/deps`). A
failed run keeps its diagnostics in `/tmp/hni-diag.*`.

## Linked checkout (development)

```sh
git clone https://github.com/massdo/herdr-npm
cd herdr-npm
sh scripts/build.sh
herdr plugin link "$PWD" --enabled
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

## Security

Only run scripts from projects you trust. Scripts run through npm or pnpm in a
Herdr shell tab, with your user's permissions and environment; they are not
sandboxed. Package-manager lifecycle scripts may also run. See [SECURITY.md](SECURITY.md)
for the reporting policy.

## Validate

```sh
sh scripts/check.sh all   # Rust + Cucumber + fetch-or-build, offline
sh scripts/e2e.sh         # isolated Herdr + PTY, macOS or Linux
```
