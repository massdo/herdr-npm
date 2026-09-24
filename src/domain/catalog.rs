use std::path::{Path, PathBuf};

use super::error::AppError;

/// Recognised package managers in V1. yarn/bun declarations resolve to Npm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Npm,
    Pnpm,
}

impl PackageManager {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageCatalog {
    pub root: PathBuf,
    pub display_name: String,
    pub manager: PackageManager,
    pub scripts: Vec<Script>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunIntent {
    pub script_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePackage {
    pub root: PathBuf,
    pub relative_path: PathBuf,
    pub catalog: Result<PackageCatalog, AppError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCatalog {
    pub root: PathBuf,
    pub packages: Vec<WorkspacePackage>,
    pub active_package: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectCatalog {
    Package(PackageCatalog),
    Workspace(WorkspaceCatalog),
}

impl ProjectCatalog {
    pub fn first_package(&self) -> Option<&PackageCatalog> {
        match self {
            Self::Package(package) => Some(package),
            Self::Workspace(workspace) => workspace
                .packages
                .iter()
                .find_map(|p| p.catalog.as_ref().ok()),
        }
    }

    pub fn package(&self, root: &Path) -> Option<&PackageCatalog> {
        match self {
            Self::Package(package) => (package.root == root).then_some(package),
            Self::Workspace(workspace) => workspace
                .packages
                .iter()
                .find(|p| p.root == root)
                .and_then(|p| p.catalog.as_ref().ok()),
        }
    }
}
