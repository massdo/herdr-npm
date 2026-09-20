use std::io::{self, stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Alignment;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::adapters::env::{ProcessEnv, TuiOrigin};
use crate::adapters::herdr_socket::HerdrSocket;
use crate::application::close_own_pane::close_own_pane;
use crate::domain::SIDEBAR_LABEL;
use crate::domain::error::AppError;

/// Empty TUI. Catalogue rendering belongs to a later lot. `q` closes the pane.
pub fn run(process: ProcessEnv) -> Result<(), AppError> {
    let mut terminal = setup()?;
    let result = event_loop(&mut terminal, &process.tui_origin);
    let _ = teardown(&mut terminal);
    if let Some(pane_id) = process.own_pane_id {
        let herdr = HerdrSocket::new(process.socket_path);
        let _ = close_own_pane(&herdr, &pane_id);
    }
    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    origin: &TuiOrigin,
) -> Result<(), AppError> {
    loop {
        terminal.draw(|frame| {
            let cwd = origin
                .foreground_cwd
                .as_ref()
                .or(origin.cwd.as_ref())
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "(no origin cwd)".into());
            let body = Paragraph::new(vec![
                Line::from(Span::styled(
                    SIDEBAR_LABEL,
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(cwd),
                Line::from("q closes"),
            ])
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL).title("npm"));
            frame.render_widget(body, frame.area());
        })?;
        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        {
            break;
        }
    }
    Ok(())
}

fn setup() -> Result<Terminal<CrosstermBackend<io::Stdout>>, AppError> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout)).map_err(AppError::from)
}

fn teardown(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
