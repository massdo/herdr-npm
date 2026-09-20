use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use super::app::{PLAY_ICON, SidebarApp};
use crate::domain::error::AppError;

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
    let mut lines: Vec<Line> = Vec::new();
    let header = format!("{}  {}", catalog.display_name, catalog.manager.as_str());
    lines.push(Line::from(Span::styled(
        ellipsize(&header, inner.width as usize),
        Style::default().add_modifier(Modifier::BOLD),
    )));

    let list_h = app.list_height();
    let scripts = &catalog.scripts;
    let visible = scripts
        .iter()
        .enumerate()
        .skip(app.list_offset)
        .take(list_h);
    for (index, script) in visible {
        let selected = index == app.selected;
        let row = format_row(script, inner.width as usize);
        let style = if selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(row, style)));
    }
    while lines.len() < 1 + list_h {
        lines.push(Line::from(""));
    }

    let command = app
        .selected_script()
        .map(|script| script.command.as_str())
        .unwrap_or("");
    let footer = scroll_text(command, app.footer_offset, inner.width as usize);
    lines.push(Line::from(footer));
    lines.push(Line::from("h/l scroll"));
    for note in app.notes() {
        if lines.len() < inner.height as usize {
            lines.push(Line::from(note));
        }
    }
    if let Some(error) = &app.launch_error
        && lines.len() < inner.height as usize
    {
        lines.push(Line::from(error.to_string()));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn format_row(script: &crate::domain::catalog::Script, width: usize) -> String {
    let icon = PLAY_ICON;
    let rest = width.saturating_sub(icon.width() + 1);
    let name_budget = (rest / 3).max(1);
    let name = ellipsize(&script.name, name_budget);
    let used = icon.width() + 1 + name.width();
    let cmd_budget = width.saturating_sub(used + 1);
    let command = ellipsize(&script.command, cmd_budget);
    format!("{icon} {name} {command}")
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
    if x <= 1 {
        "play icon"
    } else if x < inner.width / 3 {
        "script name"
    } else if x < inner.width.saturating_sub(2) {
        "command text"
    } else {
        "trailing space"
    }
}
