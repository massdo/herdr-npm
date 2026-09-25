# herdr-npm

Column of npm/pnpm scripts for the current package or declared workspace, as a Herdr plugin.

## Prerequisites

- Herdr **0.9.1** (protocol 22)
- macOS or Linux
- `npm` or `pnpm` on `PATH`
- Script tabs use Herdr's configured shell: `sh`, `bash` and `zsh` are supported
- Rust **1.89** (edition 2024), only when the install builds from source (see below)

Windows is not supported. Yarn and Bun are not supported as package managers
yet: projects that use them run their scripts through npm.

## Install from GitHub

Install the latest release (see
[Releases](https://github.com/massdo/herdr-npm/releases)) and confirm that Herdr
enabled the plugin:

```sh
herdr plugin install massdo/herdr-npm --ref v0.2.0 --yes
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

## Linked checkout (development)

```sh
git clone https://github.com/massdo/herdr-npm
cd herdr-npm
sh scripts/build.sh
herdr plugin link "$PWD" --enabled
```

## Keybindings

To toggle the column from the keyboard, add a binding to your Herdr
`config.toml`, for example:

```toml
# macOS, in a terminal that forwards Cmd shortcuts (such as Ghostty)
[[keys.command]]
key = "cmd+shift+s"
type = "plugin_action"
command = "herdr-npm.toggle"

# Linux (Herdr prefix, then Shift+S)
[[keys.command]]
key = "prefix+shift+s"
type = "plugin_action"
command = "herdr-npm.toggle"
```

## Usage

`herdr-npm.toggle` opens or closes the `npm` column on the left of the working pane. Preferred outer width is **32** columns, clamped to about 15–50% of the local split. Height follows the target pane; a pane already at half height keeps that height. The herdr-sidebar explorer is never the split target.

The header shows the package name and `npm`/`pnpm`. Each script row starts with a play icon. `j`/`k` (and arrows) move the selection without wrapping. The mouse wheel scrolls the list without moving the selection. `h`/`l` scroll the full command on the footer. A too-small pane (under 12 inner columns or 4 rows) shows `Terminal too small` and blocks launch.

Enter or a left mouse down on a row's play icon starts `npm run -- <script>` or `pnpm run -- <script>` in a **new tab**, `focus: false`, cwd = package root. Clicking the name or command only selects the script. The sidebar keeps focus and the frozen catalogue. Closing that tab stops the script (a process that daemonizes itself may survive); a normal exit leaves the tab and its output readable. `q` typed in a script tab is not eaten by the sidebar.

`/`, Ctrl+F or the header search icon opens a fuzzy search on script names. Enter applies the filter and selects its first result; another Enter launches it. Esc clears the search before closing the pane. While editing, letters such as `q`, `j` and `k` are query text.

`HERDR_NPM_ICONS=ascii` or `nerd` forces the glyph set. By default, the plugin checks for an installed Nerd Font; it cannot determine which font the terminal uses.

## Workspaces / monorepos

Open the column from the workspace root, an intermediate directory or a declared
member. It discovers `pnpm-workspace.yaml` or `package.json` `workspaces` (an array
or an object with `packages`). The nearest enclosing workspace wins, stopping at
the nearest Git root. A nested package that is not declared remains a standalone
package. When both declarations exist at one root, the YAML file takes precedence.
The Git boundary applies only to workspace discovery. Standalone lookup preserves
the original walk to the nearest `package.json`, even above a Git root, without
adopting workspace declarations encountered beyond that boundary.

Patterns support `*`, `**`, `?`, character classes and `{a,b}` alternatives.
`!` exclusions apply to all inclusions. Only declared directories containing a
`package.json` appear; `node_modules`, `.git`, external symlinks and cycles are
ignored, and internal aliases are deduplicated. The root package comes first,
then members in relative-path order. YAML workspaces can omit a root manifest.
Discovery prunes directories outside literal inclusion prefixes and bounds the
depth of non-recursive patterns when safe. Recursive globs and character classes
can require broader traversal.

Each group shows its relative path, package name and manager. The root starts
expanded; opening from a member also expands that member and selects its first
script. Packages without scripts show `No scripts`. A broken member shows a local
diagnostic while the other groups remain usable. A broken workspace declaration
shows its path and stops discovery. Long group details are available in the footer.

| Action | Workspace behavior |
|---|---|
| `j`/`k`, Up/Down | Move through visible groups and scripts without wrapping |
| Enter or click on a group | Expand or collapse; never launch |
| Right / Left | Expand a group / collapse the current group, selecting its header |
| `h`/`l` | Scroll the command or group details in the footer |
| Enter or play icon on a script | Launch that script in its package directory |
| Click a script name or command | Select only |

Search covers script names across all valid packages, including collapsed groups.
Results are grouped by package path, so identical package or script names stay
distinct. A nonempty filter temporarily expands matching groups; Esc clears it
and restores the saved tree. Enter applies the search; a second Enter launches.
Outside a workspace, Left/Right still scroll the command like `h`/`l`.

Each package inherits the nearest manager signal up to the workspace root:
at each directory, a supported `packageManager` declaration wins, followed by
`pnpm-lock.yaml`, then `package-lock.json`. A local signal wins over a parent;
no signal means npm. Yarn/bun declarations keep the existing npm fallback.
Standalone packages still use only their own manifest and lockfiles.
Packages, scripts, directories and managers stay frozen until the column closes.
Each action opens exactly one background tab in the captured Herdr workspace;
there is no recursive execution or automatic retry.

## Errors

| Message | Meaning |
|---|---|
| `Cannot determine project directory` | Origin pane has no cwd |
| `No package.json found` | Walk-up found none |
| `package.json is not valid JSON` | Parse error |
| `Cannot read package.json` | Unreadable file |
| `This package.json has no scripts` | Missing or empty `scripts` |
| `Invalid workspace <path>: ...` | Invalid or unreadable workspace declaration |
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
sh scripts/check.sh all                 # Rust + Cucumber + fetch-or-build, offline
sh scripts/e2e.sh                       # isolated Herdr + PTY, macOS or Linux
sh scripts/install-smoke.sh <SHA|tag>   # install from GitHub in a throwaway Herdr profile
```

The full E2E journey includes a disposable six-package pnpm workspace, opened
from its root and from `apps/mcp`. `HERDR_NPM_E2E_CASE=monorepo sh scripts/e2e.sh`
runs that journey alone. It uses harmless witness scripts, never a real project's
operational scripts. Set `HERDR_NPM_E2E_ARTIFACTS` to a directory to retain the
isolated configuration, launch records and logs.

`scripts/install-smoke.sh` installs the plugin from GitHub at a commit SHA or a
release tag, in a throwaway Herdr profile that leaves your personal `config.toml`
alone. In a standalone package, it opens the column, runs a witness script and
closes the column. It then opens a disposable npm workspace, whose root and two
members share a script name, from the root and from a member. It checks the
three groups and runs the member's script once, in the member's directory.

Add `--prebuilt` to validate a release install, with a published tag or the full
SHA of its commit. The run happens in a disposable environment without Rust
(empty `HOME`, and a `PATH` limited to Herdr, Git, Node, npm, Python and the
system directories). It checks that the installed commit, the tag and
`SOURCE_COMMIT` agree, that the binary matches `SHA256SUMS`, that
`herdr-plugin.toml` is unchanged, that the build reported the download, and
that nothing was compiled (no `target/release/deps`). Each run keeps its
diagnostics in `/tmp/hni-diag.*`.
