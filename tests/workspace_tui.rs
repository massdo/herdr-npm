mod workspace_fixture;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use herdr_npm::adapters::fs_project::FsProject;
use herdr_npm::adapters::tui::{
    app::{CatalogRow, SidebarApp},
    theme::Theme,
    view,
};
use herdr_npm::application::list_scripts::{list_scripts, origin_for_paths};
use ratatui::{Terminal, backend::TestBackend};
use workspace_fixture::Fixture;

fn app(f: &Fixture, origin: &str) -> SidebarApp {
    SidebarApp::new(list_scripts(
        &FsProject::capped(f.path("")),
        &origin_for_paths(Some(f.path(origin)), None),
    ))
}

fn paint(app: &mut SidebarApp, width: u16, height: u16) -> (String, Terminal<TestBackend>) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| view::render(frame, app)).unwrap();
    let buffer = terminal.backend().buffer();
    let text = (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    (text, terminal)
}

fn key(app: &mut SidebarApp, code: KeyCode) -> bool {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn search(app: &mut SidebarApp, query: &str) {
    app.open_search();
    for ch in query.chars() {
        key(app, KeyCode::Char(ch));
    }
}

fn group(app: &SidebarApp, f: &Fixture, path: &str) -> usize {
    app.rows
        .iter()
        .position(|row| row == &CatalogRow::Group(f.path(path)))
        .unwrap()
}

fn click(app: &mut SidebarApp, index: usize, gutter: bool) {
    app.select_index(index);
    paint(app, 100, 24);
    let geo = app.layout();
    let pos = app.visible_pos(index).unwrap();
    app.handle_mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: geo.list.x + if gutter { 0 } else { 8 },
        row: geo.list.y + (pos - app.list_offset) as u16,
        modifiers: KeyModifiers::NONE,
    });
}

#[test]
fn six_groups_render_from_root_and_member_with_the_correct_initial_selection() {
    let f = Fixture::journal();
    assert_eq!(f.workspace("").packages.len(), 6);
    for origin in ["", "apps/mcp"] {
        let mut app = app(&f, origin);
        let (screen, terminal) = paint(&mut app, 100, 24);
        assert!(screen.contains("6 packages"), "{screen}");
        for path in [
            "[.]",
            "[apps/auth]",
            "[apps/cli]",
            "[apps/mcp]",
            "[packages/core]",
            "[packages/infrastructure]",
        ] {
            assert!(screen.contains(path), "missing {path}: {screen}");
        }
        assert_eq!(screen.matches("No scripts").count(), 2);
        assert!(screen.contains("pnpm"));
        assert!(app.expanded.contains(&f.path("")));
        assert_eq!(
            app.selected_script().unwrap().name,
            if origin.is_empty() { "root" } else { "dev" }
        );
        assert_eq!(app.rows[app.selected].package_root(), f.path(origin));
        let y =
            app.layout().list.y + (app.visible_pos(app.selected).unwrap() - app.list_offset) as u16;
        assert_eq!(
            terminal.backend().buffer()[(app.inner.x, y)].bg,
            app.theme.palette.selection_bg
        );
        assert!(!app.expanded.contains(&f.path("apps/auth")));
    }
}

#[test]
fn collapse_selects_the_header_navigation_does_not_wrap_and_groups_never_run() {
    let f = Fixture::journal();
    let mut app = app(&f, "apps/mcp");
    paint(&mut app, 100, 24);
    key(&mut app, KeyCode::Left);
    assert_eq!(app.selected, group(&app, &f, "apps/mcp"));
    assert!(app.selected_script().is_none());
    let (screen, _) = paint(&mut app, 100, 24);
    assert!(screen.contains("+ [apps/mcp]"));
    assert!(!screen.contains("echo witness"));
    key(&mut app, KeyCode::Right);
    let (screen, _) = paint(&mut app, 100, 24);
    assert!(screen.contains("- [apps/mcp]"));
    assert!(screen.contains("echo witness"));
    key(&mut app, KeyCode::Enter);
    assert!(!app.expanded.contains(&f.path("apps/mcp")));
    assert!(app.run_intents.is_empty());
    for _ in 0..30 {
        key(&mut app, KeyCode::Up);
    }
    assert_eq!(app.selected, 0);
    for _ in 0..30 {
        key(&mut app, KeyCode::Char('j'));
    }
    assert_eq!(
        app.selected,
        *app.search.matches.last().map(|m| &m.index).unwrap()
    );
    let (screen, _) = paint(&mut app, 100, 24);
    assert!(screen.contains("No scripts"));
    app.emit_run();
    assert!(app.run_intents.is_empty());
    key(&mut app, KeyCode::Char('l'));
    assert_eq!(app.footer_offset, 1);
    key(&mut app, KeyCode::Char('h'));
    assert_eq!(app.footer_offset, 0);
}

#[test]
fn search_opens_closed_groups_temporarily_and_keeps_homonyms_distinct() {
    let f = Fixture::journal();
    let mut app = app(&f, "");
    paint(&mut app, 100, 24);
    let expanded = app.expanded.clone();
    search(&mut app, "dev");
    let (screen, _) = paint(&mut app, 100, 24);
    for path in ["apps/auth", "apps/cli", "apps/mcp"] {
        assert!(screen.contains(&format!("- [{path}]")), "{screen}");
    }
    assert_eq!(
        app.search
            .matches
            .iter()
            .filter(|m| app.row_script(m.index).is_some())
            .count(),
        3
    );
    assert_eq!(app.expanded, expanded);
    assert!(!key(&mut app, KeyCode::Enter));
    assert!(app.run_intents.is_empty());
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.run_intents.len(), 1);
    assert_eq!(app.run_intents[0].package_root, f.path("apps/auth"));
    assert_eq!(app.run_intents[0].script_name, "dev");
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.run_intents.len(), 2);
    assert_eq!(app.run_intents[1].package_root, f.path("apps/cli"));
    assert!(!key(&mut app, KeyCode::Esc));
    let (screen, _) = paint(&mut app, 100, 24);
    assert!(screen.contains("+ [apps/cli]"));
    assert_eq!(app.expanded, expanded);
    assert_eq!(app.selected, group(&app, &f, "apps/cli"));
    assert!(key(&mut app, KeyCode::Esc));
}

#[test]
fn mouse_group_body_and_play_gutter_have_distinct_actions_with_both_icon_sets() {
    let f = Fixture::journal();
    for theme in [Theme::ascii(), Theme::nerd()] {
        let mut app = app(&f, "");
        app.theme = theme;
        paint(&mut app, 100, 24);
        let header = group(&app, &f, "apps/mcp");
        click(&mut app, header, false);
        let (screen, _) = paint(&mut app, 100, 24);
        assert!(screen.contains("- [apps/mcp]"));
        assert!(app.run_intents.is_empty());
        click(&mut app, header + 1, false);
        assert!(app.run_intents.is_empty());
        click(&mut app, header + 1, true);
        assert_eq!(app.run_intents.len(), 1);
        assert_eq!(app.run_intents[0].package_root, f.path("apps/mcp"));
        assert_eq!(app.run_intents[0].script_name, "dev");
        click(&mut app, header, true);
        assert_eq!(app.run_intents.len(), 1);
        assert_eq!(app.selected, header);
        assert!(paint(&mut app, 100, 24).0.contains("+ [apps/mcp]"));
    }
}

#[test]
fn wheel_moves_the_rendered_viewport_without_moving_selection() {
    let f = Fixture::journal();
    let mut app = app(&f, "");
    let (before, _) = paint(&mut app, 100, 9);
    let selected = app.selected;
    let geo = app.layout();
    app.handle_mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: geo.list.x,
        row: geo.list.y,
        modifiers: KeyModifiers::NONE,
    });
    let offset = app.list_offset;
    assert!(offset > 0);
    let (after, _) = paint(&mut app, 100, 9);
    assert_eq!(app.selected, selected);
    assert_eq!(app.list_offset, offset);
    assert_ne!(before, after);
    assert!(after.contains("[packages/core]"), "{after}");
    app.handle_mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: geo.list.x,
        row: geo.list.y,
        modifiers: KeyModifiers::NONE,
    });
    assert!(
        paint(&mut app, 100, 9)
            .0
            .contains("[packages/infrastructure]")
    );
    assert_eq!(app.selected, selected);
    key(&mut app, KeyCode::Down);
    let (screen, _) = paint(&mut app, 100, 9);
    assert!(screen.contains("[apps/auth]"));
}

#[test]
fn no_results_escape_small_dimensions_and_search_entrypoints() {
    let f = Fixture::journal();
    let mut app = app(&f, "apps/mcp");
    paint(&mut app, 100, 24);
    app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));
    assert!(app.search.is_editing());
    key(&mut app, KeyCode::Char('z'));
    let (screen, _) = paint(&mut app, 100, 24);
    assert!(screen.contains("No script matches \"z\""));
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Enter);
    assert!(app.run_intents.is_empty());
    assert!(!key(&mut app, KeyCode::Esc));
    assert!(paint(&mut app, 100, 24).0.contains("[apps/mcp]"));
    let geo = app.layout();
    app.handle_mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: geo.magnifier.x,
        row: geo.magnifier.y,
        modifiers: KeyModifiers::NONE,
    });
    assert!(app.search.is_editing());
    key(&mut app, KeyCode::Esc);
    assert!(paint(&mut app, 10, 5).0.contains("Terminal"));
    let selected = app.selected;
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.selected, selected);
    assert!(app.run_intents.is_empty());
    assert!(key(&mut app, KeyCode::Char('q')));
}

#[test]
fn invalid_members_and_rootless_workspace_render_without_hiding_valid_scripts() {
    let f = Fixture::journal();
    f.package("apps/auth", "{");
    std::fs::remove_file(f.path("package.json")).unwrap();
    let mut app = app(&f, "apps/mcp");
    let (screen, _) = paint(&mut app, 110, 24);
    assert!(screen.contains("5 packages"));
    assert!(screen.contains("[apps/auth]"));
    assert!(screen.contains("package.json is not valid JSON"));
    assert!(screen.contains("echo witness"));
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.run_intents[0].package_root, f.path("apps/mcp"));
    for path in ["apps/mcp", "apps/cli"] {
        f.package(path, "{");
    }
    let mut broken = app_for_all_errors(&f);
    assert!(
        paint(&mut broken, 110, 24)
            .0
            .contains("package.json is not valid JSON")
    );
}

fn app_for_all_errors(f: &Fixture) -> SidebarApp {
    f.package("packages/core", "{");
    f.package("packages/infrastructure", "{");
    app(f, "")
}
