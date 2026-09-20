# herdr-npm — technical notes

Product behavior is documented in README.md and the Gherkin files. This file records observations against Herdr 0.9.1 / protocol 22. They are not new product arbitration.

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
- Height follows the target pane. Full-tab-height repair (temp tab / reparent) is not V1.
- Working target: leftmost then highest then pane id in the origin tab, excluding herdr-npm and the herdr-sidebar explorer (`herdr-sidebar-explorer` token, or labels `Sidebar`/`Explorer`).

## Environment

Read at startup only. The TUI must not treat its own process cwd as the project: the launcher injects `HERDR_NPM_ORIGIN_*`.

`HERDR_PLUGIN_STATE_DIR` is the directory of `launcher.lock` (exclusive `File::lock`, released on drop or process death). XDG fallback `~/.local/state/herdr/plugins/herdr-npm/`. Discrete toggles wait on the lock before `pane.list`.

## Identity

Visible label `npm`. Recognition is the session token, never the label. Tokens are lost on server restart.

## Copied code

Socket client, left-dock swap and resize math are adapted from herdr-sidebar 0.13.0 (`1a5d37ef84edc91e5b3d3d4e39daa32952e6ecf2`), MIT, see `NOTICE`.
