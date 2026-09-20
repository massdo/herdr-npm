use std::path::Path;

use herdr_npm::application::ports::{LoadedCatalog, ProjectPort};
use herdr_npm::domain::error::AppError;

/// Filesystem fake kept for toggle scenarios that never load a catalogue.
#[derive(Debug, Default)]
pub struct FakeProject;

impl ProjectPort for FakeProject {
    fn load_catalog(&self, start_dir: &Path) -> LoadedCatalog {
        let _ = start_dir;
        LoadedCatalog::from_error(AppError::herdr(
            "project.load_catalog",
            "not_implemented",
            "catalog tests use FsProject, not FakeProject",
        ))
    }
}
