use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::domain::catalog::PackageCatalog;
use crate::domain::error::AppError;
use crate::domain::ids::PaneId;
use crate::domain::pane::{CreatedTab, LayoutSnapshot, OpenedPane, PaneInfo};

/// Herdr socket operations the application needs. The OS lock is an adapter
/// detail, not a port.
pub trait HerdrPort {
    fn list_panes(&self, workspace_id: Option<&str>) -> Result<Vec<PaneInfo>, AppError>;
    fn pane_layout(&self, pane_id: &PaneId) -> Result<LayoutSnapshot, AppError>;
    fn open_plugin_pane(&self, request: OpenPluginPane) -> Result<OpenedPane, AppError>;
    fn swap_panes(&self, source: &PaneId, target: &PaneId) -> Result<(), AppError>;
    fn focus_pane(&self, pane_id: &PaneId) -> Result<(), AppError>;
    fn resize_pane(&self, pane_id: &PaneId, direction: &str, amount: f64) -> Result<(), AppError>;
    fn report_sidebar_identity(&self, pane_id: &PaneId) -> Result<(), AppError>;
    fn close_plugin_pane(&self, pane_id: &PaneId) -> Result<(), AppError>;
    fn create_tab(&self, request: CreateTab) -> Result<CreatedTab, AppError>;
    fn send_input(&self, pane_id: &PaneId, text: &str, keys: &[&str]) -> Result<(), AppError>;
}

#[derive(Debug, Clone)]
pub struct OpenPluginPane {
    pub plugin_id: String,
    pub entrypoint: String,
    pub target_pane_id: PaneId,
    pub workspace_id: String,
    pub focus: bool,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct CreateTab {
    pub workspace_id: String,
    pub cwd: PathBuf,
    pub label: String,
    pub focus: bool,
}

/// Filesystem access for package.json. Unimplemented in the skeleton lot.
pub trait ProjectPort {
    fn load_catalog(&self, start_dir: &Path) -> Result<PackageCatalog, AppError>;
}
