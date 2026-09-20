use std::fs;
use std::os::unix::fs::PermissionsExt;

use cucumber::{given, then, when};
use herdr_npm::domain::catalog::{PackageManager, Script};
use herdr_npm::domain::ids::PaneId;
use herdr_npm::domain::run_command::run_invocation;
use serde_json::{Map, Value, json};

use crate::support::e2e;
use crate::support::tui;
use crate::support::world::BddWorld;

fn install_fake_managers(world: &BddWorld) {
    fs::create_dir_all(&world.fake_bin).unwrap();
    let body = r#"#!/usr/bin/env python3
import json, os, sys
with open(os.environ["HERDR_NPM_ARGV"], "w") as handle:
    json.dump(sys.argv, handle)
"#;
    for name in ["npm", "pnpm"] {
        let path = world.fake_bin.join(name);
        fs::write(&path, body).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn last_argv(world: &BddWorld) -> Vec<String> {
    let bytes = fs::read(&world.argv_file)
        .unwrap_or_else(|_| panic!("missing argv file {}", world.argv_file.display()));
    serde_json::from_slice(&bytes).expect("argv json")
}

fn last_tab(world: &BddWorld) -> crate::support::fake_herdr::FakeTab {
    world
        .herdr
        .created_tabs()
        .pop()
        .expect("expected a created tab")
}

fn select_named(world: &mut BddWorld, name: &str) {
    let index = world
        .app
        .as_ref()
        .expect("sidebar")
        .scripts()
        .iter()
        .position(|script| script.name == name)
        .unwrap_or_else(|| panic!("script {name} is not listed"));
    if let Some(app) = world.app.as_mut() {
        app.select_index(index);
    }
    tui::draw(world);
}

fn add_script(world: &mut BddWorld, name: &str, command: &str) {
    let virt = world.last_pkg.clone().unwrap_or_else(|| "/work/app".into());
    let dir = world.map_path(&virt);
    let path = dir.join("package.json");
    let mut map: Map<String, Value> = if path.is_file() {
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap_or_else(|_| Map::new())
    } else {
        Map::new()
    };
    let scripts = map
        .entry("scripts".to_string())
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .expect("scripts object");
    scripts.insert(name.to_string(), json!(command));
    fs::write(
        path,
        serde_json::to_vec_pretty(&Value::Object(map)).unwrap(),
    )
    .unwrap();
    if let Some(app) = world.app.as_mut()
        && let Ok(catalog) = app.listed.catalog.as_mut()
        && !catalog.scripts.iter().any(|script| script.name == name)
    {
        catalog.scripts.push(Script {
            name: name.to_string(),
            command: command.to_string(),
        });
    }
}

#[given(regex = r#"^the sidebar is open in the workspace "([^"]+)"$"#)]
async fn open_in_workspace(world: &mut BddWorld, workspace: String) {
    if e2e::active() {
        world.workspace_id = e2e::live()
            .panes()
            .first()
            .and_then(|pane| pane.get("workspace_id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("w1")
            .to_string();
        let _ = workspace;
        e2e::open_sidebar(world);
        let pkg = e2e::fixture().join("package.json");
        assert!(
            pkg.is_file(),
            "e2e fixture package.json missing at {}",
            pkg.display()
        );
        return;
    }
    world.workspace_id = workspace.clone();
    world.auto_launch = true;
    world.tui_wanted = true;
    if world.foreground_cwd.is_none() {
        let real = world.map_path("/work/app");
        let _ = fs::create_dir_all(&real);
        world.foreground_cwd = Some(real);
    }
    world.herdr.seed_single_tab(&workspace, "main:t1", "editor");
    let sidebar = world.herdr.add_recognised_sidebar("main:t1", "editor");
    world.herdr.set_focus(&sidebar);
    world.opened_pane = Some(PaneId(sidebar));
    install_fake_managers(world);
    world
        .herdr
        .set_shell_exec(world.fake_bin.clone(), world.argv_file.clone());
    tui::open_sidebar(world);
}

#[given(regex = r#"^the resolved project root is "/([^"]+)"$"#)]
async fn given_root(world: &mut BddWorld, path: String) {
    if e2e::active() {
        let expected = e2e::fixture();
        let cwd = e2e::live().panes().into_iter().find(|pane| {
            pane.get("cwd")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|cwd| cwd.contains(&path) || cwd == expected.to_string_lossy())
        });
        assert!(
            cwd.is_some() || expected.ends_with(&path),
            "live cwd does not include /{path}"
        );
        return;
    }
    let expected = format!("/{path}");
    let root = world
        .app
        .as_ref()
        .and_then(|app| app.listed.root.clone())
        .map(|path| world.virtual_from(&path));
    assert_eq!(root.as_deref(), Some(expected.as_str()));
}

#[given(regex = r#"^the detected package manager is "([^"]+)"$"#)]
async fn given_manager(world: &mut BddWorld, manager: String) {
    if e2e::active() {
        if let Some(sidebar) = e2e::sidebar_pane() {
            let text = e2e::read_pane(&sidebar);
            assert!(
                text.contains(&manager),
                "sidebar header does not show {manager}:\n{text}"
            );
        }
        return;
    }
    let current = world
        .app
        .as_ref()
        .and_then(|app| app.catalog())
        .map(|catalog| catalog.manager.as_str().to_string())
        .unwrap_or_default();
    if current != manager {
        if let Some(app) = world.app.as_mut()
            && let Ok(catalog) = app.listed.catalog.as_mut()
        {
            catalog.manager = if manager == "pnpm" {
                PackageManager::Pnpm
            } else {
                PackageManager::Npm
            };
        }
        tui::draw(world);
    }
    let got = world
        .app
        .as_ref()
        .and_then(|app| app.catalog())
        .map(|catalog| catalog.manager.as_str())
        .unwrap_or("");
    assert_eq!(got, manager);
}

#[given(regex = r#"^the selection is on the script "([^"]+)"$"#)]
async fn given_selection(world: &mut BddWorld, name: String) {
    if e2e::active() {
        if name != "dev"
            && let Some(sidebar) = e2e::sidebar_pane()
        {
            e2e::select_script(&sidebar, &name);
        }
        return;
    }
    select_named(world, &name);
}

#[given(regex = r#"^the origin pane foreground cwd was "/([^"]+)" when the sidebar opened$"#)]
async fn reopened_from_subdir(world: &mut BddWorld, path: String) {
    let real = world.map_path(&format!("/{path}"));
    let _ = fs::create_dir_all(&real);
    world.foreground_cwd = Some(real);
    tui::open_sidebar(world);
}

#[given(regex = r#"^the package declares a script named (.+)$"#)]
async fn declare_named(world: &mut BddWorld, rest: String) {
    let (name, command) = if let Some(name) = rest.strip_suffix(" whose command is \"\"") {
        (unquote(name), String::new())
    } else {
        (unquote(&rest), "echo".into())
    };
    add_script(world, &name, &command);
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
        .to_string()
}

#[given("the sidebar is open on that catalogue")]
async fn reopen_catalogue(world: &mut BddWorld) {
    tui::open_sidebar(world);
}

#[given("the 40th script is selected")]
async fn select_fortieth(world: &mut BddWorld) {
    if let Some(app) = world.app.as_mut() {
        let last = app.scripts().len().saturating_sub(1);
        app.select_index(last);
    }
    tui::draw(world);
}

#[given("creating a tab will fail")]
async fn create_will_fail(world: &mut BddWorld) {
    world.herdr.set_create_fail();
}

#[given(regex = r#"^a tab is created with id "([^"]+)"$"#)]
async fn forced_tab_id(world: &mut BddWorld, tab_id: String) {
    world.herdr.set_forced_tab_id(&tab_id);
}

#[given("sending input to its root pane times out")]
async fn send_times_out(world: &mut BddWorld) {
    world.herdr.set_send_timeout();
}

#[given("tab creation returns no usable root pane id")]
async fn missing_root(world: &mut BddWorld) {
    world.herdr.set_missing_root();
}

#[given("the global focus moves to another workspace while the launch is running")]
async fn divert_workspace(world: &mut BddWorld) {
    world.herdr.divert_focus_on_list("other");
}

#[when(regex = r#"^I left-click the row of the script "([^"]+)"$"#)]
async fn click_row(world: &mut BddWorld, name: String) {
    let row = tui::script_row(world, &name);
    let column = tui::zone_column(world, "script name");
    tui::left_click(world, column, row);
}

#[when(regex = r#"^I run the script "(.+)"$"#)]
async fn run_named(world: &mut BddWorld, name: String) {
    select_named(world, &name);
    if let Some(app) = world.app.as_mut() {
        app.emit_run();
    }
    tui::launch_pending(world);
}

#[then(regex = r#"^(?:a|the) new tab is created in the workspace .+"#)]
async fn tab_created_in(world: &mut BddWorld) {
    if e2e::active() {
        e2e::live().wait_until(std::time::Duration::from_secs(8), |herdr| {
            herdr.tabs().len() > world.e2e_tabs_before.len().max(1)
        });
        let tabs = e2e::live().tabs();
        assert!(
            tabs.len() >= 2,
            "expected a new background tab, got {tabs:?}"
        );
        let created = tabs
            .iter()
            .find(|tab| tab.get("focused").and_then(serde_json::Value::as_bool) == Some(false))
            .or_else(|| tabs.last())
            .expect("background tab");
        assert_eq!(
            created
                .get("workspace_id")
                .and_then(serde_json::Value::as_str),
            Some(world.workspace_id.as_str())
        );
        world.e2e_script_tab = created
            .get("tab_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        return;
    }
    let tab = last_tab(world);
    assert_eq!(tab.workspace_id, world.workspace_id);
    assert!(!tab.focus);
}

#[then(regex = r#"^the package manager "([^"]+)" is invoked in "/([^"]+)"$"#)]
async fn manager_invoked(world: &mut BddWorld, manager: String, path: String) {
    let tab = last_tab(world);
    let expected = format!("/{path}");
    assert_eq!(world.virtual_from(&tab.cwd), expected);
    assert!(
        tab.label.starts_with(&format!("{manager} run --")),
        "label {} should start with {manager} run --",
        tab.label
    );
    let argv = last_argv(world);
    assert!(
        argv.first().is_some_and(|bin| bin.ends_with(&manager)),
        "argv {argv:?} should invoke {manager}"
    );
}

#[then(regex = r#"^the package manager "([^"]+)" receives a single script argument "(.+)"$"#)]
async fn receives_argument_named(world: &mut BddWorld, _manager: String, script: String) {
    assert_single_script_arg(world, &script);
}

#[then(regex = r#"^the package manager receives a single script argument "(.+)"$"#)]
async fn receives_argument(world: &mut BddWorld, script: String) {
    assert_single_script_arg(world, &script);
}

fn assert_single_script_arg(world: &BddWorld, script: &str) {
    let argv = if e2e::active() {
        let path = world.argv_file.clone();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        loop {
            if std::fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Vec<String>>(&bytes).ok())
                .and_then(|argv| argv.get(3).cloned())
                .as_deref()
                == Some(script)
            {
                break;
            }
            if std::time::Instant::now() > deadline {
                let raw = std::fs::read_to_string(&path).unwrap_or_default();
                let tabs = e2e::live().stdout(&["tab", "list"]);
                panic!(
                    "timed out waiting for argv[3]={script:?}\nargv_file={} raw={raw:?}\ntabs={tabs}",
                    path.display()
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(80));
        }
        e2e::last_argv()
    } else {
        last_argv(world)
    };
    assert_eq!(
        argv.get(3).map(String::as_str),
        Some(script),
        "expected a single script argument {script}, argv={argv:?}"
    );
    assert_eq!(
        argv.len(),
        4,
        "manager argv should be [bin, run, --, name], got {argv:?}"
    );
}

#[then(regex = r#"^the working directory of the new tab is "/([^"]+)"$"#)]
async fn tab_cwd(world: &mut BddWorld, path: String) {
    let expected = format!("/{path}");
    assert_eq!(world.virtual_from(&last_tab(world).cwd), expected);
}

#[then(regex = r#"^the selection moves to the script "([^"]+)"$"#)]
async fn then_selection_moved(world: &mut BddWorld, name: String) {
    let selected = world
        .app
        .as_ref()
        .and_then(|app| app.selected_script())
        .map(|script| script.name.as_str())
        .unwrap_or("");
    assert_eq!(selected, name);
}

#[then(regex = r#"^exactly (\d+) tab has been created in the workspace "([^"]+)"$"#)]
async fn exactly_n_tabs(world: &mut BddWorld, count: usize, workspace: String) {
    let tabs: Vec<_> = world
        .herdr
        .created_tabs()
        .into_iter()
        .filter(|tab| tab.workspace_id == workspace)
        .collect();
    assert_eq!(tabs.len(), count);
}

#[then("no tab is created")]
async fn no_tab(world: &mut BddWorld) {
    assert!(world.herdr.created_tabs().is_empty());
}

#[then("the new tab does not take the focus")]
async fn tab_no_focus(world: &mut BddWorld) {
    assert!(!last_tab(world).focus);
}

#[then("the sidebar pane is still focused")]
async fn sidebar_still_focused(world: &mut BddWorld) {
    let opened = world.opened_pane.as_ref().expect("sidebar id");
    assert_eq!(world.herdr.focused().as_deref(), Some(opened.as_str()));
}

#[then(regex = r#"^the sidebar still lists (\d+) scripts$"#)]
async fn still_lists(world: &mut BddWorld, count: usize) {
    let n = world
        .app
        .as_ref()
        .map(|app| app.scripts().len())
        .unwrap_or(0);
    assert_eq!(n, count);
}

#[then(regex = r#"^(\d+) tabs have been created in the workspace "([^"]+)"$"#)]
async fn n_tabs(world: &mut BddWorld, count: usize, workspace: String) {
    exactly_n_tabs(world, count, workspace).await;
}

#[then(regex = r#"^(\d+) tabs are running script "([^"]+)" with "([^"]+)"$"#)]
async fn tabs_running(world: &mut BddWorld, count: usize, script: String, manager: String) {
    let expected = run_invocation(
        if manager == "pnpm" {
            PackageManager::Pnpm
        } else {
            PackageManager::Npm
        },
        &script,
    );
    let tabs: Vec<_> = world
        .herdr
        .created_tabs()
        .into_iter()
        .filter(|tab| tab.label == expected)
        .collect();
    assert_eq!(tabs.len(), count, "tabs={:?}", world.herdr.created_tabs());
}

#[then("no existing tab was reused")]
async fn no_reuse(world: &mut BddWorld) {
    let tabs = world.herdr.created_tabs();
    let ids: std::collections::BTreeSet<_> = tabs.iter().map(|tab| tab.tab_id.clone()).collect();
    assert_eq!(ids.len(), tabs.len());
}

#[then(regex = r#"^the new tab is labelled with the invoked command for script "([^"]+)"$"#)]
async fn labelled_command(world: &mut BddWorld, script: String) {
    let manager = world
        .app
        .as_ref()
        .and_then(|app| app.catalog())
        .map(|catalog| catalog.manager)
        .unwrap_or(PackageManager::Npm);
    assert_eq!(last_tab(world).label, run_invocation(manager, &script));
}

#[then("that argument is the literal script name, including spaces and metacharacters")]
async fn literal_argument(world: &mut BddWorld) {
    let argv = last_argv(world);
    assert_eq!(argv.len(), 4);
    assert_eq!(argv[2], "--");
}

#[then("the package manager does not treat the script name as one of its own options")]
async fn not_an_option(world: &mut BddWorld) {
    let argv = last_argv(world);
    assert_eq!(argv.get(1).map(String::as_str), Some("run"));
    assert_eq!(argv.get(2).map(String::as_str), Some("--"));
}

#[then("no pane input is sent")]
async fn no_input(world: &mut BddWorld) {
    assert!(
        world
            .herdr
            .calls()
            .iter()
            .all(|call| call.method != "pane.send_input"),
        "unexpected send: {:?}",
        world.herdr.calls()
    );
}

#[then("no confirmed launch is announced")]
async fn no_confirmed(world: &mut BddWorld) {
    assert!(world.herdr.created_tabs().is_empty() || world.last_error.is_some());
    assert!(world.last_error.is_some());
}

#[then("the catalogue is unchanged")]
async fn catalogue_unchanged(world: &mut BddWorld) {
    let names: Vec<_> = world
        .app
        .as_ref()
        .map(|app| {
            app.scripts()
                .iter()
                .map(|script| script.name.clone())
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(names, world.names_at_open);
}

#[then(
    regex = r#"^the sidebar shows the message "Script launch not confirmed" including tab id "([^"]+)"$"#
)]
async fn launch_not_confirmed_with_id(world: &mut BddWorld, tab_id: String) {
    let message = world
        .app
        .as_ref()
        .and_then(|app| app.error_message())
        .or_else(|| world.last_error.as_ref().map(ToString::to_string))
        .unwrap_or_default();
    assert!(
        message.contains("Script launch not confirmed") && message.contains(&tab_id),
        "message={message}"
    );
}

#[then("the launcher does not retry the send")]
async fn no_retry(world: &mut BddWorld) {
    let sends = world
        .herdr
        .calls()
        .into_iter()
        .filter(|call| call.method == "pane.send_input")
        .count();
    assert_eq!(sends, 1);
}

#[then("the catalogue is still usable")]
async fn still_usable(world: &mut BddWorld) {
    let app = world.app.as_ref().expect("sidebar");
    assert!(!app.scripts().is_empty());
    assert!(app.process_running);
}
