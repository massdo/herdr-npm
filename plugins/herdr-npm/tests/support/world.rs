use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use cucumber::World;
use herdr_npm::adapters::tui::app::SidebarApp;
use herdr_npm::application::toggle_sidebar::ToggleOutcome;
use herdr_npm::domain::error::AppError;
use herdr_npm::domain::ids::PaneId;
use herdr_npm::domain::pane::{LayoutRect, OriginContext};

use super::fake_herdr::FakeHerdr;

#[derive(Debug, World)]
pub struct BddWorld {
    pub herdr: FakeHerdr,
    pub origin: Option<OriginContext>,
    pub last_error: Option<AppError>,
    pub opened_pane: Option<PaneId>,
    pub lock_dir: PathBuf,
    pub fs_root: PathBuf,
    pub confirmed: bool,
    pub pane_ids_before: Vec<String>,
    pub focus_before: Option<String>,
    pub saved_rects: Vec<(String, LayoutRect)>,
    pub concurrent: Vec<Result<ToggleOutcome, AppError>>,
    pub recognised_ids: Vec<String>,
    pub foreign_pane: Option<String>,
    pub foreground_cwd: Option<PathBuf>,
    pub start_cwd: Option<PathBuf>,
    pub backend_width: u16,
    pub backend_height: u16,
    pub size_is_interior: bool,
    pub app: Option<SidebarApp>,
    pub screen: String,
    pub last_pkg: Option<String>,
    pub long_field: Option<String>,
    pub prev_footer_offset: usize,
    pub prev_list_offset: usize,
    pub names_at_open: Vec<String>,
    pub tui_wanted: bool,
    pub auto_launch: bool,
    pub workspace_id: String,
    pub argv_file: PathBuf,
    pub fake_bin: PathBuf,
}

impl Default for BddWorld {
    fn default() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let lock_dir = std::env::temp_dir().join(format!(
            "herdr-npm-bdd-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let fs_root = lock_dir.join("fs");
        let argv_file = lock_dir.join("argv.json");
        let fake_bin = lock_dir.join("bin");
        let _ = std::fs::create_dir_all(&fs_root);
        Self {
            herdr: FakeHerdr::default(),
            origin: None,
            last_error: None,
            opened_pane: None,
            lock_dir,
            fs_root,
            confirmed: false,
            pane_ids_before: Vec::new(),
            focus_before: None,
            saved_rects: Vec::new(),
            concurrent: Vec::new(),
            recognised_ids: Vec::new(),
            foreign_pane: None,
            foreground_cwd: None,
            start_cwd: None,
            backend_width: 32,
            backend_height: 24,
            size_is_interior: false,
            app: None,
            screen: String::new(),
            last_pkg: None,
            long_field: None,
            prev_footer_offset: 0,
            prev_list_offset: 0,
            names_at_open: Vec::new(),
            tui_wanted: false,
            auto_launch: false,
            workspace_id: "main".into(),
            argv_file,
            fake_bin,
        }
    }
}

impl BddWorld {
    pub fn map_path(&self, virtual_path: &str) -> PathBuf {
        self.fs_root.join(virtual_path.trim_start_matches('/'))
    }

    pub fn virtual_from(&self, real: &std::path::Path) -> String {
        match real.strip_prefix(&self.fs_root) {
            Ok(rest) => format!("/{}", rest.display()),
            Err(_) => real.display().to_string(),
        }
    }
}

impl Drop for BddWorld {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.lock_dir);
    }
}
