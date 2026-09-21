use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use super::app::SidebarApp;
use super::theme::{FOOTER_HELP, Theme, pad_icon};

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
    pub separator: Rect,
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
    column_layout(inner, status_line_count, false, true, 1, true)
}

pub fn column_layout(
    inner: Rect,
    status_line_count: usize,
    search_editing: bool,
    show_magnifier: bool,
    magnifier_cols: u16,
    prefer_separator: bool,
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
    let rest = inner
        .height
        .saturating_sub(header_h + search_h + command_h + status_h);
    let (separator_h, list_h) = if prefer_separator && rest >= 2 {
        (1, rest - 1)
    } else {
        (0, rest)
    };

    let header = Rect::new(inner.x, inner.y, inner.width, header_h);
    let magnifier = if show_magnifier && header.width > 0 && header.height > 0 {
        let width = magnifier_cols.min(header.width);
        Rect::new(
            header.x.saturating_add(header.width.saturating_sub(width)),
            header.y,
            width,
            header.height,
        )
    } else {
        Rect::new(0, 0, 0, 0)
    };
    let separator = Rect::new(
        inner.x,
        inner.y.saturating_add(header.height),
        inner.width,
        separator_h,
    );
    let search = Rect::new(
        inner.x,
        separator.y.saturating_add(separator.height),
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
        separator,
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
    let theme = app.theme;
    let pal = theme.palette;
    let width = inner.width as usize;
    let mag_width = geo.magnifier.width as usize;
    let mut lines: Vec<Line> = vec![header_line(catalog, width, mag_width, theme)];
    if geo.separator.height > 0 {
        lines.push(Line::styled(
            "─".repeat(width),
            Style::default().fg(pal.muted),
        ));
    }
    if geo.search.height > 0 {
        let prefix = pad_icon(theme.icons.search, ICON_GUTTER_COLS as usize);
        lines.push(Line::from(format!("{prefix}{}", app.search.query)));
    }

    let chrome = (geo.header.height + geo.separator.height + geo.search.height) as usize;
    let list_h = geo.list.height as usize;
    if let Some(message) = app.no_match_message() {
        if list_h > 0 {
            lines.push(Line::from(ellipsize(&message, width)));
        }
        while lines.len() < chrome + list_h {
            lines.push(Line::from(""));
        }
    } else {
        let visible = app.search.matches.iter().skip(app.list_offset).take(list_h);
        for item in visible {
            let script = &catalog.scripts[item.index];
            let selected = item.index == app.selected;
            lines.push(format_row(script, width, &item.positions, theme, selected));
        }
        while lines.len() < chrome + list_h {
            lines.push(Line::from(""));
        }
    }

    let command = app
        .selected_script()
        .map(|script| script.command.as_str())
        .unwrap_or("");
    let footer = scroll_text(command, app.footer_offset, width);
    lines.push(Line::styled(footer, Style::default().fg(pal.muted)));
    lines.extend(
        app.status_lines()
            .into_iter()
            .map(|line| status_line(&line, theme, width)),
    );
    frame.render_widget(Paragraph::new(lines), inner);
    if app.search.is_editing() && geo.search.height > 0 {
        let cursor_x = geo.search.x.saturating_add(
            ICON_GUTTER_COLS
                + app
                    .search
                    .query
                    .width()
                    .min(geo.search.width.saturating_sub(ICON_GUTTER_COLS) as usize)
                    as u16,
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

fn header_line(
    catalog: &crate::domain::catalog::PackageCatalog,
    width: usize,
    mag_width: usize,
    theme: Theme,
) -> Line<'static> {
    let pal = theme.palette;
    let pkg = pad_icon(theme.icons.package, ICON_GUTTER_COLS as usize);
    let manager = catalog.manager.as_str();
    let count = catalog.scripts.len().to_string();
    let suffix = ellipsize(
        &format!("  {manager} {count}"),
        width.saturating_sub(pkg.width() + mag_width),
    );
    let name_budget = width.saturating_sub(pkg.width() + suffix.width() + mag_width);
    let name = ellipsize(&catalog.display_name, name_budget);
    let mut spans = vec![
        Span::styled(pkg, Style::default().fg(pal.accent)),
        Span::styled(
            name,
            Style::default().fg(pal.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(suffix, Style::default().fg(pal.muted)),
    ];
    if mag_width > 0 {
        let used = spans.iter().map(|span| span.content.width()).sum::<usize>();
        if used + mag_width < width {
            spans.push(Span::raw(" ".repeat(width - used - mag_width)));
        }
        spans.push(Span::styled(
            theme.icons.search.to_string(),
            Style::default().fg(pal.muted),
        ));
    }
    Line::from(spans)
}

fn status_line(text: &str, theme: Theme, width: usize) -> Line<'static> {
    if text == FOOTER_HELP {
        footer_help_line(theme, width)
    } else {
        Line::styled(text.to_string(), Style::default().fg(theme.palette.footer))
    }
}

fn footer_help_line(theme: Theme, width: usize) -> Line<'static> {
    let pal = theme.palette;
    let cap = |key: &'static str| {
        Span::styled(
            format!(" {key} "),
            Style::default().bg(pal.keycap_bg).fg(pal.keycap_fg),
        )
    };
    let mut spans = Vec::new();
    let mut used = 0usize;
    let push = |spans: &mut Vec<Span<'static>>, used: &mut usize, span: Span<'static>| {
        let w = span.content.width();
        if *used + w <= width {
            *used += w;
            spans.push(span);
        }
    };
    push(&mut spans, &mut used, cap("j"));
    push(&mut spans, &mut used, cap("k"));
    push(&mut spans, &mut used, Span::raw(" "));
    push(&mut spans, &mut used, cap("h"));
    push(&mut spans, &mut used, cap("l"));
    push(&mut spans, &mut used, Span::raw(" "));
    push(&mut spans, &mut used, cap("enter"));
    push(&mut spans, &mut used, Span::raw(" "));
    push(&mut spans, &mut used, cap("/"));
    Line::from(spans)
}

fn format_row(
    script: &crate::domain::catalog::Script,
    width: usize,
    highlights: &[usize],
    theme: Theme,
    selected: bool,
) -> Line<'static> {
    let pal = theme.palette;
    let gutter = ICON_GUTTER_COLS as usize;
    let rest = width.saturating_sub(gutter);
    let name_budget = (rest / 3).max(1);
    let name_fg = if selected { pal.selection_fg } else { pal.text };
    let (name_spans, name_width) = highlight_name(&script.name, highlights, name_budget, name_fg);
    let used = gutter + name_width;
    let cmd_budget = width.saturating_sub(used + 1);
    let command = ellipsize(&script.command, cmd_budget);
    let bg = selected.then_some(pal.selection_bg);
    let with_bg = |style: Style| match bg {
        Some(color) => style.bg(color),
        None => style,
    };
    let play = pad_icon(theme.icons.play, gutter);
    let mut spans = vec![Span::styled(play, with_bg(Style::default().fg(pal.accent)))];
    spans.extend(
        name_spans
            .into_iter()
            .map(|span| Span::styled(span.content, with_bg(span.style))),
    );
    spans.push(Span::styled(
        format!(" {command}"),
        with_bg(Style::default().fg(pal.muted)),
    ));
    let painted = spans.iter().map(|span| span.content.width()).sum::<usize>();
    if selected && painted < width {
        spans.push(Span::styled(
            " ".repeat(width - painted),
            Style::default().bg(pal.selection_bg).fg(pal.selection_fg),
        ));
    }
    Line::from(spans)
}

fn highlight_name(
    name: &str,
    highlights: &[usize],
    budget: usize,
    fg: ratatui::style::Color,
) -> (Vec<Span<'static>>, usize) {
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
        let mut style = Style::default().fg(fg);
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
        if used < budget {
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
    use crate::application::list_scripts::ListedScripts;
    use crate::domain::catalog::{PackageCatalog, PackageManager, Script};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Modifier;

    use super::super::app::SearchMode;
    use super::super::theme::PALETTE;

    fn listed(scripts: &[(&str, &str)]) -> ListedScripts {
        ListedScripts {
            catalog: Ok(PackageCatalog {
                root: std::path::PathBuf::from("/work/app"),
                display_name: "app".into(),
                manager: PackageManager::Npm,
                scripts: scripts
                    .iter()
                    .map(|(name, command)| Script {
                        name: (*name).into(),
                        command: (*command).into(),
                    })
                    .collect(),
            }),
            used_start_cwd: false,
            root: Some(std::path::PathBuf::from("/work/app")),
        }
    }

    fn paint(
        theme: Theme,
        width: u16,
        height: u16,
        extra: impl FnOnce(&mut SidebarApp),
    ) -> (SidebarApp, Terminal<TestBackend>) {
        let mut app = SidebarApp::with_theme(
            listed(&[
                ("dev", "vite"),
                ("build", "tsc && vite build"),
                ("test", "vitest run"),
            ]),
            theme,
        );
        extra(&mut app);
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("backend");
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("draw");
        (app, terminal)
    }

    #[test]
    fn narrow_header_keeps_the_search_icon_inside_its_hit_target() {
        for count in [40, 400] {
            let mut catalog = listed(&[("dev", "vite")]).catalog.unwrap();
            catalog.manager = PackageManager::Pnpm;
            catalog.scripts = vec![catalog.scripts[0].clone(); count];
            let mut terminal = Terminal::new(TestBackend::new(12, 1)).unwrap();
            terminal
                .draw(|frame| {
                    frame.render_widget(
                        Paragraph::new(header_line(&catalog, 12, 1, Theme::ascii())),
                        frame.area(),
                    );
                })
                .unwrap();
            assert_eq!(terminal.backend().buffer()[(11, 0)].symbol(), "/");
        }
    }

    #[test]
    fn opening_search_keeps_the_last_visible_selection_on_screen() {
        let (mut app, mut terminal) = paint(Theme::ascii(), 32, 8, |_| {});
        app.select_index(1);
        app.open_search();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let pos = app.visible_pos(app.selected).unwrap();
        assert!(pos >= app.list_offset && pos < app.list_offset + app.list_height());
    }

    #[test]
    fn search_keeps_a_result_row_at_the_minimum_supported_height() {
        let (mut app, mut terminal) = paint(Theme::ascii(), 32, 6, |_| {});
        app.open_search();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        assert!(app.list_height() > 0);
        let geo = app.layout();
        assert_eq!(
            terminal.backend().buffer()[(geo.list.x, geo.list.y)].symbol(),
            ">"
        );
    }

    #[test]
    fn icon_gutter_stops_before_the_script_name() {
        let inner = Rect::new(1, 1, 30, 22);
        let geo = column_geometry(inner, 1);
        let row = geo.list.y;
        let last_icon = inner.x + ICON_GUTTER_COLS - 1;
        let first_name = inner.x + ICON_GUTTER_COLS;
        assert_eq!(geo.separator.height, 1);
        assert_eq!(
            geo.list.y,
            inner.y + geo.header.height + geo.separator.height
        );
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
            Theme::ascii(),
            false,
        );
        let text: String = rendered
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        let gutter = pad_icon(Theme::ascii().icons.play, ICON_GUTTER_COLS as usize);
        assert!(
            text.starts_with(&gutter),
            "formatted gutter is two cells: glyph plus spacer, got {text:?}"
        );
        assert_eq!(gutter.width(), ICON_GUTTER_COLS as usize);
    }

    #[test]
    fn separator_yields_to_the_list() {
        let inner = Rect::new(1, 1, 30, 4);
        let geo = column_layout(inner, 1, false, true, 1, true);
        assert_eq!(geo.separator.height, 0);
        assert_eq!(geo.list.height, 1);
    }

    #[test]
    fn search_field_sits_below_the_header_separator() {
        let inner = Rect::new(1, 1, 30, 22);
        let geo = column_layout(inner, 1, true, true, 1, true);
        assert_eq!(geo.separator.height, 1);
        assert_eq!(geo.search.height, 1);
        assert_eq!(geo.separator.y, inner.y + geo.header.height);
        assert_eq!(
            geo.search.y,
            geo.separator.y + geo.separator.height,
            "search must not sit between the header and its separator"
        );
        assert_eq!(geo.list.y, geo.search.y + geo.search.height);

        let (app, terminal) = paint(Theme::ascii(), 32, 24, |app| {
            app.search.mode = SearchMode::Editing;
            app.search.query = "bu".into();
        });
        let geo = app.layout();
        let buffer = terminal.backend().buffer();
        let sep: String = (geo.separator.x..geo.separator.x + geo.separator.width)
            .map(|x| buffer[(x, geo.separator.y)].symbol().to_string())
            .collect();
        assert!(
            sep.contains('─'),
            "separator row should be painted at geo.separator.y, got {sep:?}"
        );
        let field: String = (geo.search.x..geo.search.x + geo.search.width)
            .map(|x| buffer[(x, geo.search.y)].symbol().to_string())
            .collect();
        assert!(
            field.contains("bu"),
            "query should be painted at geo.search.y, got {field:?}"
        );
    }

    #[test]
    fn both_icon_sets_keep_gutter_alignment_and_selection_style() {
        for theme in [Theme::ascii(), Theme::nerd()] {
            let (app, terminal) = paint(theme, 32, 24, |_| {});
            let geo = app.layout();
            assert_eq!(geo.icon_gutter_width, ICON_GUTTER_COLS);
            assert_eq!(geo.separator.height, 1);
            assert!(geo.hits_icon_gutter(geo.list.x, geo.list.y));
            assert!(geo.hits_icon_gutter(geo.list.x + ICON_GUTTER_COLS - 1, geo.list.y));
            assert!(!geo.hits_icon_gutter(geo.list.x + ICON_GUTTER_COLS, geo.list.y));

            let cell = &terminal.backend().buffer()[(geo.list.x, geo.list.y)];
            assert_eq!(cell.symbol(), theme.icons.play);
            assert_eq!(cell.fg, PALETTE.accent);
            assert_eq!(cell.bg, PALETTE.selection_bg);
            assert!(!cell.modifier.contains(Modifier::REVERSED));

            let name = &terminal.backend().buffer()[(geo.list.x + ICON_GUTTER_COLS, geo.list.y)];
            assert_eq!(name.bg, PALETTE.selection_bg);
            assert_eq!(name.fg, PALETTE.selection_fg);
            assert!(!name.modifier.contains(Modifier::REVERSED));
        }
    }

    #[test]
    fn long_text_and_search_keep_alignment_on_a_tight_pane() {
        let name = "n".repeat(80);
        let command = "x".repeat(80);
        let mut app =
            SidebarApp::with_theme(listed(&[(name.as_str(), command.as_str())]), Theme::nerd());
        let mut terminal = Terminal::new(TestBackend::new(32, 24)).expect("backend");
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("draw");
        let geo = app.layout();
        let row: String = (geo.list.x..geo.list.x + geo.list.width)
            .map(|x| {
                terminal.backend().buffer()[(x, geo.list.y)]
                    .symbol()
                    .to_string()
            })
            .collect();
        assert!(
            row.contains('…'),
            "long name/command should ellipsize: {row:?}"
        );
        let gutter = pad_icon(Theme::nerd().icons.play, ICON_GUTTER_COLS as usize);
        assert!(
            row.starts_with(&gutter),
            "nerd gutter must stay two cells, got {row:?}"
        );

        let (app, _) = paint(Theme::ascii(), 14, 7, |app| {
            app.search.mode = SearchMode::Editing;
        });
        let geo = app.layout();
        assert_eq!(geo.search.height, 1);
        assert_eq!(geo.separator.height, 0);
        assert!(geo.list.height > 0);
    }
}
