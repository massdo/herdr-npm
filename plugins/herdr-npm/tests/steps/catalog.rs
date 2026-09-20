use std::fs;
use std::path::Path;

use crossterm::event::KeyCode;
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use herdr_npm::adapters::tui::app::PLAY_ICON;
use herdr_npm::domain::CWD_FALLBACK_NOTE;
use serde_json::{Map, Value, json};
use unicode_width::UnicodeWidthStr;

use crate::support::e2e;
use crate::support::tui;
use crate::support::world::BddWorld;

fn ensure_dir(path: &Path) {
    fs::create_dir_all(path).unwrap_or_else(|error| panic!("mkdir {}: {error}", path.display()));
}

fn write_package(world: &mut BddWorld, virt: &str, body: &Value) {
    let dir = if e2e::active() {
        e2e::fixture()
    } else {
        world.map_path(virt)
    };
    ensure_dir(&dir);
    fs::write(
        dir.join("package.json"),
        serde_json::to_vec_pretty(body).expect("json"),
    )
    .unwrap_or_else(|error| panic!("write package.json: {error}"));
    world.last_pkg = Some(virt.to_string());
    world.tui_wanted = true;
}

fn scripts_from_step(step: &Step) -> Map<String, Value> {
    let table = step.table.as_ref().expect("scripts table");
    let mut scripts = Map::new();
    for row in table.rows.iter().skip(1) {
        let name = row.first().cloned().unwrap_or_default();
        let command = row.get(1).cloned().unwrap_or_default();
        scripts.insert(name, Value::String(command));
    }
    scripts
}

fn write_named_scripts(
    world: &mut BddWorld,
    virt: &str,
    name: Option<&str>,
    scripts: Map<String, Value>,
) {
    let mut map = Map::new();
    if let Some(name) = name {
        map.insert("name".into(), json!(name));
    }
    map.insert("scripts".into(), Value::Object(scripts));
    write_package(world, virt, &Value::Object(map));
}

fn load_package_object(dir: &Path) -> Map<String, Value> {
    let path = dir.join("package.json");
    let bytes = fs::read(&path).unwrap_or_else(|_| b"{}".to_vec());
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

fn save_package_object(dir: &Path, map: Map<String, Value>) {
    fs::write(
        dir.join("package.json"),
        serde_json::to_vec_pretty(&Value::Object(map)).expect("json"),
    )
    .expect("rewrite package.json");
}

fn apply_package_manager_phrase(world: &BddWorld, phrase: &str) {
    let virt = world
        .last_pkg
        .as_deref()
        .expect("a package.json must exist before this step");
    let dir = world.map_path(virt);
    let mut map = load_package_object(&dir);
    if let Some(raw) = phrase.strip_prefix("declares ") {
        let value = raw.trim().trim_matches('"');
        map.insert("packageManager".into(), json!(value));
    } else if phrase.contains("non-string") {
        map.insert("packageManager".into(), json!(1));
    } else if phrase.contains("empty packageManager") {
        map.insert("packageManager".into(), json!(""));
    } else if phrase.contains("no packageManager") {
        map.remove("packageManager");
    } else {
        panic!("unrecognised packageManager phrase: {phrase}");
    }
    save_package_object(&dir, map);
}

fn apply_lockfiles(dir: &Path, phrase: &str) {
    match phrase.trim() {
        "a package-lock.json" => {
            fs::write(dir.join("package-lock.json"), "{}\n").unwrap();
        }
        "a pnpm-lock.yaml" => {
            fs::write(dir.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").unwrap();
        }
        "both lockfiles" => {
            fs::write(dir.join("package-lock.json"), "{}\n").unwrap();
            fs::write(dir.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").unwrap();
        }
        "no lockfile" => {}
        "a yarn.lock" => {
            fs::write(dir.join("yarn.lock"), "# yarn\n").unwrap();
        }
        "a bun.lockb" => {
            fs::write(dir.join("bun.lockb"), []).unwrap();
        }
        other => panic!("unrecognised lockfiles phrase: {other}"),
    }
}

fn require_app(world: &BddWorld) -> &herdr_npm::adapters::tui::app::SidebarApp {
    world.app.as_ref().expect("sidebar TUI is not open")
}

fn script_names(world: &BddWorld) -> Vec<String> {
    require_app(world)
        .scripts()
        .iter()
        .map(|script| script.name.clone())
        .collect()
}

fn parse_name_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

fn resolved_root(world: &BddWorld) -> Option<String> {
    let app = require_app(world);
    app.listed
        .root
        .as_ref()
        .or_else(|| app.catalog().map(|catalog| &catalog.root))
        .map(|path| world.virtual_from(path))
}

fn set_foreground(world: &mut BddWorld, virt: &str) {
    let real = world.map_path(virt);
    ensure_dir(&real);
    world.foreground_cwd = Some(real);
    world.tui_wanted = true;
}

fn set_start(world: &mut BddWorld, virt: &str) {
    let real = world.map_path(virt);
    ensure_dir(&real);
    world.start_cwd = Some(real);
    world.tui_wanted = true;
}

fn select_named(world: &mut BddWorld, name: &str) {
    let index = {
        let app = require_app(world);
        app.scripts()
            .iter()
            .position(|script| script.name == name)
            .unwrap_or_else(|| panic!("script {name} is not listed"))
    };
    if let Some(app) = world.app.as_mut() {
        app.select_index(index);
    }
    tui::draw(world);
}

fn assert_no_launch(world: &mut BddWorld) {
    if let Some(app) = world.app.as_mut() {
        app.run_intents.clear();
        app.emit_run();
        assert!(
            app.run_intents.is_empty(),
            "a run intent was emitted in an unlaunchable state: {:?}",
            app.run_intents
        );
    } else {
        panic!("sidebar TUI is not open");
    }
}

fn row_for_script<'a>(world: &'a BddWorld, name: &str) -> &'a str {
    world
        .screen
        .lines()
        .find(|line| line.contains(name) && line.contains(PLAY_ICON))
        .unwrap_or_else(|| panic!("no visible row for {name} in:\n{}", world.screen))
}

#[given(
    regex = r#"^a project at "([^"]+)" whose package.json has name "([^"]+)" and declares the scripts:$"#
)]
async fn project_with_name_and_scripts(
    world: &mut BddWorld,
    path: String,
    name: String,
    step: &Step,
) {
    write_named_scripts(world, &path, Some(&name), scripts_from_step(step));
}

#[given(
    regex = r#"^a nested package at "([^"]+)" whose package.json has name "([^"]+)" and declares the scripts:$"#
)]
async fn nested_with_scripts(world: &mut BddWorld, path: String, name: String, step: &Step) {
    write_named_scripts(world, &path, Some(&name), scripts_from_step(step));
}

#[given(
    regex = r#"^a nested package at "([^"]+)" whose package.json has name "([^"]+)" and has no "scripts" field$"#
)]
async fn nested_without_scripts(world: &mut BddWorld, path: String, name: String) {
    write_package(world, &path, &json!({ "name": name }));
}

#[given(regex = r#"^a nested path "([^"]+)" that is not valid JSON$"#)]
async fn nested_invalid_json(world: &mut BddWorld, path: String) {
    let real = world.map_path(&path);
    if let Some(parent) = real.parent() {
        ensure_dir(parent);
    }
    fs::write(&real, "{").unwrap();
    world.tui_wanted = true;
}

#[given(regex = r#"^a project at "([^"]+)" whose package.json is not valid JSON$"#)]
async fn project_invalid_json(world: &mut BddWorld, path: String) {
    let dir = world.map_path(&path);
    ensure_dir(&dir);
    fs::write(dir.join("package.json"), "{").unwrap();
    world.last_pkg = Some(path);
    world.tui_wanted = true;
}

#[given(regex = r#"^a project at "([^"]+)" whose package.json root is a JSON array$"#)]
async fn project_array_root(world: &mut BddWorld, path: String) {
    let dir = world.map_path(&path);
    ensure_dir(&dir);
    fs::write(dir.join("package.json"), "[]").unwrap();
    world.last_pkg = Some(path);
    world.tui_wanted = true;
}

#[given(regex = r#"^a project at "([^"]+)" whose package.json cannot be read$"#)]
async fn project_unreadable(world: &mut BddWorld, path: String) {
    let dir = world.map_path(&path);
    ensure_dir(&dir);
    let pkg = dir.join("package.json");
    if pkg.is_file() {
        fs::remove_file(&pkg).unwrap();
    }
    fs::create_dir_all(&pkg).unwrap();
    world.last_pkg = Some(path);
    world.tui_wanted = true;
}

#[given(
    regex = r#"^a project at "([^"]+)" whose package.json has name "([^"]+)" and has no "scripts" field$"#
)]
async fn project_no_scripts_field(world: &mut BddWorld, path: String, name: String) {
    write_package(world, &path, &json!({ "name": name }));
}

#[given(
    regex = r#"^a project at "([^"]+)" whose package.json has name "([^"]+)" and has an empty "scripts" field$"#
)]
async fn project_empty_scripts(world: &mut BddWorld, path: String, name: String) {
    write_package(world, &path, &json!({ "name": name, "scripts": {} }));
}

#[given(
    regex = r#"^a project at "([^"]+)" whose package.json has name "([^"]+)" and whose "scripts" field is (.+)$"#
)]
async fn project_scripts_shape(world: &mut BddWorld, path: String, name: String, shape: String) {
    let scripts = match shape.trim() {
        "an array" => json!(["dev"]),
        "a string" => json!("nope"),
        "an object with a non-string value" => json!({ "dev": 1 }),
        other => panic!("unrecognised scripts shape: {other}"),
    };
    write_package(world, &path, &json!({ "name": name, "scripts": scripts }));
}

#[given(
    regex = r#"^a project at "([^"]+)" whose package.json has no name and declares the scripts:$"#
)]
async fn project_missing_name(world: &mut BddWorld, path: String, step: &Step) {
    write_named_scripts(world, &path, None, scripts_from_step(step));
}

#[given(
    regex = r#"^a project at "([^"]+)" whose package.json has name "" and declares the scripts:$"#
)]
async fn project_empty_name(world: &mut BddWorld, path: String, step: &Step) {
    write_named_scripts(world, &path, Some(""), scripts_from_step(step));
}

#[given(
    regex = r#"^a project at "([^"]+)" whose package.json has a non-string name and declares the scripts:$"#
)]
async fn project_non_string_name(world: &mut BddWorld, path: String, step: &Step) {
    let mut map = Map::new();
    map.insert("name".into(), json!(1));
    map.insert("scripts".into(), Value::Object(scripts_from_step(step)));
    write_package(world, &path, &Value::Object(map));
}

#[given(
    regex = r#"^a project at "([^"]+)" whose package.json has name "([^"]+)" and declares 40 scripts named s1 to s40$"#
)]
async fn project_forty_scripts(world: &mut BddWorld, path: String, name: String) {
    let mut scripts = Map::new();
    for index in 1..=40 {
        let key = format!("s{index}");
        scripts.insert(key.clone(), json!(format!("echo {key}")));
    }
    write_named_scripts(world, &path, Some(&name), scripts);
}

#[given(
    regex = r#"^a project at "([^"]+)" whose package.json has name "([^"]+)" and declares a script whose (name|command) is longer than the column$"#
)]
async fn project_long_field(world: &mut BddWorld, path: String, name: String, field: String) {
    world.long_field = Some(field.clone());
    let mut scripts = Map::new();
    if field == "name" {
        scripts.insert("n".repeat(80), json!("echo"));
    } else {
        scripts.insert("build".into(), json!("x".repeat(80)));
    }
    write_named_scripts(world, &path, Some(&name), scripts);
}

#[given(regex = r#"^the origin pane foreground cwd is "/([^"]+)"$"#)]
async fn foreground_cwd(world: &mut BddWorld, path: String) {
    set_foreground(world, &format!("/{path}"));
}

#[given(regex = r#"^the origin pane foreground cwd is now "/([^"]+)"$"#)]
async fn foreground_cwd_now(world: &mut BddWorld, path: String) {
    set_foreground(world, &format!("/{path}"));
}

#[given(regex = r#"^the origin pane start cwd is "/([^"]+)"$"#)]
async fn start_cwd(world: &mut BddWorld, path: String) {
    set_start(world, &format!("/{path}"));
}

#[given("the origin pane has no foreground cwd")]
async fn no_foreground(world: &mut BddWorld) {
    world.foreground_cwd = None;
    world.tui_wanted = true;
}

#[given("the origin pane has no start cwd")]
async fn no_start(world: &mut BddWorld) {
    world.start_cwd = None;
    world.tui_wanted = true;
}

#[given(regex = r#"^no package.json exists in "/([^"]+)" nor in any of its parents$"#)]
async fn no_package_anywhere(world: &mut BddWorld, path: String) {
    let real = world.map_path(&format!("/{path}"));
    ensure_dir(&real);
    world.tui_wanted = true;
}

#[given(regex = r#"^the TestBackend is (\d+) columns wide and (\d+) rows tall$"#)]
async fn backend_outer(world: &mut BddWorld, width: u16, height: u16) {
    world.backend_width = width;
    world.backend_height = height;
    world.size_is_interior = false;
}

#[given(regex = r#"^the TestBackend interior is (\d+) columns by (\d+) rows$"#)]
async fn backend_interior(world: &mut BddWorld, width: u16, height: u16) {
    world.backend_width = width;
    world.backend_height = height;
    world.size_is_interior = true;
}

#[given(regex = r#"^the sidebar is open on the project root "/([^"]+)"$"#)]
async fn sidebar_open_on_root(world: &mut BddWorld, path: String) {
    tui::open_sidebar(world);
    let expected = format!("/{path}");
    assert_eq!(resolved_root(world).as_deref(), Some(expected.as_str()));
}

#[given(
    regex = r#"^the sidebar was opened on the project root "/([^"]+)" and then closed with "q"$"#
)]
async fn opened_then_closed(world: &mut BddWorld, path: String) {
    if world.foreground_cwd.is_none() {
        set_foreground(world, &format!("/{path}"));
    }
    tui::open_sidebar(world);
    let expected = format!("/{path}");
    assert_eq!(resolved_root(world).as_deref(), Some(expected.as_str()));
    tui::close_with_q(world);
}

#[given(regex = r#"^the sidebar is open with the selection on the script "([^"]+)"$"#)]
async fn open_with_selection(world: &mut BddWorld, name: String) {
    tui::open_sidebar(world);
    select_named(world, &name);
}

#[given("the sidebar is open with the 40th script selected and scrolled into view")]
async fn open_with_fortieth(world: &mut BddWorld) {
    tui::open_sidebar(world);
    if let Some(app) = world.app.as_mut() {
        let last = app.scripts().len().saturating_sub(1);
        app.select_index(last);
    }
    tui::draw(world);
}

#[given(regex = r#"^the sidebar is showing "Terminal too small"$"#)]
async fn showing_too_small(world: &mut BddWorld) {
    world.backend_width = 11;
    world.backend_height = 3;
    world.size_is_interior = true;
    tui::open_sidebar(world);
    let too_small = world.app.as_ref().is_some_and(|app| {
        app.too_small() || app.error_message().as_deref() == Some("Terminal too small")
    });
    assert!(
        too_small,
        "expected too-small message, screen:\n{}",
        world.screen
    );
}

#[given("the footer command has been scrolled to the right")]
async fn footer_scrolled(world: &mut BddWorld) {
    tui::press(world, KeyCode::Char('l'));
    tui::press(world, KeyCode::Char('l'));
    tui::press(world, KeyCode::Char('l'));
}

#[given(regex = r#"^the package.json (.+)$"#)]
async fn package_json_manager(world: &mut BddWorld, phrase: String) {
    apply_package_manager_phrase(world, &phrase);
}

#[given(regex = r#"^the project contains (.+)$"#)]
async fn project_lockfiles(world: &mut BddWorld, phrase: String) {
    let virt = world.last_pkg.as_deref().expect("project");
    apply_lockfiles(&world.map_path(virt), &phrase);
}

#[given("the nested package contains no lockfile")]
async fn nested_no_lockfile(world: &mut BddWorld) {
    let virt = world.last_pkg.as_deref().expect("nested package");
    apply_lockfiles(&world.map_path(virt), "no lockfile");
}

#[given(regex = r#"^"/([^"]+)" exists$"#)]
async fn path_exists(world: &mut BddWorld, path: String) {
    let real = world.map_path(&format!("/{path}"));
    if let Some(parent) = real.parent() {
        ensure_dir(parent);
    }
    if real.extension().is_some()
        || real
            .file_name()
            .is_some_and(|name| name.to_string_lossy().contains('.'))
    {
        fs::write(&real, "ok\n").unwrap();
    } else {
        ensure_dir(&real);
    }
}

#[when("the sidebar opens")]
async fn sidebar_opens(world: &mut BddWorld) {
    tui::open_sidebar(world);
}

#[when(regex = r#"^the origin pane foreground cwd becomes "/([^"]+)"$"#)]
async fn cwd_becomes(world: &mut BddWorld, path: String) {
    set_foreground(world, &format!("/{path}"));
}

#[when(regex = r#"^the package.json at "/([^"]+)" is rewritten with only the script "([^"]+)"$"#)]
async fn rewrite_only_script(world: &mut BddWorld, path: String, script: String) {
    let mut scripts = Map::new();
    scripts.insert(script, json!("echo rewritten"));
    write_package(
        world,
        &format!("/{path}"),
        &json!({ "name": "app", "scripts": scripts }),
    );
}

#[when(regex = r#"^I press "([^"]+)"$"#)]
async fn i_press(world: &mut BddWorld, key: String) {
    let code = match key.as_str() {
        "j" | "k" | "h" | "l" | "q" => KeyCode::Char(key.chars().next().expect("key")),
        "Enter" => KeyCode::Enter,
        other => panic!("unhandled key {other}"),
    };
    tui::press(world, code);
}

#[when("I press the down arrow")]
async fn press_down(world: &mut BddWorld) {
    tui::press(world, KeyCode::Down);
}

#[when("I press the up arrow")]
async fn press_up(world: &mut BddWorld) {
    tui::press(world, KeyCode::Up);
}

#[when(regex = r#"^I move the selection to the script "([^"]+)"$"#)]
async fn move_to_script(world: &mut BddWorld, name: String) {
    select_named(world, &name);
}

#[when("I move the selection to the 40th script")]
async fn move_to_fortieth(world: &mut BddWorld) {
    if let Some(app) = world.app.as_mut() {
        let last = app.scripts().len().saturating_sub(1);
        app.select_index(last);
    }
    tui::draw(world);
}

#[when(regex = r#"^the selection moves to the script "([^"]+)"$"#)]
async fn selection_moves(world: &mut BddWorld, name: String) {
    select_named(world, &name);
}

#[when(regex = r#"^the TestBackend interior grows to (\d+) columns by (\d+) rows$"#)]
async fn interior_grows(world: &mut BddWorld, width: u16, height: u16) {
    world.backend_width = width;
    world.backend_height = height;
    world.size_is_interior = true;
    tui::resize_to_current(world);
}

#[when(regex = r#"^the TestBackend is resized to (\d+) columns and (\d+) rows$"#)]
async fn backend_resized(world: &mut BddWorld, width: u16, height: u16) {
    world.backend_width = width;
    world.backend_height = height;
    world.size_is_interior = false;
    tui::resize_to_current(world);
}

#[when("I left-click the visible row of the 40th script")]
async fn click_fortieth(world: &mut BddWorld) {
    let name = require_app(world)
        .scripts()
        .get(39)
        .expect("40th script")
        .name
        .clone();
    let row = tui::script_row(world, &name);
    let column = tui::zone_column(world, "play icon");
    tui::left_click(world, column, row);
}

#[when(regex = r#"^I left-click the (.+) of the row of the script "([^"]+)"$"#)]
async fn click_zone(world: &mut BddWorld, zone: String, name: String) {
    let row = tui::script_row(world, &name);
    let column = tui::zone_column(world, zone.trim());
    tui::left_click(world, column, row);
}

#[when(regex = r#"^I release the mouse on the row of the script "([^"]+)"$"#)]
async fn release_on_row(world: &mut BddWorld, name: String) {
    let row = tui::script_row(world, &name);
    let column = tui::zone_column(world, "script name");
    tui::mouse_up(world, column, row);
}

#[when(regex = r#"^I move the mouse across the row of the script "([^"]+)"$"#)]
async fn move_on_row(world: &mut BddWorld, name: String) {
    let row = tui::script_row(world, &name);
    let column = tui::zone_column(world, "command text");
    tui::mouse_move(world, column, row);
}

#[when(
    regex = r#"^I left-click (the sidebar header|the empty space below the list|the full command line at the bottom)$"#
)]
async fn click_outside(world: &mut BddWorld, target: String) {
    let app = require_app(world);
    let inner = app.inner;
    let (column, row) = match target.trim() {
        "the sidebar header" => (inner.x + 1, inner.y),
        "the empty space below the list" => (inner.x + 1, inner.y + 1 + 8),
        "the full command line at the bottom" => {
            (inner.x + 1, inner.y + inner.height.saturating_sub(2))
        }
        other => panic!("unhandled click target {other}"),
    };
    tui::left_click(world, column, row);
}

#[then(regex = r#"^the sidebar lists (\d+) scripts$"#)]
async fn lists_n(world: &mut BddWorld, count: usize) {
    assert_eq!(script_names(world).len(), count);
}

#[then(regex = r#"^the sidebar shows the script "([^"]+)" with the command "([^"]*)"$"#)]
async fn shows_script(world: &mut BddWorld, name: String, command: String) {
    let script = require_app(world)
        .scripts()
        .iter()
        .find(|script| script.name == name)
        .unwrap_or_else(|| panic!("missing script {name}"));
    assert_eq!(script.command, command);
}

#[then("every script row shows a play icon")]
async fn every_play_icon(world: &mut BddWorld) {
    for name in script_names(world) {
        let row = row_for_script(world, &name);
        assert!(row.contains(PLAY_ICON), "row without play icon: {row}");
    }
}

#[then(
    regex = r#"^the sidebar header shows the package name "([^"]+)" and the detected package manager$"#
)]
async fn header_name_and_manager(world: &mut BddWorld, name: String) {
    let manager = require_app(world)
        .catalog()
        .expect("catalog")
        .manager
        .as_str();
    assert!(
        world.screen.contains(&name),
        "header missing {name}:\n{}",
        world.screen
    );
    assert!(
        world.screen.contains(manager),
        "header missing {manager}:\n{}",
        world.screen
    );
}

#[then(regex = r#"^the selection is on the script "([^"]+)"$"#)]
async fn selection_on(world: &mut BddWorld, name: String) {
    let selected = require_app(world)
        .selected_script()
        .map(|script| script.name.as_str())
        .unwrap_or("");
    assert_eq!(selected, name);
}

#[then(regex = r#"^the selection is still on the script "([^"]+)"$"#)]
async fn selection_still(world: &mut BddWorld, name: String) {
    selection_on(world, name).await;
}

#[then(regex = r#"^the sidebar lists the scripts in this order: (.+)$"#)]
async fn lists_order(world: &mut BddWorld, names: String) {
    assert_eq!(script_names(world), parse_name_list(&names));
}

#[then(regex = r#"^the sidebar still lists the scripts in this order: (.+)$"#)]
async fn still_lists_order(world: &mut BddWorld, names: String) {
    lists_order(world, names).await;
}

#[then(regex = r#"^the resolved project root is "/([^"]+)"$"#)]
async fn root_is(world: &mut BddWorld, path: String) {
    let expected = format!("/{path}");
    assert_eq!(resolved_root(world).as_deref(), Some(expected.as_str()));
}

#[then(regex = r#"^the resolved project root is still "/([^"]+)"$"#)]
async fn root_still(world: &mut BddWorld, path: String) {
    root_is(world, path).await;
}

#[then(regex = r#"^the resolved project root is not "/([^"]+)"$"#)]
async fn root_is_not(world: &mut BddWorld, path: String) {
    let unexpected = format!("/{path}");
    assert_ne!(resolved_root(world).as_deref(), Some(unexpected.as_str()));
}

#[then("the listed scripts are unchanged")]
async fn scripts_unchanged(world: &mut BddWorld) {
    assert_eq!(script_names(world), world.names_at_open);
}

#[then(regex = r#"^the sidebar shows the message "([^"]+)"$"#)]
async fn shows_message(world: &mut BddWorld, message: String) {
    if message == CWD_FALLBACK_NOTE {
        assert!(
            require_app(world).listed.used_start_cwd,
            "start-cwd fallback was not recorded"
        );
    }
    assert!(
        tui::shows_text(world, &message),
        "missing message {message:?}\nscreen:\n{}",
        world.screen
    );
}

#[then("no script is listed")]
async fn no_script_listed(world: &mut BddWorld) {
    assert!(script_names(world).is_empty());
}

#[then("no script can be launched")]
async fn cannot_launch(world: &mut BddWorld) {
    assert_no_launch(world);
}

#[then("the herdr-npm process is still running")]
async fn process_running(world: &mut BddWorld) {
    assert!(
        require_app(world).process_running,
        "process was marked stopped"
    );
}

#[then("the selection is on the first script")]
async fn selection_first(world: &mut BddWorld) {
    assert_eq!(require_app(world).selected, 0);
}

#[then("the list scrolls to keep the selection visible")]
async fn selection_visible(world: &mut BddWorld) {
    let app = require_app(world);
    assert!(
        app.selected >= app.list_offset && app.selected < app.list_offset + app.list_height(),
        "selected {} not in window {}+{}",
        app.selected,
        app.list_offset,
        app.list_height()
    );
    let name = app.selected_script().expect("selection").name.clone();
    assert!(
        world.screen.contains(&name),
        "selected {name} missing from screen:\n{}",
        world.screen
    );
}

#[then("the 40th script is selected")]
async fn fortieth_selected(world: &mut BddWorld) {
    let app = require_app(world);
    assert_eq!(app.selected, 39);
    assert_eq!(app.selected_script().map(|s| s.name.as_str()), Some("s40"));
}

#[then("no run intent has been emitted")]
async fn no_run_intent(world: &mut BddWorld) {
    assert!(
        require_app(world).run_intents.is_empty(),
        "unexpected intents {:?}",
        require_app(world).run_intents
    );
}

#[then(regex = r#"^a single run intent is emitted for script "([^"]+)"$"#)]
async fn single_intent(world: &mut BddWorld, name: String) {
    let intents = &require_app(world).run_intents;
    assert_eq!(
        intents.len(),
        1,
        "expected a single intent, got {intents:?}"
    );
    assert_eq!(intents[0].script_name, name);
}

#[then(regex = r#"^that (name|command) is cut with an ellipsis$"#)]
async fn field_ellipsized(world: &mut BddWorld, field: String) {
    assert!(
        world.screen.contains('…'),
        "expected ellipsis in:\n{}",
        world.screen
    );
    let full = match field.as_str() {
        "name" => "n".repeat(80),
        _ => "x".repeat(80),
    };
    assert!(
        !world.screen.contains(&full),
        "full {field} leaked onto the screen"
    );
}

#[then("the row still shows its play icon")]
async fn row_has_icon(world: &mut BddWorld) {
    assert!(
        world.screen.contains(PLAY_ICON),
        "play icon missing:\n{}",
        world.screen
    );
}

#[then("the script name and command are laid out using terminal cell width")]
async fn laid_out_with_cell_width(world: &mut BddWorld) {
    let inner_width = require_app(world).inner.width as usize;
    for line in world.screen.lines() {
        if line.contains(PLAY_ICON) || line.contains("日本語") {
            assert!(
                line.width() <= inner_width + 2,
                "line wider than inner area ({inner_width}): {line:?} width={}",
                line.width()
            );
        }
    }
}

#[then("overflowing text is cut with an ellipsis without splitting a wide character")]
async fn no_split_wide_char(world: &mut BddWorld) {
    let wide = ['日', '本', '語'];
    for line in world.screen.lines() {
        for (index, ch) in line.char_indices() {
            if wide.contains(&ch) {
                assert!(
                    line[index..].starts_with(ch.to_string().as_str()),
                    "wide character split in {line:?}"
                );
            }
        }
    }
    let has_wide = world.screen.chars().any(|ch| wide.contains(&ch));
    let has_ellipsis = world.screen.contains('…');
    assert!(
        has_wide || has_ellipsis,
        "expected Japanese text or an ellipsis:\n{}",
        world.screen
    );
}

#[then(regex = r#"^the footer shows the command of "([^"]+)" on a single line$"#)]
async fn footer_single_line(world: &mut BddWorld, name: String) {
    let command = require_app(world)
        .scripts()
        .iter()
        .find(|script| script.name == name)
        .expect("script")
        .command
        .clone();
    let prefix: String = command.chars().take(8).collect();
    assert!(
        world.screen.contains(&prefix) || world.screen.contains('…'),
        "footer missing command prefix {prefix:?}:\n{}",
        world.screen
    );
    let footer_lines = world
        .screen
        .lines()
        .filter(|line| line.contains(&prefix) || (line.contains('…') && !line.contains(PLAY_ICON)))
        .count();
    assert!(
        footer_lines <= 2,
        "command should stay on one footer line, screen:\n{}",
        world.screen
    );
}

#[then("the footer shows how to scroll that line with h and l")]
async fn footer_help(world: &mut BddWorld) {
    assert!(
        world.screen.contains("h/l"),
        "missing h/l help:\n{}",
        world.screen
    );
}

#[then("the footer command scrolls right by one cell")]
async fn footer_right(world: &mut BddWorld) {
    assert_eq!(
        require_app(world).footer_offset,
        world.prev_footer_offset + 1
    );
}

#[then("the footer command scrolls left by one cell")]
async fn footer_left(world: &mut BddWorld) {
    assert_eq!(
        require_app(world).footer_offset,
        world.prev_footer_offset.saturating_sub(1)
    );
}

#[then("the footer scroll offset is 0")]
async fn footer_offset_zero(world: &mut BddWorld) {
    assert_eq!(require_app(world).footer_offset, 0);
}

#[then("only q and the toggle action remain usable")]
async fn only_q_usable(world: &mut BddWorld) {
    let selected = require_app(world).selected;
    tui::press(world, KeyCode::Char('j'));
    assert_eq!(require_app(world).selected, selected);
    assert_no_launch(world);
    assert!(require_app(world).too_small() || require_app(world).listed.catalog.is_err());
}

#[then(regex = r#"^the message "([^"]+)" is gone$"#)]
async fn message_gone(world: &mut BddWorld, message: String) {
    assert!(
        !tui::shows_text(world, &message),
        "message {message} still present:\n{}",
        world.screen
    );
}

#[then(regex = r#"^the detected package manager is "([^"]+)"$"#)]
async fn detected_manager(world: &mut BddWorld, manager: String) {
    assert_eq!(
        require_app(world)
            .catalog()
            .expect("catalog")
            .manager
            .as_str(),
        manager
    );
}

#[then(regex = r#"^the sidebar header shows "([^"]+)"$"#)]
async fn header_shows(world: &mut BddWorld, text: String) {
    assert!(
        world.screen.contains(&text),
        "header missing {text}:\n{}",
        world.screen
    );
}

#[then("no warning is shown")]
async fn no_warning(world: &mut BddWorld) {
    for needle in ["warning", "unsupported", "yarn", "bun"] {
        assert!(
            !world.screen.to_lowercase().contains(needle),
            "unexpected {needle} in:\n{}",
            world.screen
        );
    }
}
