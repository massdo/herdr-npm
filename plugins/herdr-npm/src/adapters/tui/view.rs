use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use super::app::{MAGNIFIER_ICON, PLAY_ICON, SidebarApp};
use crate::domain::error::AppError;

/// Play glyph plus the following spacer. Shared by `format_row`, `row_zone`,
/// and the mouse hit-test; do not hard-code `1` on one side only.
pub const ICON_GUTTER_COLS: u16 = 2;

/// Rectangles actually painted inside the pane border.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnGeometry {
    pub inner: Rect,
    pub header: Rect,
    pub magnifier: Rect,
    pub search: Rect,
    pub list: Rect,
    pub command: Rect,
    pub status: Rect,
    pub icon_gutter_width: u16,
}

pub fn ellipsize(text: &str, max_cells: usize) -> String {
    if max_cells == 0 {
        return String::new();
    }
    if text.width() <= max_cells {
        return text.to_string();
    }
    if max_cells == 1 {
        return "…".into();
    }
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + w > max_cells - 1 {
            break;
        }
        out.push(ch);
        used += w;
    }
    out.push('…');
    out
}

pub fn column_geometry(inner: Rect, status_line_count: usize) -> ColumnGeometry {
    column_layout(inner, status_line_count, false, true)
}

pub fn column_layout(
    inner: Rect,
    status_line_count: usize,
    search_editing: bool,
    show_magnifier: bool,
) -> ColumnGeometry {
    let header_h = 1u16.min(inner.height);
    let search_h = if search_editing {
        1u16.min(inner.height.saturating_sub(header_h))
    } else {
        0
    };
    let command_h = 1u16.min(inner.height.saturating_sub(header_h + search_h));
    let status_h = (status_line_count as u16)
        .min(inner.height.saturating_sub(header_h + search_h + command_h));
    let list_h = inner
        .height
        .saturating_sub(header_h + search_h + command_h + status_h);

    let header = Rect::new(inner.x, inner.y, inner.width, header_h);
    let magnifier = if show_magnifier && header.width > 0 && header.height > 0 {
        let width = MAGNIFIER_ICON.width().min(header.width as usize) as u16;
        Rect::new(
            header.x.saturating_add(header.width.saturating_sub(width)),
            header.y,
            width,
            header.height,
        )
    } else {
        Rect::new(0, 0, 0, 0)
    };
    let search = Rect::new(
        inner.x,
        inner.y.saturating_add(header.height),
        inner.width,
        search_h,
    );
    let list = Rect::new(
        inner.x,
        search.y.saturating_add(search.height),
        inner.width,
        list_h,
    );
    let command = Rect::new(
        inner.x,
        list.y.saturating_add(list.height),
        inner.width,
        command_h,
    );
    let status = Rect::new(
        inner.x,
        command.y.saturating_add(command.height),
        inner.width,
        status_h,
    );
    ColumnGeometry {
        inner,
        header,
        magnifier,
        search,
        list,
        command,
        status,
        icon_gutter_width: ICON_GUTTER_COLS,
    }
}

impl ColumnGeometry {
    pub fn script_index(
        &self,
        column: u16,
        row: u16,
        list_offset: usize,
        script_count: usize,
    ) -> Option<usize> {
        if !self
            .list
            .contains(ratatui::layout::Position { x: column, y: row })
        {
            return None;
        }
        let index = list_offset + (row - self.list.y) as usize;
        (index < script_count).then_some(index)
    }

    pub fn hits_icon_gutter(&self, column: u16, row: u16) -> bool {
        if row < self.list.y || row >= self.list.y.saturating_add(self.list.height) {
            return false;
        }
        column >= self.list.x && column < self.list.x.saturating_add(self.icon_gutter_width)
    }
}

pub fn render(frame: &mut Frame, app: &mut SidebarApp) {
    let area = frame.area();
    let block = Block::default().borders(Borders::ALL).title("npm");
    let inner = block.inner(area);
    app.set_inner(inner);
    frame.render_widget(block, area);

    if app.too_small() {
        let msg = Paragraph::new(AppError::TerminalTooSmall.to_string())
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false });
        frame.render_widget(msg, inner);
        return;
    }

    if let Err(error) = &app.listed.catalog {
        let mut lines = vec![Line::from(error.to_string())];
        for note in app.notes() {
            lines.push(Line::from(note));
        }
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
        return;
    }

    let catalog = app.catalog().expect("ok catalog");
    let geo = app.layout();
    let mut lines: Vec<Line> = Vec::new();
    let mag_width = if geo.magnifier.width == 0 {
        0
    } else {
        MAGNIFIER_ICON.width()
    };
    let header_budget = (inner.width as usize).saturating_sub(mag_width);
    let header = format!("{}  {}", catalog.display_name, catalog.manager.as_str());
    let mut header_spans = vec![Span::styled(
        ellipsize(&header, header_budget),
        Style::default().add_modifier(Modifier::BOLD),
    )];
    if mag_width > 0 {
        let used = header_spans
            .iter()
            .map(|span| span.content.width())
            .sum::<usize>();
        if used < inner.width as usize {
            header_spans.push(Span::raw(
                " ".repeat(inner.width as usize - used - mag_width),
            ));
        }
        header_spans.push(Span::raw(MAGNIFIER_ICON));
    }
    lines.push(Line::from(header_spans));
    if geo.search.height > 0 {
        lines.push(Line::from(format!("/ {}", app.search.query)));
    }

    let list_h = geo.list.height as usize;
    if let Some(message) = app.no_match_message() {
        if list_h > 0 {
            lines.push(Line::from(ellipsize(&message, inner.width as usize)));
        }
        while lines.len() < 1 + geo.search.height as usize + list_h {
            lines.push(Line::from(""));
        }
    } else {
        let visible = app.search.matches.iter().skip(app.list_offset).take(list_h);
        for item in visible {
            let script = &catalog.scripts[item.index];
            let selected = item.index == app.selected;
            let row = format_row(script, inner.width as usize, &item.positions);
            let style = if selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            lines.push(row.patch_style(style));
        }
        while lines.len() < 1 + geo.search.height as usize + list_h {
            lines.push(Line::from(""));
        }
    }

    let command = app
        .selected_script()
        .map(|script| script.command.as_str())
        .unwrap_or("");
    let footer = scroll_text(command, app.footer_offset, inner.width as usize);
    lines.push(Line::from(footer));
    lines.extend(app.status_lines().into_iter().map(Line::from));
    frame.render_widget(Paragraph::new(lines), inner);
    if app.search.is_editing() && geo.search.height > 0 {
        let cursor_x = geo.search.x.saturating_add(
            2 + app
                .search
                .query
                .width()
                .min(geo.search.width.saturating_sub(2) as usize) as u16,
        );
        frame.set_cursor_position((
            cursor_x.min(
                geo.search
                    .x
                    .saturating_add(geo.search.width.saturating_sub(1)),
            ),
            geo.search.y,
        ));
    }
}

fn format_row(
    script: &crate::domain::catalog::Script,
    width: usize,
    highlights: &[usize],
) -> Line<'static> {
    let gutter = ICON_GUTTER_COLS as usize;
    let rest = width.saturating_sub(gutter);
    let name_budget = (rest / 3).max(1);
    let (name_spans, name_width) = highlight_name(&script.name, highlights, name_budget);
    let used = gutter + name_width;
    let cmd_budget = width.saturating_sub(used + 1);
    let command = ellipsize(&script.command, cmd_budget);
    let mut spans = vec![Span::raw(format!("{PLAY_ICON} "))];
    spans.extend(name_spans);
    spans.push(Span::raw(format!(" {command}")));
    Line::from(spans)
}

fn highlight_name(name: &str, highlights: &[usize], budget: usize) -> (Vec<Span<'static>>, usize) {
    if budget == 0 {
        return (Vec::new(), 0);
    }
    let marked: std::collections::HashSet<usize> = highlights.iter().copied().collect();
    let mut spans = Vec::new();
    let mut used = 0;
    for (index, ch) in name.chars().enumerate() {
        let width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + width > budget {
            break;
        }
        let mut style = Style::default();
        if marked.contains(&index) {
            style = style.add_modifier(Modifier::BOLD);
        }
        spans.push(Span::styled(ch.to_string(), style));
        used += width;
    }
    if name.width() > budget {
        if budget == 1 {
            return (vec![Span::raw("…")], 1);
        }
        while used >= budget {
            let Some(last) = spans.pop() else {
                break;
            };
            used = used.saturating_sub(last.content.width());
        }
        if used + 1 <= budget {
            spans.push(Span::raw("…"));
            used += 1;
        }
    }
    (spans, used)
}

fn scroll_text(text: &str, offset: usize, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut skipped = 0usize;
    let mut start = 0usize;
    for (index, ch) in text.char_indices() {
        if skipped >= offset {
            start = index;
            return ellipsize(&text[start..], width);
        }
        skipped += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        start = index + ch.len_utf8();
    }
    ellipsize(&text[start..], width)
}

pub fn row_zone(inner: Rect, column: u16) -> &'static str {
    let x = column.saturating_sub(inner.x);
    if x < ICON_GUTTER_COLS {
        "play icon"
    } else if x < inner.width / 3 {
        "script name"
    } else if x < inner.width.saturating_sub(2) {
        "command text"
    } else {
        "trailing space"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::catalog::Script;

    #[test]
    fn icon_gutter_stops_before_the_script_name() {
        let inner = Rect::new(1, 1, 30, 22);
        let geo = column_geometry(inner, 1);
        let row = geo.list.y;
        let last_icon = inner.x + ICON_GUTTER_COLS - 1;
        let first_name = inner.x + ICON_GUTTER_COLS;
        assert_eq!(geo.list.y, inner.y + 1);
        assert!(geo.hits_icon_gutter(inner.x, row));
        assert!(geo.hits_icon_gutter(last_icon, row));
        assert!(!geo.hits_icon_gutter(first_name, row));
        assert_eq!(
            geo.script_index(first_name, row, 0, 3),
            Some(0),
            "name column is still on the first list row"
        );

        let rendered = format_row(
            &Script {
                name: "test".into(),
                command: "vitest".into(),
            },
            inner.width as usize,
            &[],
        );
        let text: String = rendered
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(
            text.starts_with(&format!("{PLAY_ICON} ")),
            "formatted gutter is two cells: glyph plus spacer, got {text:?}"
        );
        assert_eq!(PLAY_ICON.width() + 1, ICON_GUTTER_COLS as usize);
    }
}
