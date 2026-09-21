use crate::application::ports::HerdrPort;
use crate::domain::error::AppError;
use crate::domain::ids::PaneId;

/// Close the TUI's own managed pane. Used by `q`.
pub fn close_own_pane<H: HerdrPort>(herdr: &H, pane_id: &PaneId) -> Result<(), AppError> {
    herdr.close_plugin_pane(pane_id)
}
