use crate::domain::catalog::{PackageManager, Script};
use crate::domain::error::AppError;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Lockfiles {
    pub pnpm: bool,
    pub npm: bool,
}

/// Detect npm vs pnpm. yarn/bun declarations resolve to npm with no warning.
pub fn detect_package_manager(field: Option<&Value>, lockfiles: Lockfiles) -> PackageManager {
    package_manager_signal(field, lockfiles).unwrap_or(PackageManager::Npm)
}

pub fn package_manager_signal(
    field: Option<&Value>,
    lockfiles: Lockfiles,
) -> Option<PackageManager> {
    match field {
        Some(Value::String(raw)) => {
            let name = raw.split('@').next().unwrap_or("").trim();
            match name {
                "pnpm" => return Some(PackageManager::Pnpm),
                "npm" | "yarn" | "bun" => return Some(PackageManager::Npm),
                "" => {}
                _ => {}
            }
        }
        Some(_) => {}
        None => {}
    }
    if lockfiles.pnpm {
        Some(PackageManager::Pnpm)
    } else if lockfiles.npm {
        Some(PackageManager::Npm)
    } else {
        None
    }
}

pub fn parse_package_json(
    root: &Path,
    bytes: &[u8],
    lockfiles: Lockfiles,
) -> Result<crate::domain::catalog::PackageCatalog, AppError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| AppError::InvalidPackageJson)?;
    let manager = detect_package_manager(value.get("packageManager"), lockfiles);
    parse_package(root, value, manager, false)
}

pub fn parse_workspace_package(
    root: &Path,
    bytes: &[u8],
    manager: PackageManager,
) -> Result<crate::domain::catalog::PackageCatalog, AppError> {
    let value = serde_json::from_slice(bytes).map_err(|_| AppError::InvalidPackageJson)?;
    parse_package(root, value, manager, true)
}

fn parse_package(
    root: &Path,
    value: Value,
    manager: PackageManager,
    allow_empty: bool,
) -> Result<crate::domain::catalog::PackageCatalog, AppError> {
    let Value::Object(map) = value else {
        return Err(AppError::InvalidPackageJson);
    };
    let display_name = match map.get("name") {
        Some(Value::String(name)) if !name.is_empty() => name.clone(),
        _ => root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "package".into()),
    };
    let scripts = match map.get("scripts") {
        None if allow_empty => Vec::new(),
        None => return Err(AppError::NoScripts),
        Some(Value::Object(scripts)) => {
            if scripts.is_empty() && !allow_empty {
                return Err(AppError::NoScripts);
            }
            let mut listed = Vec::new();
            for (name, command) in scripts {
                let Value::String(command) = command else {
                    return Err(AppError::ScriptsNotObjectOfStrings);
                };
                listed.push(Script {
                    name: name.clone(),
                    command: command.clone(),
                });
            }
            listed
        }
        Some(_) => return Err(AppError::ScriptsNotObjectOfStrings),
    };
    Ok(crate::domain::catalog::PackageCatalog {
        root: PathBuf::from(root),
        display_name,
        manager,
        scripts,
    })
}
