use std::path::PathBuf;

use crate::application::ports::{LoadedCatalog, ProjectPort};
use crate::domain::catalog::PackageCatalog;
use crate::domain::error::AppError;
use crate::domain::pane::OriginContext;

#[derive(Debug, Clone)]
pub struct ListedScripts {
    pub catalog: Result<PackageCatalog, AppError>,
    pub used_start_cwd: bool,
    pub root: Option<PathBuf>,
}

/// Resolve the project once from the origin pane, then freeze it.
pub fn list_scripts<P: ProjectPort>(project: &P, origin: &OriginContext) -> ListedScripts {
    match (&origin.foreground_cwd, &origin.cwd) {
        (Some(foreground), _) => from_loaded(project.load_catalog(foreground), false),
        (None, Some(start)) => from_loaded(project.load_catalog(start), true),
        (None, None) => ListedScripts {
            catalog: Err(AppError::CannotDetermineProjectDir),
            used_start_cwd: false,
            root: None,
        },
    }
}

fn from_loaded(loaded: LoadedCatalog, used_start_cwd: bool) -> ListedScripts {
    ListedScripts {
        catalog: loaded.catalog,
        used_start_cwd,
        root: loaded.root,
    }
}

pub fn origin_for_paths(foreground: Option<PathBuf>, start: Option<PathBuf>) -> OriginContext {
    OriginContext {
        workspace_id: crate::domain::ids::WorkspaceId("w1".into()),
        tab_id: crate::domain::ids::TabId("t1".into()),
        pane_id: crate::domain::ids::PaneId("p1".into()),
        foreground_cwd: foreground,
        cwd: start,
    }
}
