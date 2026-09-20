use std::path::Path;

use herdr_npm::application::ports::ProjectPort;
use herdr_npm::domain::catalog::PackageCatalog;
use herdr_npm::domain::error::AppError;

/// Filesystem fake. Catalogue loading is implemented in the catalog lot.
#[derive(Debug, Default)]
pub struct FakeProject;

impl ProjectPort for FakeProject {
    fn load_catalog(&self, start_dir: &Path) -> Result<PackageCatalog, AppError> {
        let _ = start_dir;
        Err(AppError::herdr(
            "project.load_catalog",
            "not_implemented",
            "catalog lot has not implemented package.json reading yet",
        ))
    }
}
