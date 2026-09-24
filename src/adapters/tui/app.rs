use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use unicode_width::UnicodeWidthChar;

use super::theme::{FOOTER_HELP, Theme};
use crate::application::list_scripts::ListedScripts;
use crate::domain::CWD_FALLBACK_NOTE;
use crate::domain::catalog::{PackageCatalog, ProjectCatalog, RunIntent, Script, WorkspaceCatalog};
use crate::domain::error::AppError;
use crate::domain::fuzzy::{FuzzyMatch, filter_names};

pub const MIN_INNER_COLS: u16 = 12;
pub const MIN_INNER_ROWS: u16 = 4;
pub const WHEEL_LINES: isize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Off,
    Editing,
    Applied,
}

#[derive(Debug, Clone)]
pub struct SearchState {
    pub mode: SearchMode,
    pub query: String,
    pub matches: Vec<FuzzyMatch>,
}

impl SearchState {
    fn for_len(len: usize) -> Self {
        Self {
            mode: SearchMode::Off,
            query: String::new(),
            matches: unfiltered(len),
        }
    }

    pub fn is_editing(&self) -> bool {
        self.mode == SearchMode::Editing
    }

    pub fn has_filter(&self) -> bool {
        self.mode != SearchMode::Off || !self.query.is_empty()
    }
}

fn unfiltered(len: usize) -> Vec<FuzzyMatch> {
    (0..len)
        .map(|index| FuzzyMatch {
            index,
            positions: Vec::new(),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogRow {
    Group(PathBuf),
    Script(RunIntent),
}

impl CatalogRow {
    pub fn package_root(&self) -> &Path {
        match self {
            Self::Group(root) => root,
            Self::Script(intent) => &intent.package_root,
        }
    }
}

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
    pub search: SearchState,
    pub theme: Theme,
    /// Stable identities in catalogue order; visible positions live in search.matches.
    pub rows: Vec<CatalogRow>,
    pub expanded: BTreeSet<PathBuf>,
}

impl SidebarApp {
    pub fn new(listed: ListedScripts) -> Self {
        Self::with_theme(listed, Theme::ascii())
    }

    pub fn with_theme(listed: ListedScripts, theme: Theme) -> Self {
        let mut rows = Vec::new();
        let mut expanded = BTreeSet::new();
        let mut preferred = None;
        let mut append_scripts = |catalog: &PackageCatalog| {
            rows.extend(catalog.scripts.iter().map(|script| {
                CatalogRow::Script(RunIntent {
                    package_root: catalog.root.clone(),
                    script_name: script.name.clone(),
                })
            }));
        };
        match &listed.catalog {
            Ok(ProjectCatalog::Package(package)) => append_scripts(package),
            Ok(ProjectCatalog::Workspace(workspace)) => {
                expanded.insert(workspace.root.clone());
                if let Some(active) = &workspace.active_package {
                    expanded.insert(active.clone());
                }
                preferred = workspace
                    .active_package
                    .as_ref()
                    .and_then(|root| workspace.packages.iter().find(|p| &p.root == root))
                    .and_then(|p| p.catalog.as_ref().ok())
                    .filter(|p| !p.scripts.is_empty())
                    .or_else(|| {
                        workspace
                            .packages
                            .iter()
                            .find(|p| p.root == workspace.root)
                            .and_then(|p| p.catalog.as_ref().ok())
                    })
                    .and_then(|p| {
                        p.scripts.first().map(|s| RunIntent {
                            package_root: p.root.clone(),
                            script_name: s.name.clone(),
                        })
                    });
                for package in &workspace.packages {
                    rows.push(CatalogRow::Group(package.root.clone()));
                    if let Ok(catalog) = &package.catalog {
                        rows.extend(catalog.scripts.iter().map(|script| {
                            CatalogRow::Script(RunIntent {
                                package_root: catalog.root.clone(),
                                script_name: script.name.clone(),
                            })
                        }));
                    }
                }
            }
            Err(_) => {}
        }
        let selected = preferred
            .and_then(|intent| {
                rows.iter()
                    .position(|row| row == &CatalogRow::Script(intent.clone()))
            })
            .unwrap_or(0);
        let mut app = Self {
            listed,
            selected,
            list_offset: 0,
            footer_offset: 0,
            run_intents: Vec::new(),
            inner: Rect::new(0, 0, 32, 24),
            process_running: true,
            workspace_id: String::new(),
            launch_error: None,
            search: SearchState::for_len(0),
            theme,
            rows,
            expanded,
        };
        app.refilter(false);
        app
    }

    pub fn catalog(&self) -> Option<&PackageCatalog> {
        let project = self.listed.catalog.as_ref().ok()?;
        self.rows
            .get(self.selected)
            .and_then(|row| project.package(row.package_root()))
            .or_else(|| project.first_package())
    }

    pub fn workspace(&self) -> Option<&WorkspaceCatalog> {
        match self.listed.catalog.as_ref().ok()? {
            ProjectCatalog::Workspace(workspace) => Some(workspace),
            ProjectCatalog::Package(_) => None,
        }
    }

    pub fn row_script(&self, index: usize) -> Option<&Script> {
        let CatalogRow::Script(intent) = self.rows.get(index)? else {
            return None;
        };
        self.listed
            .catalog
            .as_ref()
            .ok()?
            .package(&intent.package_root)?
            .scripts
            .iter()
            .find(|script| script.name == intent.script_name)
    }

    pub fn group_is_open(&self, root: &Path) -> bool {
        !self.search.query.is_empty() || self.expanded.contains(root)
    }

    pub fn selected_detail(&self) -> String {
        if let Some(script) = self.selected_script() {
            return script.command.clone();
        }
        let Some(CatalogRow::Group(root)) = self.rows.get(self.selected) else {
            return String::new();
        };
        let Some(package) = self
            .workspace()
            .and_then(|w| w.packages.iter().find(|p| &p.root == root))
        else {
            return String::new();
        };
        match &package.catalog {
            Ok(catalog) => format!(
                "{} [{}] {}{}",
                catalog.display_name,
                package.relative_path.display(),
                catalog.manager.as_str(),
                if catalog.scripts.is_empty() {
                    " - No scripts"
                } else {
                    ""
                }
            ),
            Err(error) => format!("{}: {error}", package.relative_path.display()),
        }
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

    pub fn search_available(&self) -> bool {
        self.listed.catalog.is_ok() && !self.too_small()
    }

    pub fn layout(&self) -> super::view::ColumnGeometry {
        super::view::column_layout(
            self.inner,
            self.status_lines().len(),
            self.search.is_editing(),
            self.search_available(),
            self.theme.magnifier_cols(),
            self.search_available(),
        )
    }

    pub fn scripts(&self) -> &[Script] {
        self.catalog()
            .map(|catalog| catalog.scripts.as_slice())
            .unwrap_or(&[])
    }

    pub fn visible_len(&self) -> usize {
        self.search.matches.len()
    }

    pub fn visible_pos(&self, catalog_index: usize) -> Option<usize> {
        self.search
            .matches
            .iter()
            .position(|item| item.index == catalog_index)
    }

    pub fn catalog_at_visible(&self, visible: usize) -> Option<usize> {
        self.search.matches.get(visible).map(|item| item.index)
    }

    pub fn no_match_message(&self) -> Option<String> {
        if self.search.query.is_empty() || !self.search.matches.is_empty() {
            return None;
        }
        Some(format!("No script matches \"{}\"", self.search.query))
    }

    pub fn selected_script(&self) -> Option<&Script> {
        self.visible_pos(self.selected)?;
        self.row_script(self.selected)
    }

    pub fn list_height(&self) -> usize {
        self.layout().list.height as usize
    }

    /// Reserve visible footer rows for launch errors and cwd fallback notices.
    pub fn status_lines(&self) -> Vec<String> {
        let messages: Vec<String> = self
            .launch_error
            .iter()
            .map(ToString::to_string)
            .chain(self.notes())
            .collect();
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
        let search_h = if self.search.is_editing() { 1 } else { 0 };
        // Keep a header, one result and its command before reserving status rows.
        let remaining = self.inner.height.saturating_sub(3 + search_h) as usize;
        if lines.is_empty() {
            if remaining >= 1 {
                lines.push(FOOTER_HELP.into());
            }
        } else if remaining > lines.len() {
            lines.insert(0, FOOTER_HELP.into());
        }
        lines.truncate(remaining);
        lines
    }

    pub fn ensure_visible(&mut self) {
        let height = self.list_height();
        if height == 0 {
            self.list_offset = 0;
            return;
        }
        let Some(pos) = self.visible_pos(self.selected) else {
            self.list_offset = 0;
            return;
        };
        if pos < self.list_offset {
            self.list_offset = pos;
        } else if pos >= self.list_offset + height {
            self.list_offset = pos + 1 - height;
        }
    }

    pub fn clamp_list_offset(&mut self) {
        let max = max_list_offset(self.visible_len(), self.list_height());
        if self.list_offset > max {
            self.list_offset = max;
        }
    }

    pub fn scroll_list(&mut self, delta: isize) {
        self.list_offset = bounded_list_offset(
            self.list_offset,
            delta,
            self.visible_len(),
            self.list_height(),
        );
    }

    pub fn move_selection(&mut self, delta: isize) {
        let len = self.visible_len();
        if len == 0 {
            return;
        }
        let pos = self.visible_pos(self.selected).unwrap_or(0);
        let next = (pos as isize + delta).clamp(0, len as isize - 1) as usize;
        let catalog = self.search.matches[next].index;
        if catalog != self.selected {
            self.selected = catalog;
            self.footer_offset = 0;
        }
        self.ensure_visible();
    }

    pub fn select_index(&mut self, index: usize) {
        if self.visible_pos(index).is_some() {
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
        self.ensure_visible();
        if self.selected_script().is_some()
            && let CatalogRow::Script(intent) = &self.rows[self.selected]
        {
            self.run_intents.push(intent.clone());
        }
    }

    fn toggle_selected_group(&mut self) {
        let Some(CatalogRow::Group(root)) = self.rows.get(self.selected) else {
            return;
        };
        let open = !self.expanded.contains(root);
        self.set_group_open(open);
    }

    fn set_group_open(&mut self, open: bool) {
        // Filtering temporarily opens matching groups without changing the saved tree.
        if !self.search.query.is_empty() {
            return;
        }
        let Some(row) = self.rows.get(self.selected) else {
            return;
        };
        if open && !matches!(row, CatalogRow::Group(_)) {
            return;
        }
        let root = row.package_root().to_path_buf();
        if open {
            self.expanded.insert(root);
        } else {
            self.expanded.remove(&root);
            if let Some(index) = self
                .rows
                .iter()
                .position(|row| row == &CatalogRow::Group(root.clone()))
            {
                self.selected = index;
            }
        }
        self.refilter(false);
        self.footer_offset = 0;
    }

    pub fn scroll_footer(&mut self, delta: isize) {
        let next = self.footer_offset as isize + delta;
        self.footer_offset = next.max(0) as usize;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.search.is_editing() {
            return self.handle_search_key(key);
        }
        if key.code == KeyCode::Esc && self.search.has_filter() {
            self.clear_search();
            return false;
        }
        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
            return true;
        }
        if self.too_small() || self.listed.catalog.is_err() {
            return false;
        }
        if self.is_open_search_key(&key) {
            self.open_search();
            return false;
        }
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Right if self.workspace().is_some() => self.set_group_open(true),
            KeyCode::Left if self.workspace().is_some() => self.set_group_open(false),
            KeyCode::Char('l') | KeyCode::Right => self.scroll_footer(1),
            KeyCode::Char('h') | KeyCode::Left => self.scroll_footer(-1),
            KeyCode::Enter
                if matches!(self.rows.get(self.selected), Some(CatalogRow::Group(_))) =>
            {
                self.toggle_selected_group()
            }
            KeyCode::Enter => self.emit_run(),
            _ => {}
        }
        false
    }

    fn is_open_search_key(&self, key: &KeyEvent) -> bool {
        matches!(key.code, KeyCode::Char('/'))
            || (matches!(key.code, KeyCode::Char('f') | KeyCode::Char('F'))
                && key.modifiers.contains(KeyModifiers::CONTROL))
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => self.clear_search(),
            KeyCode::Enter => self.apply_search(),
            KeyCode::Backspace => {
                self.search.query.pop();
                self.refilter(true);
            }
            KeyCode::Down => self.move_selection(1),
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.search.query.push(ch);
                self.refilter(true);
            }
            _ => {}
        }
        false
    }

    pub fn open_search(&mut self) {
        if !self.search_available() {
            return;
        }
        self.search.mode = SearchMode::Editing;
        self.ensure_visible();
    }

    fn apply_search(&mut self) {
        self.search.mode = SearchMode::Applied;
        if let Some(first) = self.first_visible_script() {
            self.selected = first;
            self.list_offset = 0;
            self.footer_offset = 0;
        }
        self.ensure_visible();
    }

    fn clear_search(&mut self) {
        self.search.mode = SearchMode::Off;
        self.search.query.clear();
        self.refilter(false);
    }

    fn refilter(&mut self, select_first: bool) {
        self.search.matches = self.filtered_rows();
        if select_first {
            if let Some(first) = self.first_visible_script() {
                self.selected = first;
            }
            self.list_offset = 0;
            self.footer_offset = 0;
        } else if self.visible_pos(self.selected).is_none() {
            let group = self.rows.get(self.selected).and_then(|selected| {
                self.rows.iter().position(
                    |row| matches!(row, CatalogRow::Group(root) if root == selected.package_root()),
                )
            });
            self.selected = group
                .filter(|index| self.visible_pos(*index).is_some())
                .or_else(|| self.search.matches.first().map(|item| item.index))
                .unwrap_or(0);
            self.list_offset = 0;
        }
        self.clamp_list_offset();
        self.ensure_visible();
    }

    fn first_visible_script(&self) -> Option<usize> {
        self.search
            .matches
            .iter()
            .find(|item| matches!(self.rows[item.index], CatalogRow::Script(_)))
            .map(|item| item.index)
    }

    fn filtered_rows(&self) -> Vec<FuzzyMatch> {
        let mut visible = Vec::new();
        let mut start = 0;
        while start < self.rows.len() {
            let group = match &self.rows[start] {
                CatalogRow::Group(root) => Some(root),
                _ => None,
            };
            let scripts_start = start + usize::from(group.is_some());
            let end = (scripts_start..self.rows.len())
                .find(|&i| matches!(self.rows[i], CatalogRow::Group(_)))
                .unwrap_or(self.rows.len());
            let names = (scripts_start..end).map(|i| match &self.rows[i] {
                CatalogRow::Script(intent) => intent.script_name.as_str(),
                _ => unreachable!(),
            });
            let matches = filter_names(names, &self.search.query);
            if let Some(root) = group {
                if self.search.query.is_empty() || !matches.is_empty() {
                    visible.push(FuzzyMatch {
                        index: start,
                        positions: Vec::new(),
                    });
                }
                if !self.group_is_open(root) {
                    start = end;
                    continue;
                }
            }
            visible.extend(matches.into_iter().map(|item| FuzzyMatch {
                index: scripts_start + item.index,
                positions: item.positions,
            }));
            start = end;
        }
        visible
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.too_small() || self.listed.catalog.is_err() {
            return;
        }
        let geo = self.layout();
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if geo.magnifier.contains(ratatui::layout::Position {
                    x: mouse.column,
                    y: mouse.row,
                }) {
                    self.open_search();
                    return;
                }
                let Some(visible) = geo.script_index(
                    mouse.column,
                    mouse.row,
                    self.list_offset,
                    self.visible_len(),
                ) else {
                    return;
                };
                let Some(index) = self.catalog_at_visible(visible) else {
                    return;
                };
                self.select_index(index);
                if matches!(self.rows[index], CatalogRow::Group(_)) {
                    self.toggle_selected_group();
                } else if geo.hits_icon_gutter(mouse.column, mouse.row) {
                    self.emit_run();
                }
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                if !geo.list.contains(ratatui::layout::Position {
                    x: mouse.column,
                    y: mouse.row,
                }) {
                    return;
                }
                let delta = if mouse.kind == MouseEventKind::ScrollDown {
                    WHEEL_LINES
                } else {
                    -WHEEL_LINES
                };
                self.scroll_list(delta);
            }
            _ => {}
        }
    }

    pub fn row_at(&self, column: u16, row: u16) -> Option<usize> {
        let visible =
            self.layout()
                .script_index(column, row, self.list_offset, self.visible_len())?;
        self.catalog_at_visible(visible)
    }

    pub fn set_inner(&mut self, inner: Rect) {
        let size_changed = inner.width != self.inner.width || inner.height != self.inner.height;
        self.inner = inner;
        if size_changed {
            self.ensure_visible();
        } else {
            self.clamp_list_offset();
        }
    }
}

pub fn max_list_offset(len: usize, height: usize) -> usize {
    if height == 0 {
        0
    } else {
        len.saturating_sub(height)
    }
}

pub fn bounded_list_offset(offset: usize, delta: isize, len: usize, height: usize) -> usize {
    let max = max_list_offset(len, height) as isize;
    (offset as isize + delta).clamp(0, max) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_list_offset_stops_at_both_ends() {
        assert_eq!(bounded_list_offset(0, -3, 40, 19), 0);
        assert_eq!(bounded_list_offset(0, 3, 40, 19), 3);
        assert_eq!(bounded_list_offset(20, 3, 40, 19), 21);
        assert_eq!(bounded_list_offset(21, 3, 40, 19), 21);
        assert_eq!(bounded_list_offset(0, 3, 3, 19), 0);
        assert_eq!(bounded_list_offset(5, 3, 40, 0), 0);
    }
}
