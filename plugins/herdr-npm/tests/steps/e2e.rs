use std::time::Duration;

use cucumber::{given, then, when};
use serde_json::Value;

use crate::support::e2e;
use crate::support::live::{is_explorer, is_npm_sidebar, pane_id, tab_id};
use crate::support::world::BddWorld;

#[given("the herdr-sidebar explorer already occupies the left edge of the focused tab")]
async fn explorer_on_left(world: &mut BddWorld) {
    e2e::open_explorer(world);
    let explorer = world.e2e_explorer_pane.as_deref().expect("explorer");
    let layout = e2e::layout_for(explorer);
    let panes = layout
        .pointer("/layout/panes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let explorer_rect = panes
        .iter()
        .find(|pane| pane.get("pane_id").and_then(Value::as_str) == Some(explorer))
        .and_then(|pane| pane.get("rect"))
        .expect("explorer rect");
    let x = explorer_rect
        .get("x")
        .and_then(Value::as_u64)
        .unwrap_or(999);
    let min_x = panes
        .iter()
        .filter_map(|pane| pane.get("rect")?.get("x")?.as_u64())
        .min()
        .unwrap_or(0);
    assert_eq!(
        x, min_x,
        "explorer is not the left edge (min x={min_x}): {explorer_rect}"
    );
}

#[given("the focused tab has no herdr-npm sidebar")]
async fn no_npm_sidebar(_world: &mut BddWorld) {
    assert!(
        e2e::sidebar_pane().is_none(),
        "herdr-npm sidebar already present"
    );
}

#[then("the herdr-npm sidebar is docked against the explorer, on the centre side")]
async fn docked_against_explorer(world: &mut BddWorld) {
    let sidebar = e2e::sidebar_pane().expect("npm sidebar");
    let explorer = world.e2e_explorer_pane.as_deref().expect("explorer");
    let layout = e2e::layout_for(&sidebar);
    let panes = layout
        .pointer("/layout/panes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let rect = |id: &str| {
        panes
            .iter()
            .find(|pane| pane.get("pane_id").and_then(Value::as_str) == Some(id))
            .and_then(|pane| pane.get("rect"))
            .cloned()
            .unwrap_or_else(|| panic!("missing rect for {id} in {panes:?}"))
    };
    let explorer_rect = rect(explorer);
    let sidebar_rect = rect(&sidebar);
    let ex = explorer_rect.get("x").and_then(Value::as_u64).unwrap_or(0);
    let ew = explorer_rect
        .get("width")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let sx = sidebar_rect.get("x").and_then(Value::as_u64).unwrap_or(0);
    assert!(
        sx >= ex + ew.saturating_sub(1),
        "sidebar should sit on the centre side of the explorer: explorer={explorer_rect} sidebar={sidebar_rect}"
    );
}

#[then("the explorer pane is not split")]
async fn explorer_not_split(world: &mut BddWorld) {
    let explorer = world.e2e_explorer_pane.as_deref().expect("explorer");
    let still = e2e::live()
        .panes()
        .iter()
        .any(|pane| pane_id(pane) == explorer && is_explorer(pane));
    assert!(still, "explorer pane disappeared");
}

#[then("the working-pane target is the pane immediately to the right of the explorer")]
async fn working_right_of_explorer(world: &mut BddWorld) {
    let explorer = world.e2e_explorer_pane.as_deref().expect("explorer");
    let working = e2e::working_pane(world);
    let layout = e2e::layout_for(&working);
    let panes = layout
        .pointer("/layout/panes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let x_of = |id: &str| {
        panes
            .iter()
            .find(|pane| pane.get("pane_id").and_then(Value::as_str) == Some(id))
            .and_then(|pane| pane.get("rect"))
            .and_then(|rect| rect.get("x"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    assert!(
        x_of(&working) > x_of(explorer),
        "working pane is not to the right of the explorer"
    );
}

#[given("the sidebar is open and focused")]
async fn sidebar_open_focused(world: &mut BddWorld) {
    e2e::open_sidebar(world);
    let sidebar = e2e::sidebar_pane().expect("sidebar");
    let focused = e2e::live()
        .panes()
        .iter()
        .find(|pane| pane_id(pane) == sidebar)
        .and_then(|pane| pane.get("focused"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !focused {
        let _ = e2e::live().output(&["plugin", "pane", "focus", &sidebar]);
    }
    world.e2e_sidebar_pane = Some(sidebar);
}

#[when(regex = r#"^I press "q" in the sidebar$"#)]
async fn press_q_in_sidebar(world: &mut BddWorld) {
    let sidebar = world
        .e2e_sidebar_pane
        .clone()
        .or_else(e2e::sidebar_pane)
        .expect("sidebar");
    e2e::send_keys(&sidebar, &["q"]);
}

#[then("the sidebar pane is closed")]
async fn sidebar_closed(_world: &mut BddWorld) {
    e2e::wait_sidebar_gone();
}

#[given("a herdr-npm sidebar was open")]
async fn sidebar_was_open(world: &mut BddWorld) {
    e2e::open_sidebar(world);
    world.e2e_restored_pane = e2e::sidebar_pane();
}

#[when("the Herdr server is restarted")]
async fn server_restarted(_world: &mut BddWorld) {
    e2e::restart_server();
}

#[then(regex = r#"^the restored pane no longer carries token "herdr_npm_sidebar" equal to "v1"$"#)]
async fn restored_inert(world: &mut BddWorld) {
    let restored = world.e2e_restored_pane.clone().expect("restored pane id");
    e2e::live().wait_until(Duration::from_secs(8), |herdr| {
        herdr.panes().iter().any(|pane| pane_id(pane) == restored)
    });
    let pane = e2e::live()
        .panes()
        .into_iter()
        .find(|pane| pane_id(pane) == restored)
        .expect("restored pane");
    assert!(
        !is_npm_sidebar(&pane),
        "restored pane still carries the live token: {pane}"
    );
}

#[when("I close that restored pane with Herdr")]
async fn close_restored(world: &mut BddWorld) {
    let restored = world.e2e_restored_pane.clone().expect("restored pane");
    e2e::live().json(&["pane", "close", &restored]);
}

#[then("a new sidebar pane recognised by token is opened")]
async fn new_recognised(world: &mut BddWorld) {
    e2e::live().wait_until(Duration::from_secs(8), |_| e2e::sidebar_pane().is_some());
    let sidebar = e2e::sidebar_pane().expect("new sidebar");
    if let Some(old) = world.e2e_restored_pane.as_deref() {
        assert_ne!(sidebar, old, "plugin reused the inert restored pane");
    }
    world.e2e_sidebar_pane = Some(sidebar);
}

#[then("the plugin does not replace the restored pane automatically")]
async fn no_auto_replace(world: &mut BddWorld) {
    let old = world.e2e_restored_pane.as_deref().expect("old pane");
    let still = e2e::live().panes().iter().any(|pane| pane_id(pane) == old);
    assert!(
        !still,
        "restored pane {old} is still present after the documented manual close"
    );
}

#[given(regex = r#"^I ran the script "([^"]+)" in a new tab$"#)]
async fn ran_script(world: &mut BddWorld, name: String) {
    e2e::run_script(world, &name);
}

#[when("I close that tab")]
async fn close_script_tab(world: &mut BddWorld) {
    let tab = world.e2e_script_tab.clone().expect("script tab");
    e2e::live().json(&["tab", "close", &tab]);
}

#[then("the process of that ordinary recipe script is terminated")]
async fn script_terminated(world: &mut BddWorld) {
    let pid = world
        .e2e_script_pid
        .expect("recipe script PID must be known before closing its tab");
    e2e::live().wait_until(Duration::from_secs(8), |_| !e2e::pid_alive(pid));
    assert!(!e2e::pid_alive(pid), "pid {pid} is still alive");
    let tab = world.e2e_script_tab.clone().expect("script tab");
    e2e::live().wait_until(Duration::from_secs(8), |herdr| {
        !herdr.tabs().iter().any(|item| tab_id(item) == tab)
    });
}

#[then("the sidebar is still open")]
async fn sidebar_still_open(_world: &mut BddWorld) {
    assert!(e2e::sidebar_pane().is_some(), "sidebar closed unexpectedly");
}

#[when("the script exits")]
async fn wait_script_exit(world: &mut BddWorld) {
    let pane = world.e2e_script_pane.clone().expect("script pane");
    e2e::live().wait_until(Duration::from_secs(15), |_| {
        let text = e2e::read_pane(&pane);
        text.contains("BUILD_DONE") || text.contains("VITE_BUILD_OK")
    });
}

#[then("the tab is still open")]
async fn tab_still_open(world: &mut BddWorld) {
    let tab = world.e2e_script_tab.clone().expect("script tab");
    assert!(
        e2e::live().tabs().iter().any(|item| tab_id(item) == tab),
        "script tab {tab} disappeared"
    );
}

#[then("its output is still readable")]
async fn output_readable(world: &mut BddWorld) {
    let pane = world.e2e_script_pane.clone().expect("script pane");
    let text = e2e::read_pane(&pane);
    assert!(
        text.contains("BUILD_DONE") || text.contains("VITE_BUILD_OK"),
        "script output missing in:\n{text}"
    );
}

#[when(regex = r#"^I focus that tab and press "q"$"#)]
async fn focus_tab_press_q(world: &mut BddWorld) {
    let tab = world.e2e_script_tab.clone().expect("script tab");
    e2e::live().json(&["tab", "focus", &tab]);
    let pane = world.e2e_script_pane.clone().expect("script pane");
    e2e::live().wait_until(Duration::from_secs(8), |_| {
        e2e::read_pane(&pane).contains("VITE_HOLD_START")
    });
    e2e::send_keys(&pane, &["q"]);
}

#[then(regex = r#"^"q" is sent to the running script$"#)]
async fn q_reached_script(_world: &mut BddWorld) {
    e2e::live().wait_until(Duration::from_secs(5), |_| {
        std::fs::read_to_string(e2e::hold_log())
            .unwrap_or_default()
            .contains('q')
    });
    let log = std::fs::read_to_string(e2e::hold_log()).unwrap_or_default();
    assert!(log.contains('q'), "hold log has no q: {log:?}");
}

#[when(
    regex = r#"^I send the keys "j" then "Enter" to the sidebar pane with "herdr pane send-keys"$"#
)]
async fn send_j_enter(world: &mut BddWorld) {
    let mut sidebar = world.e2e_sidebar_pane.clone().or_else(e2e::sidebar_pane);
    if sidebar
        .as_ref()
        .is_none_or(|pane| !e2e::catalog_ready(pane))
    {
        e2e::open_sidebar(world);
        sidebar = e2e::sidebar_pane();
    }
    let sidebar = sidebar.expect("sidebar");
    e2e::live().wait_until(Duration::from_secs(8), |_| e2e::catalog_ready(&sidebar));
    world.e2e_tabs_before = e2e::live()
        .tabs()
        .iter()
        .map(|tab| tab_id(tab).to_string())
        .collect();
    let _ = std::fs::remove_file(std::env::var("HERDR_NPM_E2E_ARGV").unwrap_or_default());
    e2e::send_keys(&sidebar, &["j"]);
    let moved = std::time::Instant::now();
    loop {
        let footer = e2e::footer_command(&sidebar);
        if footer.contains("tsc") && footer.contains("vite build") {
            break;
        }
        if moved.elapsed() > Duration::from_secs(4) {
            panic!(
                "j did not select build; footer={footer:?}\n{}",
                e2e::read_pane(&sidebar)
            );
        }
        std::thread::sleep(Duration::from_millis(80));
    }
    e2e::send_keys(&sidebar, &["Enter"]);
}
