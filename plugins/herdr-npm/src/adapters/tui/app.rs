use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use unicode_width::UnicodeWidthChar;

use crate::application::list_scripts::ListedScripts;
use crate::domain::CWD_FALLBACK_NOTE;
use crate::domain::catalog::{PackageCatalog, RunIntent, Script};
use crate::domain::error::AppError;

pub const PLAY_ICON: &str = "▶";
pub const MIN_INNER_COLS: u16 = 12;
pub const MIN_INNER_ROWS: u16 = 4;

#[derive(Debug, Clone)]
pub struct SidebarApp {
    pub listed: ListedScripts,
    pub selected: usize,
    pub list_offset: usize,
    pub footer_offset: usize,
    pub run_intents: Vec<RunIntent>,
    pub inner: Rect,
    pub process_running: bool,
    pub workspace_id: String,
    pub launch_error: Option<AppError>,
}

impl SidebarApp {
    pub fn new(listed: ListedScripts) -> Self {
        Self {
            listed,
            selected: 0,
            list_offset: 0,
            footer_offset: 0,
            run_intents: Vec::new(),
            inner: Rect::new(0, 0, 32, 24),
            process_running: true,
            workspace_id: String::new(),
            launch_error: None,
        }
    }

    pub fn catalog(&self) -> Option<&PackageCatalog> {
        self.listed.catalog.as_ref().ok()
    }

    pub fn error_message(&self) -> Option<String> {
        match &self.listed.catalog {
            Err(error) => Some(error.to_string()),
            Ok(_) if self.too_small() => Some(AppError::TerminalTooSmall.to_string()),
            Ok(_) => self.launch_error.as_ref().map(ToString::to_string),
        }
    }

    pub fn notes(&self) -> Vec<String> {
        let mut notes = Vec::new();
        if self.listed.used_start_cwd {
            notes.push(CWD_FALLBACK_NOTE.to_string());
        }
        notes
    }

    pub fn too_small(&self) -> bool {
        self.inner.width < MIN_INNER_COLS || self.inner.height < MIN_INNER_ROWS
    }

    pub fn scripts(&self) -> &[Script] {
        self.catalog()
            .map(|catalog| catalog.scripts.as_slice())
            .unwrap_or(&[])
    }

    pub fn selected_script(&self) -> Option<&Script> {
        self.scripts().get(self.selected)
    }

    pub fn list_height(&self) -> usize {
        (self.inner.height as usize)
            .saturating_sub(2 + self.status_lines().len())
            .max(1)
    }

    /// Reserve visible footer rows for launch errors and cwd fallback notices.
    pub fn status_lines(&self) -> Vec<String> {
        let messages: Vec<String> = self
            .launch_error
            .iter()
            .map(ToString::to_string)
            .chain(self.notes())
            .collect();
        if messages.is_empty() {
            return vec!["h/l scroll".into()];
        }
        let mut lines = Vec::new();
        for message in messages {
            let mut line = String::new();
            let mut cells = 0;
            for ch in message.chars() {
                let width = ch.width().unwrap_or(0);
                if ch == '\n' || cells + width > self.inner.width.max(1) as usize {
                    lines.push(std::mem::take(&mut line));
                    cells = 0;
                }
                if ch != '\n' {
                    line.push(ch);
                    cells += width;
                }
            }
            lines.push(line);
        }
        // Keep a header, one selectable script and the command line even in a small pane.
        let available = self.inner.height.saturating_sub(3).max(1) as usize;
        if lines.len() < available {
            lines.insert(0, "h/l scroll".into());
        }
        lines.truncate(available);
        lines
    }

    pub fn ensure_visible(&mut self) {
        let height = self.list_height();
        if self.selected < self.list_offset {
            self.list_offset = self.selected;
        } else if self.selected >= self.list_offset + height {
            self.list_offset = self.selected + 1 - height;
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        let len = self.scripts().len();
        if len == 0 {
            return;
        }
        let next = self.selected as isize + delta;
        let clamped = next.clamp(0, len as isize - 1) as usize;
        if clamped != self.selected {
            self.selected = clamped;
            self.footer_offset = 0;
            self.ensure_visible();
        }
    }

    pub fn select_index(&mut self, index: usize) {
        if index < self.scripts().len() {
            if self.selected != index {
                self.footer_offset = 0;
            }
            self.selected = index;
            self.ensure_visible();
        }
    }

    pub fn emit_run(&mut self) {
        if self.too_small() || self.listed.catalog.is_err() {
            return;
        }
        if let Some(script) = self.selected_script() {
            self.run_intents.push(RunIntent {
                script_name: script.name.clone(),
            });
        }
    }

    pub fn scroll_footer(&mut self, delta: isize) {
        let next = self.footer_offset as isize + delta;
        self.footer_offset = next.max(0) as usize;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
            return true;
        }
        if self.too_small() || self.listed.catalog.is_err() {
            return false;
        }
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Char('l') | KeyCode::Right => self.scroll_footer(1),
            KeyCode::Char('h') | KeyCode::Left => self.scroll_footer(-1),
            KeyCode::Enter => self.emit_run(),
            _ => {}
        }
        false
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.too_small() || self.listed.catalog.is_err() {
            return;
        }
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return;
        }
        let Some(index) = self.row_at(mouse.column, mouse.row) else {
            return;
        };
        self.select_index(index);
        self.emit_run();
    }

    pub fn row_at(&self, column: u16, row: u16) -> Option<usize> {
        if column < self.inner.x || row < self.inner.y {
            return None;
        }
        let inner_y = row.saturating_sub(self.inner.y);
        let inner_x = column.saturating_sub(self.inner.x);
        if inner_x >= self.inner.width {
            return None;
        }
        if inner_y == 0 {
            return None;
        }
        if inner_y as usize > self.list_height() {
            return None;
        }
        let list_y = inner_y.saturating_sub(1) as usize;
        let index = self.list_offset + list_y;
        if index < self.scripts().len() {
            Some(index)
        } else {
            None
        }
    }

    pub fn set_inner(&mut self, inner: Rect) {
        self.inner = inner;
        self.ensure_visible();
    }
}
