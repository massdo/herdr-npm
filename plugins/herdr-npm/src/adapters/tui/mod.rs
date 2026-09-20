pub mod app;
pub mod keymap;
pub mod view;

use std::io::{self, stdout};
use std::time::Duration;

use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::adapters::env::ProcessEnv;
use crate::adapters::fs_project::FsProject;
use crate::adapters::herdr_socket::HerdrSocket;
use crate::application::close_own_pane::close_own_pane;
use crate::application::list_scripts::{list_scripts, origin_for_paths};
use crate::domain::error::AppError;

use self::app::SidebarApp;

/// Catalogue TUI. `q` closes the pane.
pub fn run(process: ProcessEnv) -> Result<(), AppError> {
    let listed = list_scripts(
        &FsProject::new(),
        &origin_for_paths(
            process.tui_origin.foreground_cwd.clone(),
            process.tui_origin.cwd.clone(),
        ),
    );
    let mut app = SidebarApp::new(listed);
    let mut terminal = setup()?;
    let result = event_loop(&mut terminal, &mut app);
    let _ = teardown(&mut terminal);
    if let Some(pane_id) = process.own_pane_id {
        let herdr = HerdrSocket::new(process.socket_path);
        let _ = close_own_pane(&herdr, &pane_id);
    }
    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut SidebarApp,
) -> Result<(), AppError> {
    loop {
        terminal.draw(|frame| view::render(frame, app))?;
        if event::poll(Duration::from_millis(200))? {
            let event = event::read()?;
            if matches!(event, Event::Resize(_, _)) {
                keymap::handle_event(app, event);
                continue;
            }
            if keymap::handle_event(app, event) {
                break;
            }
        }
    }
    Ok(())
}

fn setup() -> Result<Terminal<CrosstermBackend<io::Stdout>>, AppError> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(stdout)).map_err(AppError::from)
}

fn teardown(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    disable_raw_mode()?;
    terminal.show_cursor()?;
    Ok(())
}
