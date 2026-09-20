use std::fs;
use std::path::{Path, PathBuf};

use crate::application::ports::{LoadedCatalog, ProjectPort};
use crate::domain::error::AppError;
use crate::domain::package_manager::{Lockfiles, parse_package_json};

/// Reads the nearest package.json. Does not skip an invalid or empty nested
/// package in favour of a parent.
pub struct FsProject {
    /// Walk stops at this directory so tests cannot see the real machine root.
    cap: Option<PathBuf>,
}

impl FsProject {
    pub fn new() -> Self {
        Self { cap: None }
    }

    pub fn capped(cap: PathBuf) -> Self {
        Self { cap: Some(cap) }
    }
}

impl Default for FsProject {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectPort for FsProject {
    fn load_catalog(&self, start_dir: &Path) -> LoadedCatalog {
        let start = if start_dir.is_absolute() {
            start_dir.to_path_buf()
        } else {
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(start_dir),
                Err(error) => return LoadedCatalog::from_error(error.into()),
            }
        };
        let mut current = start;
        loop {
            let candidate = current.join("package.json");
            if candidate.exists() {
                let bytes = match fs::read(&candidate) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        return LoadedCatalog {
                            root: Some(current),
                            catalog: Err(AppError::CannotReadPackageJson { path: candidate }),
                        };
                    }
                };
                let lockfiles = Lockfiles {
                    pnpm: current.join("pnpm-lock.yaml").is_file(),
                    npm: current.join("package-lock.json").is_file(),
                };
                return LoadedCatalog {
                    root: Some(current.clone()),
                    catalog: parse_package_json(&current, &bytes, lockfiles),
                };
            }
            if self.cap.as_ref().is_some_and(|cap| current == *cap) {
                return LoadedCatalog::from_error(AppError::NoPackageJson);
            }
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => return LoadedCatalog::from_error(AppError::NoPackageJson),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::ProjectPort;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_root() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-npm-fs-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
    }

    #[test]
    fn walk_stops_at_the_first_package_json() {
        let root = temp_root();
        write(
            &root.join("app/package.json"),
            r#"{"name":"app","scripts":{"dev":"vite"}}"#,
        );
        write(&root.join("app/src/components/.keep"), "");
        let loaded = FsProject::capped(root.clone()).load_catalog(&root.join("app/src/components"));
        let catalog = loaded.catalog.unwrap();
        assert_eq!(loaded.root.as_deref(), Some(root.join("app").as_path()));
        assert_eq!(catalog.display_name, "app");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn invalid_nested_package_is_not_skipped() {
        let root = temp_root();
        write(
            &root.join("app/package.json"),
            r#"{"name":"app","scripts":{"dev":"vite"}}"#,
        );
        write(&root.join("app/packages/broken/package.json"), "{");
        let loaded =
            FsProject::capped(root.clone()).load_catalog(&root.join("app/packages/broken/src"));
        assert_eq!(
            loaded.root.as_deref(),
            Some(root.join("app/packages/broken").as_path())
        );
        assert!(matches!(loaded.catalog, Err(AppError::InvalidPackageJson)));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn empty_nested_package_is_not_skipped() {
        let root = temp_root();
        write(
            &root.join("app/package.json"),
            r#"{"name":"app","scripts":{"dev":"vite"}}"#,
        );
        write(
            &root.join("app/packages/empty/package.json"),
            r#"{"name":"empty"}"#,
        );
        let loaded = FsProject::capped(root.clone()).load_catalog(&root.join("app/packages/empty"));
        assert_eq!(
            loaded.root.as_deref(),
            Some(root.join("app/packages/empty").as_path())
        );
        assert!(matches!(loaded.catalog, Err(AppError::NoScripts)));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn parent_lockfile_is_ignored() {
        let root = temp_root();
        write(
            &root.join("app/package.json"),
            r#"{"name":"app","scripts":{"build":"tsc"}}"#,
        );
        write(&root.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n");
        let loaded = FsProject::capped(root.clone()).load_catalog(&root.join("app"));
        let catalog = loaded.catalog.unwrap();
        assert_eq!(catalog.manager.as_str(), "npm");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn directory_named_package_json_cannot_be_read() {
        let root = temp_root();
        fs::create_dir_all(root.join("app/package.json")).unwrap();
        let loaded = FsProject::capped(root.clone()).load_catalog(&root.join("app"));
        assert!(matches!(
            loaded.catalog,
            Err(AppError::CannotReadPackageJson { .. })
        ));
        fs::remove_dir_all(&root).ok();
    }
}
