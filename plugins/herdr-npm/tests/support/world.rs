use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use cucumber::World;
use herdr_npm::application::toggle_sidebar::ToggleOutcome;
use herdr_npm::domain::error::AppError;
use herdr_npm::domain::ids::PaneId;
use herdr_npm::domain::pane::{LayoutRect, OriginContext};

use super::fake_herdr::FakeHerdr;
use super::fake_project::FakeProject;

#[derive(Debug, World)]
pub struct BddWorld {
    pub herdr: FakeHerdr,
    #[allow(dead_code)]
    pub project: FakeProject,
    pub origin: Option<OriginContext>,
    pub last_error: Option<AppError>,
    pub opened_pane: Option<PaneId>,
    pub closed_pane: Option<PaneId>,
    pub plugin_id: Option<String>,
    pub lock_dir: PathBuf,
    pub confirmed: bool,
    pub pane_ids_before: Vec<String>,
    pub focus_before: Option<String>,
    pub saved_rects: Vec<(String, LayoutRect)>,
    pub concurrent: Vec<Result<ToggleOutcome, AppError>>,
    pub recognised_ids: Vec<String>,
    pub foreign_pane: Option<String>,
    pub package_json_reads: u32,
}

impl Default for BddWorld {
    fn default() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let lock_dir = std::env::temp_dir().join(format!(
            "herdr-npm-bdd-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::create_dir_all(&lock_dir);
        Self {
            herdr: FakeHerdr::default(),
            project: FakeProject,
            origin: None,
            last_error: None,
            opened_pane: None,
            closed_pane: None,
            plugin_id: None,
            lock_dir,
            confirmed: false,
            pane_ids_before: Vec::new(),
            focus_before: None,
            saved_rects: Vec::new(),
            concurrent: Vec::new(),
            recognised_ids: Vec::new(),
            foreign_pane: None,
            package_json_reads: 0,
        }
    }
}
