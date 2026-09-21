use crossterm::event::{KeyCode, KeyModifiers};
use cucumber::{then, when};

use crate::support::tui;
use crate::support::world::BddWorld;
use herdr_npm::adapters::tui::app::SearchMode;

fn require_app(world: &BddWorld) -> &herdr_npm::adapters::tui::app::SidebarApp {
    world.app.as_ref().expect("sidebar TUI is not open")
}

#[when("I press Ctrl+F")]
async fn press_ctrl_f(world: &mut BddWorld) {
    tui::press_modified(world, KeyCode::Char('f'), KeyModifiers::CONTROL);
}

#[when(regex = r#"^I type "([^"]*)"$"#)]
async fn type_text(world: &mut BddWorld, text: String) {
    for ch in text.chars() {
        tui::press(world, KeyCode::Char(ch));
    }
}

#[when("I left-click the search magnifier")]
async fn click_magnifier(world: &mut BddWorld) {
    let geo = tui::geometry(world);
    assert!(geo.magnifier.width > 0, "magnifier rectangle is empty");
    tui::left_click(world, geo.magnifier.x, geo.magnifier.y);
}

#[then("the search field is open")]
async fn search_open(world: &mut BddWorld) {
    assert_eq!(require_app(world).search.mode, SearchMode::Editing);
    let geo = tui::geometry(world);
    assert!(geo.search.height > 0, "search field has no height");
}

#[then("the search field is closed")]
async fn search_closed(world: &mut BddWorld) {
    assert_ne!(require_app(world).search.mode, SearchMode::Editing);
    let geo = tui::geometry(world);
    assert_eq!(geo.search.height, 0);
}

#[then(regex = r#"^the search query is "([^"]*)"$"#)]
async fn search_query(world: &mut BddWorld, query: String) {
    assert_eq!(require_app(world).search.query, query);
}

#[then(regex = r#"^no script matches "([^"]+)"$"#)]
async fn no_script_matches(world: &mut BddWorld, query: String) {
    let expected = format!("No script matches \"{query}\"");
    assert_eq!(
        require_app(world).no_match_message().as_deref(),
        Some(expected.as_str())
    );
    assert!(
        tui::shows_text(world, &expected),
        "missing {expected:?}\n{}",
        world.screen
    );
}

#[then(regex = r#"^the sidebar shows the matching scripts: (.+)$"#)]
async fn matching_scripts(world: &mut BddWorld, names: String) {
    let expected: Vec<String> = names
        .split(',')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect();
    let app = require_app(world);
    let got: Vec<String> = app
        .search
        .matches
        .iter()
        .map(|item| app.scripts()[item.index].name.clone())
        .collect();
    assert_eq!(got, expected);
}
