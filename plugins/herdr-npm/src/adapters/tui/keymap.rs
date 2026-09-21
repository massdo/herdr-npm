use crossterm::event::{Event, KeyEventKind};
use ratatui::layout::Rect;

use super::app::SidebarApp;

/// Apply a crossterm event. Returns true when the TUI should quit.
pub fn handle_event(app: &mut SidebarApp, event: Event) -> bool {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => app.handle_key(key),
        Event::Mouse(mouse) => {
            app.handle_mouse(mouse);
            false
        }
        Event::Resize(width, height) => {
            app.set_inner(Rect {
                x: app.inner.x,
                y: app.inner.y,
                width: width.saturating_sub(2),
                height: height.saturating_sub(2),
            });
            false
        }
        _ => false,
    }
}
