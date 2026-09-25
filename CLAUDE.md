# herdr-npm — technical notes

Product behavior is documented in `README.md` and the Gherkin files under `tests/features/`. This file records observations against Herdr 0.9.1 / protocol 22. They are not new product arbitration.

## Socket

- One request/response per Unix socket connection, newline-delimited JSON.
- Timeout 5s, response cap 4 MiB. Verify `id`, then `error` or `result`.
- `plugin.pane.open` returns `result.plugin_pane.pane.pane_id`.
- Do not send `workspace_id` together with a split: Herdr 0.9.1 then errors `use target_pane_id` even if `target_pane_id` is present. The target pane implies the tab.
- `tab.create` returns `result.tab` and `result.root_pane` (used by the run lot).
- `pane.report_metadata` with `source = "herdr-npm"`, token `herdr_npm_sidebar = "v1"`, no `ttl_ms`.
- An I/O timeout after the request is written is uncertain: no automatic retry.

## Geometry

- Left dock: `plugin.pane.open` split `right` on the working-pane target, then `pane.swap` with that target, then `pane.focus`. `plugin.pane.focus` only accepts plugin-managed panes, so returning focus to the working pane after close uses `pane.focus`.
- Preferred outer width 32 columns, clamped to about 15–50% of the local horizontal split via `pane.resize` ratio deltas (same share bounds as herdr-sidebar 0.13.0).
- Grow with `right` on npm; shrink with `left` on the working pane. Herdr chooses the requested edge, so `left` on npm can resize the explorer split instead of the npm/working split.
- Height follows the target pane. Full-tab-height repair (temp tab / reparent) is not V1.
- Working target: leftmost then highest then pane id in the origin tab, excluding herdr-npm and the herdr-sidebar explorer (`herdr-sidebar-explorer` token, or labels `Sidebar`/`Explorer`).

## Environment

Read at startup only. The TUI must not treat its own process cwd as the project: the launcher injects `HERDR_NPM_ORIGIN_*`.

`HERDR_PLUGIN_STATE_DIR` is the directory of `launcher.lock` (exclusive `File::lock`, released on drop or process death). XDG fallback `~/.local/state/herdr/plugins/herdr-npm/`. Discrete toggles wait on the lock before `pane.list`.

## Identity

Visible label `npm`. Recognition is the session token, never the label. Tokens are lost on server restart.

## Workspace catalogue

- `FsProject` owns discovery and disk access. `ProjectCatalog` separates standalone packages from a workspace snapshot with per-member results. YAML is parsed by `serde_yaml_ng`; `globset` matches directory patterns with literal separators, and `walkdir` handles traversal and cycle detection. Exclusions are applied after inclusions; paths are canonicalized and bounded to the workspace.
- Walk scopes prune by literal inclusion prefixes and safe depth bounds before canonicalization. Recursive patterns, classes (which can match separators), and escapes retain conservative depth bounds. Canonical member paths determine final ordering; the walk itself is unsorted to avoid eagerly reading pruned directories.
- Manager signals are searched per directory, stopping at the workspace root. The original standalone `parse_package_json` still rejects missing/empty scripts; workspace members retain empty lists instead.
- The first `.git` boundary stops workspace discovery only. When no package has been found yet, standalone lookup continues upward to the nearest `package.json` as in V1, without checking further workspace declarations.
- `CatalogRow` holds a package path or a `RunIntent` (package path + script name). `SidebarApp.selected` indexes this frozen row catalogue, never the filtered viewport. `search.matches` maps visible rows back to it; `expanded` stores only the unfiltered tree state.
- `flush_intents` resolves each intent against the frozen project before calling `run_script`. Display names and visible row positions never determine the launch directory or manager.
- `tests/workspace_discovery.rs`, `tests/workspace_tui.rs`, `tests/run_argv.rs` and `workspace_scripts.feature` cover discovery, rendered cells, interaction and actual npm/pnpm witness execution. The E2E monorepo case drives the same behavior through an isolated Herdr PTY.

## Build and release

- `[[build]]` runs `scripts/fetch-or-build.sh` (POSIX sh). It keeps the binary of release `v<version>` (version from `Cargo.toml` `[package]`) only if `SOURCE_COMMIT` equals `git rev-parse HEAD`, no tracked file is modified, and `SHA256SUMS` holds one well-formed entry for `herdr-npm-<triple>` that matches the download. Otherwise it prints the reason on stderr, sources `$HOME/.cargo/env` and execs `scripts/build.sh`.
- Triples: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-musl`. Downloads use curl or GNU wget; BusyBox wget is skipped because it does not verify TLS certificates. Temporaries live under `target/release`, so the final `mv` is a rename.
- Herdr 0.9.1 hides the output of a successful build and shows it, capped, when the build fails. The script also appends its status lines to `$HERDR_NPM_BUILD_LOG` when set; `scripts/install-smoke.sh` sets it.
- `scripts/test-fetch-or-build.sh` runs the script offline with fake tools on an isolated PATH; `check.sh all` includes it.
- `scripts/install-smoke.sh <SHA|tag> [--prebuilt]` installs from GitHub in a throwaway profile. It runs a standalone `hello`, then opens a disposable npm workspace (root and two members sharing `hello`) from its root and from `packages/beta`, whose witness must be written once in that member. A `v*` tag resolves to its commit for comparisons. `--prebuilt` validates a published release: empty `HOME`, `PATH` limited to symlinks of herdr/git/node/npm/python3 plus `/usr/bin:/bin:/usr/sbin:/sbin`, and a refusal when cargo or rustc is still reachable. Diagnostics stay in `/tmp/hni-diag.*` after every run.
- `.github/workflows/release.yml` runs on `v*` tags: version gate (tag, `Cargo.toml`, `herdr-plugin.toml`, `Cargo.lock`), `check.sh all` on macOS and Linux, three native builds, then a draft release that is uploaded, downloaded again, compared and published. A rerun resumes a draft; a published release is never overwritten. Immutable releases are enabled.

## E2E profile

`scripts/e2e.sh` isolates the recipe with `XDG_CONFIG_HOME` + `XDG_STATE_HOME` + `HERDR_CONFIG_PATH` + named session `herdr-npm-e2e-<run>`. The socket and config must differ from `~/.config/herdr/herdr.sock` and `~/.config/herdr/config.toml`. `scripts/e2e_journey.py` checks the named session and isolated socket/config paths before acting. Cucumber runs offline; the Python journey is the single live E2E driver. Cleanup stops only that session.

## Copied code

Socket client, left-dock swap and resize math are adapted from herdr-sidebar 0.13.0 (`1a5d37ef84edc91e5b3d3d4e39daa32952e6ecf2`), MIT, see `NOTICE`, as are `scripts/fetch-or-build.sh`, its test and the release workflow.
