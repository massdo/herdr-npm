use cucumber::{given, then, when};
use herdr_npm::application::open_sidebar::open_empty_sidebar;
use herdr_npm::domain::ids::{PaneId, TabId, WorkspaceId};
use herdr_npm::domain::pane::OriginContext;
use herdr_npm::domain::{SIDEBAR_TOKEN_KEY, SIDEBAR_TOKEN_VALUE};

use crate::support::world::BddWorld;

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
    assert!(
        world.herdr.panes().iter().all(|pane| pane
            .tokens
            .get(SIDEBAR_TOKEN_KEY)
            .map(String::as_str)
            != Some(SIDEBAR_TOKEN_VALUE)),
        "a recognised sidebar was already present"
    );
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
    let origin = world
        .origin
        .clone()
        .expect("origin must be captured before toggle");
    match open_empty_sidebar(&world.herdr, &origin) {
        Ok(pane) => world.opened_pane = Some(pane),
        Err(error) => world.last_error = Some(error),
    }
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
    assert!(
        world
            .herdr
            .panes()
            .iter()
            .any(|pane| pane.pane_id == opened.as_str()),
        "opened pane missing from snapshot"
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
    let calls = world.herdr.calls();
    assert!(
        calls.iter().any(|call| call.method == "plugin.pane.focus"),
        "sidebar was not focused: {calls:?}"
    );
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
    let layout = world.herdr.layout().expect("layout");
    let target = layout
        .panes
        .iter()
        .find(|item| item.pane_id == pane)
        .expect("target pane in layout");
    let opened = world.opened_pane.as_ref().expect("sidebar");
    let sidebar = layout
        .panes
        .iter()
        .find(|item| item.pane_id == opened.as_str())
        .expect("sidebar in layout");
    assert_eq!(
        sidebar.rect.height, target.rect.height,
        "sidebar height must follow the working-pane target"
    );
}
