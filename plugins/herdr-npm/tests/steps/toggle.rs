use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use cucumber::{given, then, when};
use herdr_npm::adapters::launcher_lock;
use herdr_npm::application::toggle_sidebar::{ToggleOutcome, toggle_sidebar};
use herdr_npm::domain::error::AppError;
use herdr_npm::domain::ids::{PaneId, TabId, WorkspaceId};
use herdr_npm::domain::pane::OriginContext;
use herdr_npm::domain::{SIDEBAR_LABEL, SIDEBAR_TOKEN_KEY, SIDEBAR_TOKEN_VALUE};

use crate::support::e2e;
use crate::support::world::BddWorld;

fn origin_tab(world: &BddWorld) -> &str {
    world
        .origin
        .as_ref()
        .map(|origin| origin.tab_id.0.as_str())
        .unwrap_or("w1:t1")
}

fn origin_workspace(world: &BddWorld) -> &str {
    world
        .origin
        .as_ref()
        .map(|origin| origin.workspace_id.0.as_str())
        .unwrap_or("w1")
}

fn recognised(world: &BddWorld) -> Vec<herdr_npm::domain::pane::PaneInfo> {
    let tab = origin_tab(world);
    world
        .herdr
        .panes()
        .into_iter()
        .filter(|pane| pane.tab_id == tab && pane.is_herdr_npm_sidebar())
        .collect()
}

#[given("the focused tab already has a herdr-npm sidebar recognised by token")]
async fn already_has_sidebar(world: &mut BddWorld) {
    let tab = origin_tab(world).to_string();
    let split_from = world
        .origin
        .as_ref()
        .map(|origin| origin.pane_id.0.clone())
        .unwrap_or_else(|| "w1:p1".into());
    let id = world.herdr.add_recognised_sidebar(&tab, &split_from);
    world.recognised_ids = vec![id];
}

#[given(regex = r#"^that sidebar was split from pane "([^"]+)"$"#)]
async fn split_from(world: &mut BddWorld, pane: String) {
    let tab = origin_tab(world).to_string();
    let working = world
        .herdr
        .panes()
        .into_iter()
        .find(|item| item.tab_id == tab && !item.is_herdr_npm_sidebar())
        .expect("working pane to rename");
    if working.pane_id != pane {
        world.herdr.rename_pane(&working.pane_id, &pane);
        if let Some(origin) = world.origin.as_mut()
            && origin.pane_id.0 == working.pane_id
        {
            origin.pane_id = PaneId(pane);
        }
    }
}

#[given("the focus is on the sidebar pane")]
async fn focus_sidebar(world: &mut BddWorld) {
    let id = recognised(world)
        .into_iter()
        .next()
        .expect("recognised sidebar")
        .pane_id;
    world.herdr.set_focus(&id);
    if let Some(origin) = world.origin.as_mut() {
        origin.pane_id = PaneId(id);
    }
}

#[given("the focus is on another pane of that tab")]
async fn focus_other(world: &mut BddWorld) {
    let tab = origin_tab(world).to_string();
    let other = world
        .herdr
        .panes()
        .into_iter()
        .find(|pane| pane.tab_id == tab && !pane.is_herdr_npm_sidebar())
        .expect("working pane");
    world.herdr.set_focus(&other.pane_id);
    if let Some(origin) = world.origin.as_mut() {
        origin.pane_id = PaneId(other.pane_id.clone());
    }
    world.focus_before = Some(other.pane_id);
}

#[given(regex = r#"^the tab "([^"]+)" has a herdr-npm sidebar recognised by token$"#)]
async fn tab_has_sidebar(world: &mut BddWorld, tab: String) {
    let workspace = origin_workspace(world).to_string();
    let editor = format!("{tab}-editor");
    world.herdr.ensure_working_pane(&workspace, &tab, &editor);
    let id = world.herdr.add_recognised_sidebar(&tab, &editor);
    world.recognised_ids.push(id);
}

#[given(
    regex = r#"^the focused tab is "([^"]+)" and has no pane carrying token "herdr_npm_sidebar" equal to "v1"$"#
)]
async fn focused_tab_without_token(world: &mut BddWorld, tab: String) {
    let workspace = origin_workspace(world).to_string();
    let pane = format!("{tab}-editor");
    world.herdr.ensure_working_pane(&workspace, &tab, &pane);
    world.herdr.set_focus(&pane);
    world.origin = Some(OriginContext {
        workspace_id: WorkspaceId(workspace),
        tab_id: TabId(tab),
        pane_id: PaneId(pane),
        foreground_cwd: Some("/work/app".into()),
        cwd: Some("/work/app".into()),
    });
}

#[given(
    regex = r#"^the tab "([^"]+)" has a pane carrying token "herdr_npm_sidebar" equal to "v1"$"#
)]
async fn tab_has_token_pane(world: &mut BddWorld, tab: String) {
    let workspace = origin_workspace(world).to_string();
    world
        .herdr
        .add_token_pane(&workspace, &tab, &format!("{tab}-token"));
}

#[given("the focused tab is \"current\" and has no such token")]
async fn focused_current(world: &mut BddWorld) {
    focused_tab_without_token(world, "current".into()).await;
}

#[given(
    regex = r#"^the focused tab has a pane labelled "npm" that does not carry token "herdr_npm_sidebar" equal to "v1"$"#
)]
async fn labelled_npm_foreign(world: &mut BddWorld) {
    let tab = origin_tab(world).to_string();
    world.foreign_pane = Some(world.herdr.add_labelled_npm(&tab));
}

#[given("the pane list returned by Herdr cannot be interpreted")]
async fn unreadable_list(world: &mut BddWorld) {
    world.herdr.set_list_unreadable();
}

#[given("the origin workspace, tab or pane of the action is missing")]
async fn missing_origin(world: &mut BddWorld) {
    world.origin = None;
}

#[given("the captured origin pane is no longer present in the pane list")]
async fn origin_gone(world: &mut BddWorld) {
    if let Some(origin) = world.origin.clone() {
        world.herdr.remove_pane(origin.pane_id.as_str());
    }
}

#[given(
    regex = r#"^the focused tab has two panes carrying token "herdr_npm_sidebar" equal to "v1"$"#
)]
async fn two_sidebars(world: &mut BddWorld) {
    let tab = origin_tab(world).to_string();
    let workspace = origin_workspace(world).to_string();
    let split_from = world
        .origin
        .as_ref()
        .map(|origin| origin.pane_id.0.clone())
        .unwrap_or_else(|| "w1:p1".into());
    let first = world.herdr.add_recognised_sidebar(&tab, &split_from);
    world
        .herdr
        .add_token_pane(&workspace, &tab, &format!("{tab}-sidebar-2"));
    world.recognised_ids = vec![first, format!("{tab}-sidebar-2")];
}

#[given("the focused tab has no working pane that can be used as a split target")]
async fn no_working_target(world: &mut BddWorld) {
    let workspace = origin_workspace(world).to_string();
    let tab = origin_tab(world).to_string();
    world.herdr.seed_explorer_only(&workspace, &tab, "explorer");
    world.origin = Some(OriginContext {
        workspace_id: WorkspaceId(workspace),
        tab_id: TabId(tab),
        pane_id: PaneId("explorer".into()),
        foreground_cwd: Some("/work/app".into()),
        cwd: Some("/work/app".into()),
    });
}

#[given("opening a pane returns an uncertain transport error")]
async fn open_uncertain(world: &mut BddWorld) {
    world.herdr.set_open_uncertain();
}

#[given("the global focus moves to another tab while the toggle is running")]
async fn divert_focus(world: &mut BddWorld) {
    let workspace = origin_workspace(world).to_string();
    world
        .herdr
        .ensure_working_pane(&workspace, "other-focus", "other-focus-editor");
    world.herdr.divert_focus_on_list("other-focus");
}

#[given(
    regex = r#"^the working pane "([^"]+)" already occupies only the top half of the focused tab$"#
)]
async fn half_height(world: &mut BddWorld, editor: String) {
    let workspace = origin_workspace(world).to_string();
    let tab = origin_tab(world).to_string();
    world
        .herdr
        .seed_half_height(&workspace, &tab, &editor, "bottom");
    world.origin = Some(OriginContext {
        workspace_id: WorkspaceId(workspace.clone()),
        tab_id: TabId(tab.clone()),
        pane_id: PaneId(editor.clone()),
        foreground_cwd: Some("/work/app".into()),
        cwd: Some("/work/app".into()),
    });
    world.saved_rects = world.herdr.other_pane_rects(&tab, &[&editor]);
}

#[when(regex = r#"^two "herdr-npm.toggle" invocations start at the same time$"#)]
async fn concurrent_toggles(world: &mut BddWorld) {
    let origin = world.origin.clone().expect("origin");
    world.herdr.set_list_delay(Duration::from_millis(80));
    let herdr_a = world.herdr.clone();
    let herdr_b = world.herdr.clone();
    let dir_a = world.lock_dir.clone();
    let dir_b = world.lock_dir.clone();
    let origin_a = origin.clone();
    let origin_b = origin;
    let barrier = Arc::new(Barrier::new(3));
    let b1 = barrier.clone();
    let b2 = barrier.clone();
    let t1 = thread::spawn(move || {
        b1.wait();
        let _lock = launcher_lock::acquire(&dir_a).unwrap();
        toggle_sidebar(&herdr_a, &origin_a)
    });
    let t2 = thread::spawn(move || {
        b2.wait();
        let _lock = launcher_lock::acquire(&dir_b).unwrap();
        toggle_sidebar(&herdr_b, &origin_b)
    });
    barrier.wait();
    world.concurrent = vec![t1.join().expect("t1"), t2.join().expect("t2")];
}

#[then("the recognised sidebar pane is closed")]
async fn recognised_closed(world: &mut BddWorld) {
    assert!(
        recognised(world).is_empty(),
        "recognised sidebar still present"
    );
    assert!(
        world
            .herdr
            .calls()
            .iter()
            .any(|call| call.method == "plugin.pane.close"),
        "plugin.pane.close was not called"
    );
}

#[then("the herdr-npm process has exited")]
async fn process_exited(world: &mut BddWorld) {
    if e2e::active() {
        e2e::wait_sidebar_gone();
        return;
    }
    assert!(
        !world.herdr.exited().is_empty(),
        "sidebar process was not marked exited"
    );
}

#[then(regex = r#"^the focus returns to pane "([^"]+)"$"#)]
async fn focus_returns(world: &mut BddWorld, pane: String) {
    assert_eq!(world.herdr.focused().as_deref(), Some(pane.as_str()));
}

#[then("the focus does not move")]
async fn focus_stays(world: &mut BddWorld) {
    assert_eq!(world.herdr.focused(), world.focus_before);
}

#[then("a sidebar pane recognised by token is open in the focused tab")]
async fn token_open(world: &mut BddWorld) {
    assert_eq!(recognised(world).len(), 1);
}

#[then("that sidebar pane is closed")]
async fn that_sidebar_closed(world: &mut BddWorld) {
    recognised_closed(world).await;
}

#[then(regex = r#"^a sidebar pane is opened in the tab "([^"]+)"$"#)]
async fn opened_in_tab(world: &mut BddWorld, tab: String) {
    let found = world.herdr.panes().into_iter().any(|pane| {
        pane.tab_id == tab
            && pane.tokens.get(SIDEBAR_TOKEN_KEY).map(String::as_str) == Some(SIDEBAR_TOKEN_VALUE)
    });
    assert!(found, "no recognised sidebar in tab {tab}");
}

#[then(regex = r#"^the sidebar pane of the tab "([^"]+)" is left untouched$"#)]
async fn tab_sidebar_untouched(world: &mut BddWorld, tab: String) {
    let still = world.herdr.panes().into_iter().any(|pane| {
        pane.tab_id == tab
            && pane.tokens.get(SIDEBAR_TOKEN_KEY).map(String::as_str) == Some(SIDEBAR_TOKEN_VALUE)
    });
    assert!(still, "sidebar of tab {tab} was mutated");
    assert!(
        world
            .herdr
            .calls()
            .iter()
            .all(|call| call.method != "plugin.pane.close" || !call.detail.contains(&tab)),
        "close targeted tab {tab}"
    );
}

#[then("the pane of the tab \"other\" is left untouched")]
async fn other_token_untouched(world: &mut BddWorld) {
    tab_sidebar_untouched(world, "other".into()).await;
}

#[then("a sidebar pane recognised by token is opened in the focused tab")]
async fn opened_in_focused(world: &mut BddWorld) {
    token_open(world).await;
}

#[then("the foreign pane labelled \"npm\" is left untouched")]
async fn foreign_untouched(world: &mut BddWorld) {
    let id = world.foreign_pane.as_deref().expect("foreign pane");
    let pane = world
        .herdr
        .panes()
        .into_iter()
        .find(|pane| pane.pane_id == id)
        .expect("foreign still listed");
    assert_eq!(pane.label.as_deref(), Some(SIDEBAR_LABEL));
    assert_ne!(
        pane.tokens.get(SIDEBAR_TOKEN_KEY).map(String::as_str),
        Some(SIDEBAR_TOKEN_VALUE)
    );
}

#[then("the invocations run one after the other under the launcher lock")]
async fn serialised(world: &mut BddWorld) {
    assert_eq!(
        world.herdr.max_list_inflight(),
        1,
        "pane.list overlapped; lock did not serialise"
    );
}

#[then("they open a sidebar then close it")]
async fn open_then_close(world: &mut BddWorld) {
    let opened = world
        .concurrent
        .iter()
        .filter(|item| matches!(item, Ok(ToggleOutcome::Opened(_))))
        .count();
    let closed = world
        .concurrent
        .iter()
        .filter(|item| matches!(item, Ok(ToggleOutcome::Closed(_))))
        .count();
    assert_eq!(
        (opened, closed),
        (1, 1),
        "expected one open and one close, got {:?}",
        world.concurrent
    );
}

#[then(
    regex = r#"^the focused tab never holds two panes carrying token "herdr_npm_sidebar" equal to "v1"$"#
)]
async fn never_two(world: &mut BddWorld) {
    assert!(recognised(world).len() <= 1);
}

#[then("the launcher exits with a non-zero status")]
async fn non_zero(world: &mut BddWorld) {
    let error = world.last_error.as_ref().expect("expected an error");
    assert_ne!(error.exit_code(), 0);
}

#[then("no pane is opened")]
async fn no_open(world: &mut BddWorld) {
    let after: Vec<_> = world
        .herdr
        .panes()
        .into_iter()
        .map(|pane| pane.pane_id)
        .collect();
    for id in &after {
        if !world.pane_ids_before.contains(id) {
            panic!("unexpected new pane {id}");
        }
    }
    assert!(!world.confirmed);
}

#[then("no existing pane is closed by the plugin")]
async fn no_close(world: &mut BddWorld) {
    assert!(
        world
            .herdr
            .calls()
            .iter()
            .all(|call| call.method != "plugin.pane.close"),
        "plugin.pane.close was called"
    );
}

#[then("neither recognised pane is closed")]
async fn neither_closed(world: &mut BddWorld) {
    for id in &world.recognised_ids {
        assert!(
            world.herdr.panes().iter().any(|pane| pane.pane_id == *id),
            "recognised pane {id} was closed"
        );
    }
}

#[then("no additional pane is opened")]
async fn no_additional(world: &mut BddWorld) {
    no_open(world).await;
}

#[then("the launcher does not retry the opening")]
async fn no_retry(world: &mut BddWorld) {
    let opens = world
        .herdr
        .calls()
        .iter()
        .filter(|call| call.method == "plugin.pane.open")
        .count();
    assert_eq!(opens, 1, "opening was retried");
}

#[then("the launcher reports that the layout must be inspected before another attempt")]
async fn inspect_layout(world: &mut BddWorld) {
    let error = world.last_error.as_ref().expect("error");
    let text = error.to_string();
    assert!(
        text.contains("Inspect the layout"),
        "missing inspection wording: {text}"
    );
    assert!(matches!(error, AppError::Uncertain { .. }));
}

#[then("the launcher does not announce a confirmed open or close")]
async fn not_confirmed(world: &mut BddWorld) {
    assert!(!world.confirmed);
}

#[then(regex = r#"^the sidebar is opened in tab "([^"]+)"$"#)]
async fn opened_named_tab(world: &mut BddWorld, tab: String) {
    opened_in_tab(world, tab).await;
}

#[then("no pane of the newly focused tab is mutated")]
async fn diverted_untouched(world: &mut BddWorld) {
    let mutated = world.herdr.calls().iter().any(|call| {
        (call.method == "plugin.pane.open"
            || call.method == "plugin.pane.close"
            || call.method == "pane.swap")
            && call.detail.contains("other-focus")
    });
    assert!(!mutated, "diverted tab was mutated");
    assert!(
        world
            .herdr
            .panes()
            .iter()
            .all(|pane| pane.tab_id != "other-focus" || !pane.is_herdr_npm_sidebar())
    );
}

#[then("the sidebar is not stretched to the full tab height")]
async fn not_full_height(world: &mut BddWorld) {
    let opened = world.opened_pane.as_ref().expect("sidebar");
    let layout = world
        .herdr
        .layout_for_pane(opened.as_str())
        .expect("layout");
    let sidebar = layout
        .panes
        .iter()
        .find(|pane| pane.pane_id == opened.as_str())
        .expect("sidebar rect");
    assert!(
        sidebar.rect.height < layout.area.height,
        "sidebar filled the tab height"
    );
}

#[then("the other splits of the tab keep their size")]
async fn other_splits_kept(world: &mut BddWorld) {
    let tab = origin_tab(world).to_string();
    let opened = world
        .opened_pane
        .as_ref()
        .map(|id| id.0.clone())
        .unwrap_or_default();
    let now = world.herdr.other_pane_rects(&tab, &[opened.as_str()]);
    for (id, rect) in &world.saved_rects {
        let current = now
            .iter()
            .find(|(pane, _)| pane == id)
            .map(|(_, rect)| *rect);
        assert_eq!(current.as_ref(), Some(rect), "pane {id} moved");
    }
}

#[then("no package.json file is read")]
async fn no_package_json(world: &mut BddWorld) {
    assert_eq!(world.package_json_reads, 0);
    assert!(
        world
            .herdr
            .calls()
            .iter()
            .all(|call| !call.method.contains("package")),
        "toggle touched package.json"
    );
}
