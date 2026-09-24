use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use serde_json::Value;
use walkdir::WalkDir;

use crate::application::ports::{LoadedCatalog, ProjectPort};
use crate::domain::catalog::{PackageManager, ProjectCatalog, WorkspaceCatalog, WorkspacePackage};
use crate::domain::error::AppError;
use crate::domain::package_manager::{
    Lockfiles, package_manager_signal, parse_package_json, parse_workspace_package,
};

/// Discovers declared workspaces, or the nearest standalone package.
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
        let mut nearest_package = None;
        let mut current = start.clone();
        loop {
            let candidate = current.join("package.json");
            if nearest_package.is_none() && exists(&candidate) {
                nearest_package = Some(current.clone());
            }
            match workspace_declaration(&current) {
                Ok(Some((path, patterns))) => {
                    let workspace =
                        discover_workspace(&current, &path, &patterns, nearest_package.as_deref());
                    match workspace {
                        Ok(workspace)
                            if nearest_package.as_ref().is_none_or(|package| {
                                package == &current
                                    || workspace.packages.iter().any(|p| {
                                        p.root
                                            == package
                                                .canonicalize()
                                                .unwrap_or_else(|_| package.clone())
                                    })
                            }) =>
                        {
                            return LoadedCatalog {
                                root: Some(workspace.root.clone()),
                                catalog: Ok(ProjectCatalog::Workspace(workspace)),
                            };
                        }
                        Ok(_) => {}
                        Err(error) => {
                            return LoadedCatalog {
                                root: Some(current),
                                catalog: Err(error),
                            };
                        }
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    return LoadedCatalog {
                        root: Some(current),
                        catalog: Err(error),
                    };
                }
            }
            if exists(&current.join(".git"))
                || self.cap.as_ref().is_some_and(|cap| {
                    current == *cap || current == cap.canonicalize().unwrap_or_else(|_| cap.clone())
                })
            {
                break;
            }
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => break,
            }
        }
        match nearest_package {
            Some(root) => LoadedCatalog {
                catalog: read_manifest(&root)
                    .and_then(|bytes| parse_package_json(&root, &bytes, lockfiles(&root)))
                    .map(ProjectCatalog::Package),
                root: Some(root),
            },
            None => LoadedCatalog::from_error(AppError::NoPackageJson),
        }
    }
}

fn exists(path: &Path) -> bool {
    !matches!(fs::symlink_metadata(path), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
}

fn read_manifest(root: &Path) -> Result<Vec<u8>, AppError> {
    let path = root.join("package.json");
    fs::read(&path).map_err(|_| AppError::CannotReadPackageJson { path })
}

fn lockfiles(root: &Path) -> Lockfiles {
    Lockfiles {
        pnpm: root.join("pnpm-lock.yaml").is_file(),
        npm: root.join("package-lock.json").is_file(),
    }
}

fn invalid_workspace(path: &Path, detail: impl ToString) -> AppError {
    AppError::InvalidWorkspace {
        path: path.to_path_buf(),
        detail: detail.to_string(),
    }
}

fn workspace_declaration(root: &Path) -> Result<Option<(PathBuf, Vec<String>)>, AppError> {
    let yaml = root.join("pnpm-workspace.yaml");
    if exists(&yaml) {
        #[derive(Deserialize)]
        struct PnpmWorkspace {
            packages: Vec<serde_yaml_ng::Value>,
        }
        let bytes = fs::read(&yaml).map_err(|e| invalid_workspace(&yaml, e))?;
        let config: PnpmWorkspace =
            serde_yaml_ng::from_slice(&bytes).map_err(|e| invalid_workspace(&yaml, e))?;
        let packages = config
            .packages
            .into_iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| invalid_workspace(&yaml, "package patterns must be strings"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(Some((yaml, packages)));
    }
    let json = root.join("package.json");
    // A broken package remains a local package error unless it declares a workspace.
    let Some(value) = fs::read(&json)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
    else {
        return Ok(None);
    };
    let Some(workspaces) = value.get("workspaces") else {
        return Ok(None);
    };
    let packages = if workspaces.is_object() {
        workspaces.get("packages").unwrap_or(&Value::Null)
    } else {
        workspaces
    };
    let patterns: Vec<String> =
        serde_json::from_value(packages.clone()).map_err(|e| invalid_workspace(&json, e))?;
    Ok(Some((json, patterns)))
}

fn pattern_sets(path: &Path, patterns: &[String]) -> Result<(GlobSet, GlobSet), AppError> {
    let mut includes = GlobSetBuilder::new();
    let mut excludes = GlobSetBuilder::new();
    for raw in patterns {
        let (negative, pattern) = raw
            .strip_prefix('!')
            .map(|p| (true, p))
            .unwrap_or((false, raw));
        let pattern = pattern
            .strip_prefix("./")
            .unwrap_or(pattern)
            .trim_end_matches('/');
        if pattern.is_empty()
            || Path::new(pattern).is_absolute()
            || pattern.split('/').any(|part| part == "..")
        {
            return Err(invalid_workspace(
                path,
                format!("invalid package pattern {raw:?}"),
            ));
        }
        let glob = GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .map_err(|e| invalid_workspace(path, e))?;
        if negative {
            excludes.add(glob);
        } else {
            includes.add(glob);
        }
    }
    Ok((
        includes.build().map_err(|e| invalid_workspace(path, e))?,
        excludes.build().map_err(|e| invalid_workspace(path, e))?,
    ))
}

fn ignored(path: &Path) -> bool {
    path.components()
        .any(|part| part.as_os_str() == "node_modules" || part.as_os_str() == ".git")
}

fn discover_workspace(
    root: &Path,
    declaration: &Path,
    patterns: &[String],
    active: Option<&Path>,
) -> Result<WorkspaceCatalog, AppError> {
    let (includes, excludes) = pattern_sets(declaration, patterns)?;
    let root = root
        .canonicalize()
        .map_err(|e| invalid_workspace(declaration, e))?;
    let mut members = BTreeSet::new();
    let walker = WalkDir::new(&root)
        .follow_links(true)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            if ignored(entry.path().strip_prefix(&root).unwrap_or(entry.path())) {
                return false;
            }
            entry
                .path()
                .canonicalize()
                .is_ok_and(|path| path.strip_prefix(&root).is_ok_and(|p| !ignored(p)))
        });
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) if error.loop_ancestor().is_some() => continue,
            Err(error)
                if error.path().is_some_and(|path| {
                    // A member already discovered below retains its own manifest error.
                    path.canonicalize()
                        .is_ok_and(|path| members.contains(&path))
                        || fs::symlink_metadata(path)
                            .is_ok_and(|meta| meta.file_type().is_symlink())
                }) =>
            {
                continue;
            }
            Err(error) => return Err(invalid_workspace(declaration, error)),
        };
        if entry.depth() == 0 || !entry.file_type().is_dir() {
            continue;
        }
        let relative = entry.path().strip_prefix(&root).expect("walk inside root");
        if includes.is_match(relative)
            && !excludes.is_match(relative)
            && exists(&entry.path().join("package.json"))
        {
            let canonical = entry
                .path()
                .canonicalize()
                .map_err(|e| invalid_workspace(declaration, e))?;
            // Exclusions also apply when an included alias points to an excluded member.
            if !excludes.is_match(canonical.strip_prefix(&root).expect("filtered root")) {
                members.insert(canonical);
            }
        }
    }
    members.remove(&root);
    let roots = exists(&root.join("package.json"))
        .then_some(root.clone())
        .into_iter()
        .chain(members);
    let packages = roots
        .map(|package_root| {
            let relative_path = if package_root == root {
                PathBuf::from(".")
            } else {
                package_root.strip_prefix(&root).unwrap().to_path_buf()
            };
            let catalog = if package_root
                .join("package.json")
                .canonicalize()
                .is_ok_and(|path| !path.starts_with(&root))
            {
                Err(AppError::CannotReadPackageJson {
                    path: package_root.join("package.json"),
                })
            } else {
                read_manifest(&package_root).and_then(|bytes| {
                    parse_workspace_package(
                        &package_root,
                        &bytes,
                        inherited_manager(&package_root, &root),
                    )
                })
            };
            WorkspacePackage {
                root: package_root,
                relative_path,
                catalog,
            }
        })
        .collect();
    Ok(WorkspaceCatalog {
        root,
        packages,
        active_package: active.map(|p| p.canonicalize().unwrap_or_else(|_| p.to_path_buf())),
    })
}

fn inherited_manager(package: &Path, root: &Path) -> PackageManager {
    for current in package.ancestors() {
        let value = read_manifest(current)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
        if let Some(manager) = package_manager_signal(
            value.as_ref().and_then(|v| v.get("packageManager")),
            lockfiles(current),
        ) {
            return manager;
        }
        if current == root {
            break;
        }
    }
    PackageManager::Npm
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
        let project = loaded.catalog.unwrap();
        let catalog = project.first_package().unwrap();
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
        let project = loaded.catalog.unwrap();
        let catalog = project.first_package().unwrap();
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

    #[test]
    fn broken_package_symlink_does_not_load_the_parent_package() {
        let root = temp_root();
        write(
            &root.join("package.json"),
            r#"{"scripts":{"dev":"echo parent"}}"#,
        );
        fs::create_dir(root.join("nested")).unwrap();
        std::os::unix::fs::symlink("missing.json", root.join("nested/package.json")).unwrap();
        let loaded = FsProject::capped(root.clone()).load_catalog(&root.join("nested"));
        assert_eq!(loaded.root, Some(root.join("nested")));
        assert!(matches!(
            loaded.catalog,
            Err(AppError::CannotReadPackageJson { .. })
        ));
        fs::remove_dir_all(root).unwrap();
    }
}
