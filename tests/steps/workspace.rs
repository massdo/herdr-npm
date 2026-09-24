use crossterm::event::KeyCode;
use cucumber::{given, then, when};
use std::fs;

use crate::support::{tui, world::BddWorld};

#[given("a disposable Journal-shaped pnpm workspace")]
async fn fixture(world: &mut BddWorld) {
    let root = world.map_path("/monorepo");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("pnpm-workspace.yaml"),
        "packages: ['apps/*', 'packages/*']\nuseNodeVersion: 22.0.0\n",
    )
    .unwrap();
    fs::write(root.join("package.json"), r#"{"name":"journal-fixture","packageManager":"pnpm@10.10.0","scripts":{"root":"echo root"}}"#).unwrap();
    for member in [
        "apps/auth",
        "apps/cli",
        "apps/mcp",
        "packages/core",
        "packages/infrastructure",
    ] {
        fs::create_dir_all(root.join(member)).unwrap();
        let body = if member.starts_with("apps/") {
            r#"{"name":"same-name","scripts":{"dev":"echo witness","start":"echo start"}}"#
        } else {
            r#"{"name":"empty","scripts":{}}"#
        };
        fs::write(root.join(member).join("package.json"), body).unwrap();
    }
    world.backend_width = 110;
    world.backend_height = 24;
    world.auto_launch = false;
}

#[when(expr = "the workspace sidebar opens from {string}")]
async fn open(world: &mut BddWorld, origin: String) {
    world.foreground_cwd = Some(world.map_path("/monorepo").join(origin));
    tui::open_sidebar(world);
}

#[then("all six workspace groups and both empty packages are rendered")]
async fn groups(world: &mut BddWorld) {
    for path in [
        "[.]",
        "[apps/auth]",
        "[apps/cli]",
        "[apps/mcp]",
        "[packages/core]",
        "[packages/infrastructure]",
    ] {
        assert!(
            world.screen.contains(path),
            "missing {path}: {}",
            world.screen
        );
    }
    assert!(world.screen.contains("6 packages"));
    assert_eq!(world.screen.matches("No scripts").count(), 2);
}

#[then("all workspace packages resolve to pnpm")]
async fn managers(world: &mut BddWorld) {
    for package in &world.app.as_ref().unwrap().workspace().unwrap().packages {
        assert_eq!(package.catalog.as_ref().unwrap().manager.as_str(), "pnpm");
    }
}

#[then(expr = "the selected workspace script belongs to {string}")]
async fn selected(world: &mut BddWorld, member: String) {
    let app = world.app.as_ref().unwrap();
    assert!(app.selected_script().is_some());
    assert_eq!(
        app.rows[app.selected].package_root(),
        world
            .map_path("/monorepo")
            .join(member)
            .canonicalize()
            .unwrap()
    );
}

#[when(expr = "I filter workspace scripts by {string}")]
async fn filter(world: &mut BddWorld, query: String) {
    tui::press(world, KeyCode::Char('/'));
    for ch in query.chars() {
        tui::press(world, KeyCode::Char(ch));
    }
}

#[then("the three dev results are grouped under their package paths")]
async fn results(world: &mut BddWorld) {
    let app = world.app.as_ref().unwrap();
    assert_eq!(
        app.search
            .matches
            .iter()
            .filter(|m| app.row_script(m.index).is_some())
            .count(),
        3
    );
    for path in ["apps/auth", "apps/cli", "apps/mcp"] {
        assert!(world.screen.contains(&format!("- [{path}]")));
    }
}

#[when("I apply the workspace search")]
async fn apply(world: &mut BddWorld) {
    tui::press(world, KeyCode::Enter);
}

#[then("no workspace script has been launched")]
async fn no_launch(world: &mut BddWorld) {
    assert!(world.app.as_ref().unwrap().run_intents.is_empty());
    assert!(world.herdr.created_tabs().is_empty());
}

#[when("I launch the selected workspace script")]
async fn launch(world: &mut BddWorld) {
    tui::press(world, KeyCode::Enter);
    herdr_npm::adapters::tui::flush_intents(world.app.as_mut().unwrap(), &world.herdr);
}

#[then(expr = "exactly one background tab runs pnpm dev in {string}")]
async fn launched(world: &mut BddWorld, member: String) {
    let tabs = world.herdr.created_tabs();
    assert_eq!(tabs.len(), 1);
    assert_eq!(
        tabs[0].cwd,
        world
            .map_path("/monorepo")
            .join(member)
            .canonicalize()
            .unwrap()
    );
    assert_eq!(tabs[0].workspace_id, world.workspace_id);
    assert!(!tabs[0].focus);
    assert_eq!(tabs[0].label, "pnpm run -- dev");
    assert_eq!(
        world
            .herdr
            .calls()
            .iter()
            .filter(|call| call.method == "pane.send_input")
            .count(),
        1
    );
}

#[when("I clear the workspace search")]
async fn clear(world: &mut BddWorld) {
    tui::press(world, KeyCode::Esc);
}

#[then("the workspace member groups are collapsed again")]
async fn collapsed(world: &mut BddWorld) {
    for path in ["apps/auth", "apps/cli", "apps/mcp"] {
        assert!(
            world.screen.contains(&format!("+ [{path}]")),
            "{}",
            world.screen
        );
    }
}

#[given(expr = "the workspace member {string} has invalid JSON")]
async fn broken(world: &mut BddWorld, member: String) {
    fs::write(
        world
            .map_path("/monorepo")
            .join(member)
            .join("package.json"),
        "{",
    )
    .unwrap();
}

#[then("the broken member diagnostic and the mcp scripts are rendered")]
async fn diagnostic(world: &mut BddWorld) {
    for text in [
        "[apps/auth]",
        "package.json is not valid JSON",
        "- [apps/mcp]",
        "echo witness",
    ] {
        assert!(
            world.screen.contains(text),
            "missing {text}: {}",
            world.screen
        );
    }
}
