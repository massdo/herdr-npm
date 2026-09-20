use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use herdr_npm::adapters::fs_project::FsProject;
use herdr_npm::adapters::tui::app::SidebarApp;
use herdr_npm::adapters::tui::keymap;
use herdr_npm::adapters::tui::view;
use herdr_npm::application::list_scripts::{list_scripts, origin_for_paths};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use unicode_width::UnicodeWidthStr;

use super::world::BddWorld;

pub fn backend_size(world: &BddWorld) -> (u16, u16) {
    if world.size_is_interior {
        (
            world.backend_width.saturating_add(2),
            world.backend_height.saturating_add(2),
        )
    } else {
        (world.backend_width, world.backend_height)
    }
}

pub fn draw(world: &mut BddWorld) {
    let (width, height) = backend_size(world);
    let Some(app) = world.app.as_mut() else {
        world.screen.clear();
        return;
    };
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal");
    terminal
        .draw(|frame| view::render(frame, app))
        .expect("draw");
    world.screen = buffer_to_string(terminal.backend());
}

fn buffer_to_string(backend: &TestBackend) -> String {
    let buffer = backend.buffer();
    let area = buffer.area;
    let mut lines = Vec::new();
    for y in 0..area.height {
        let mut line = String::new();
        let mut x = 0u16;
        while x < area.width {
            let cell = &buffer[(x, y)];
            let symbol = cell.symbol();
            line.push_str(symbol);
            let width = symbol.width().max(1) as u16;
            x = x.saturating_add(width);
        }
        lines.push(line.trim_end().to_string());
    }
    lines.join("\n")
}

pub fn open_sidebar(world: &mut BddWorld) {
    world.tui_wanted = true;
    let project = FsProject::capped(world.fs_root.clone());
    let listed = list_scripts(
        &project,
        &origin_for_paths(world.foreground_cwd.clone(), world.start_cwd.clone()),
    );
    let mut app = SidebarApp::new(listed);
    world.names_at_open = app
        .scripts()
        .iter()
        .map(|script| script.name.clone())
        .collect();
    app.workspace_id = world.workspace_id.clone();
    world.app = Some(app);
    draw(world);
}

pub fn snapshot(world: &mut BddWorld) {
    if let Some(app) = world.app.as_ref() {
        world.prev_footer_offset = app.footer_offset;
        world.prev_selected = app.selected;
    }
}

pub fn press(world: &mut BddWorld, code: KeyCode) {
    snapshot(world);
    let Some(app) = world.app.as_mut() else {
        panic!("sidebar TUI is not open");
    };
    let key = KeyEvent::new(code, KeyModifiers::NONE);
    if keymap::handle_event(app, Event::Key(key)) {
        app.process_running = false;
        world.app = None;
        world.screen.clear();
        return;
    }
    draw(world);
    if world.auto_launch {
        launch_pending(world);
    }
}

pub fn resize_to_current(world: &mut BddWorld) {
    snapshot(world);
    let (width, height) = backend_size(world);
    if let Some(app) = world.app.as_mut() {
        keymap::handle_event(app, Event::Resize(width, height));
    }
    draw(world);
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

pub fn left_click(world: &mut BddWorld, column: u16, row: u16) {
    snapshot(world);
    let Some(app) = world.app.as_mut() else {
        panic!("sidebar TUI is not open");
    };
    app.run_intents.clear();
    keymap::handle_event(
        app,
        Event::Mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row)),
    );
    draw(world);
    if world.auto_launch {
        launch_pending(world);
    }
}

pub fn mouse_up(world: &mut BddWorld, column: u16, row: u16) {
    snapshot(world);
    let Some(app) = world.app.as_mut() else {
        panic!("sidebar TUI is not open");
    };
    keymap::handle_event(
        app,
        Event::Mouse(mouse(MouseEventKind::Up(MouseButton::Left), column, row)),
    );
    draw(world);
}

pub fn mouse_move(world: &mut BddWorld, column: u16, row: u16) {
    snapshot(world);
    let Some(app) = world.app.as_mut() else {
        panic!("sidebar TUI is not open");
    };
    keymap::handle_event(app, Event::Mouse(mouse(MouseEventKind::Moved, column, row)));
    draw(world);
}

pub fn script_row(world: &BddWorld, name: &str) -> u16 {
    let app = world.app.as_ref().expect("sidebar TUI is not open");
    let index = app
        .scripts()
        .iter()
        .position(|script| script.name == name)
        .unwrap_or_else(|| panic!("script {name} is not listed"));
    assert!(
        index >= app.list_offset && index < app.list_offset + app.list_height(),
        "script {name} is not in the visible window"
    );
    app.inner.y + 1 + (index - app.list_offset) as u16
}

pub fn zone_column(world: &BddWorld, zone: &str) -> u16 {
    let app = world.app.as_ref().expect("sidebar TUI is not open");
    for column in app.inner.x..app.inner.x.saturating_add(app.inner.width) {
        if view::row_zone(app.inner, column) == zone {
            return column;
        }
    }
    panic!("no column mapped to zone {zone}");
}

pub fn close_with_q(world: &mut BddWorld) {
    press(world, KeyCode::Char('q'));
}

pub fn launch_pending(world: &mut BddWorld) {
    let intents = match world.app.as_mut() {
        Some(app) => std::mem::take(&mut app.run_intents),
        None => return,
    };
    let catalog = world.app.as_ref().and_then(|app| app.catalog().cloned());
    let workspace = world
        .app
        .as_ref()
        .map(|app| app.workspace_id.clone())
        .unwrap_or_else(|| world.workspace_id.clone());
    let Some(catalog) = catalog else {
        return;
    };
    for intent in intents {
        match herdr_npm::application::run_script::run_script(
            &world.herdr,
            &catalog,
            &workspace,
            &intent.script_name,
        ) {
            Ok(_) => {
                if let Some(app) = world.app.as_mut() {
                    app.launch_error = None;
                }
                world.last_error = None;
            }
            Err(error) => {
                world.last_error = Some(error.clone());
                if let Some(app) = world.app.as_mut() {
                    app.launch_error = Some(error);
                }
            }
        }
    }
    draw(world);
}
