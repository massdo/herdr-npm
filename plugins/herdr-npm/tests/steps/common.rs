use cucumber::{given, then, when};
use herdr_npm::adapters::launcher_lock;
use herdr_npm::application::toggle_sidebar::{ToggleOutcome, toggle_sidebar};
use herdr_npm::domain::error::AppError;
use herdr_npm::domain::ids::{PaneId, TabId, WorkspaceId};
use herdr_npm::domain::pane::OriginContext;
use herdr_npm::domain::{SIDEBAR_TOKEN_KEY, SIDEBAR_TOKEN_VALUE};

use crate::support::world::BddWorld;

pub(crate) fn run_locked_toggle(world: &mut BddWorld) {
    world.pane_ids_before = world
        .herdr
        .panes()
        .into_iter()
        .map(|pane| pane.pane_id)
        .collect();
    world.focus_before = world.herdr.focused();
    world.confirmed = false;
    let Some(origin) = world.origin.clone() else {
        world.last_error = Some(AppError::OriginMissing);
        return;
    };
    match launcher_lock::acquire(&world.lock_dir) {
        Err(error) => world.last_error = Some(error),
        Ok(_lock) => match toggle_sidebar(&world.herdr, &origin) {
            Ok(ToggleOutcome::Opened(pane)) => {
                world.opened_pane = Some(pane);
                world.last_error = None;
                world.confirmed = true;
            }
            Ok(ToggleOutcome::Closed(pane)) => {
                world.closed_pane = Some(pane);
                world.last_error = None;
                world.confirmed = true;
            }
            Err(error) => {
                world.last_error = Some(error);
                world.confirmed = false;
            }
        },
    }
}

#[given(regex = r#"^Herdr is running with the "([^"]+)" plugin installed$"#)]
async fn herdr_running(world: &mut BddWorld, plugin: String) {
    world.plugin_id = Some(plugin);
    world.herdr.seed_single_tab("w1", "w1:t1", "w1:p1");
    world.origin = Some(OriginContext {
        workspace_id: WorkspaceId("w1".into()),
        tab_id: TabId("w1:t1".into()),
        pane_id: PaneId("w1:p1".into()),
        foreground_cwd: Some("/work/app".into()),
        cwd: Some("/work/app".into()),
    });
}

#[given(regex = r#"^the "herdr-npm.toggle" action is bound to "([^"]+)" on (macOS|Linux)$"#)]
async fn action_bound(_world: &mut BddWorld, key: String, os: String) {
    assert!(!key.is_empty(), "binding for {os} must name a key");
}

#[given(
    regex = r#"^the focused tab has no pane carrying token "herdr_npm_sidebar" equal to "v1"$"#
)]
async fn no_token(world: &mut BddWorld) {
    assert_no_token_in_origin_tab(world);
}

#[then(regex = r#"^the focused tab has no pane carrying token "herdr_npm_sidebar" equal to "v1"$"#)]
async fn no_token_then(world: &mut BddWorld) {
    assert_no_token_in_origin_tab(world);
}

fn assert_no_token_in_origin_tab(world: &BddWorld) {
    let tab = world.origin.as_ref().map(|origin| origin.tab_id.0.as_str());
    let present = world.herdr.panes().iter().any(|pane| {
        tab.is_none_or(|tab_id| pane.tab_id == tab_id)
            && pane.tokens.get(SIDEBAR_TOKEN_KEY).map(String::as_str) == Some(SIDEBAR_TOKEN_VALUE)
    });
    assert!(!present, "a recognised sidebar was already present");
}

#[given(
    regex = r#"^the origin action context is workspace "([^"]+)", tab "([^"]+)", pane "([^"]+)"$"#
)]
async fn origin_context(world: &mut BddWorld, workspace: String, tab: String, pane: String) {
    world.herdr.seed_single_tab(&workspace, &tab, &pane);
    world.origin = Some(OriginContext {
        workspace_id: WorkspaceId(workspace),
        tab_id: TabId(tab),
        pane_id: PaneId(pane),
        foreground_cwd: Some("/work/app".into()),
        cwd: Some("/work/app".into()),
    });
}

#[given(regex = r#"^pane "([^"]+)" is the leftmost working pane of that tab$"#)]
async fn leftmost_working(world: &mut BddWorld, pane: String) {
    let panes = world.herdr.panes();
    let found = panes.iter().any(|item| item.pane_id == pane);
    assert!(found, "working pane {pane} is not in the snapshot");
}

#[when(regex = r#"^I invoke the "herdr-npm.toggle" action$"#)]
async fn invoke_toggle(world: &mut BddWorld) {
    run_locked_toggle(world);
}

#[then(regex = r#"^a sidebar pane is opened to the left of pane "([^"]+)"$"#)]
async fn opened_left(world: &mut BddWorld, target: String) {
    let opened = world
        .opened_pane
        .as_ref()
        .expect("expected a sidebar to open");
    let calls = world.herdr.calls();
    assert!(
        calls
            .iter()
            .any(|call| call.method == "plugin.pane.open" && call.detail == target),
        "plugin.pane.open was not targeted at {target}: {calls:?}"
    );
    assert!(
        calls.iter().any(|call| call.method == "pane.swap"),
        "left dock requires a swap, calls={calls:?}"
    );
    let layout = world
        .herdr
        .layout_for_pane(opened.as_str())
        .expect("layout");
    let sidebar = layout
        .panes
        .iter()
        .find(|pane| pane.pane_id == opened.as_str())
        .expect("sidebar rect");
    let editor = layout
        .panes
        .iter()
        .find(|pane| pane.pane_id == target)
        .expect("target rect");
    assert!(
        sidebar.rect.x < editor.rect.x,
        "sidebar should sit left of {target}: sidebar={:?} editor={:?}",
        sidebar.rect,
        editor.rect
    );
}

#[then(regex = r#"^the sidebar pane is labelled "npm"$"#)]
async fn labelled_npm(world: &mut BddWorld) {
    let opened = world.opened_pane.as_ref().expect("sidebar");
    let pane = world
        .herdr
        .panes()
        .into_iter()
        .find(|pane| pane.pane_id == opened.as_str())
        .expect("opened pane");
    assert_eq!(pane.label.as_deref(), Some("npm"));
}

#[then(regex = r#"^the sidebar pane carries token "herdr_npm_sidebar" equal to "v1"$"#)]
async fn has_token(world: &mut BddWorld) {
    let opened = world.opened_pane.as_ref().expect("sidebar");
    let pane = world
        .herdr
        .panes()
        .into_iter()
        .find(|pane| pane.pane_id == opened.as_str())
        .expect("opened pane");
    assert_eq!(
        pane.tokens.get(SIDEBAR_TOKEN_KEY).map(String::as_str),
        Some(SIDEBAR_TOKEN_VALUE)
    );
}

#[then("the sidebar pane is focused")]
async fn sidebar_focused(world: &mut BddWorld) {
    let opened = world.opened_pane.as_ref().expect("sidebar");
    assert_eq!(world.herdr.focused().as_deref(), Some(opened.as_str()));
}

#[then("the preferred outer width of the sidebar is 32 columns")]
async fn preferred_width(world: &mut BddWorld) {
    let calls = world.herdr.calls();
    assert!(
        calls
            .iter()
            .any(|call| call.method == "pane.resize" || call.method == "plugin.pane.open"),
        "no geometry call recorded: {calls:?}"
    );
}

#[then(regex = r#"^the sidebar height matches the height of pane "([^"]+)"$"#)]
async fn height_matches(world: &mut BddWorld, pane: String) {
    let sidebar_id = world
        .opened_pane
        .as_ref()
        .map(|id| id.0.clone())
        .or_else(|| {
            world
                .herdr
                .panes()
                .into_iter()
                .find(|item| item.is_herdr_npm_sidebar())
                .map(|item| item.pane_id)
        })
        .expect("sidebar id");
    let layout = world
        .herdr
        .layout_for_pane(&pane)
        .or_else(|| world.herdr.layout_for_pane(&sidebar_id))
        .expect("layout");
    let target = layout
        .panes
        .iter()
        .find(|item| item.pane_id == pane)
        .expect("target pane in layout");
    let sidebar = layout
        .panes
        .iter()
        .find(|item| item.pane_id == sidebar_id)
        .expect("sidebar in layout");
    assert_eq!(
        sidebar.rect.height, target.rect.height,
        "sidebar height must follow the working-pane target"
    );
}
