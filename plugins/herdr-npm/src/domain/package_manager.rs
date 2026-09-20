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
    match field {
        Some(Value::String(raw)) => {
            let name = raw.split('@').next().unwrap_or("").trim();
            match name {
                "pnpm" => return PackageManager::Pnpm,
                "npm" | "yarn" | "bun" => return PackageManager::Npm,
                "" => {}
                _ => {}
            }
        }
        Some(_) => {}
        None => {}
    }
    if lockfiles.pnpm {
        PackageManager::Pnpm
    } else {
        PackageManager::Npm
    }
}

pub fn parse_package_json(
    root: &Path,
    bytes: &[u8],
    lockfiles: Lockfiles,
) -> Result<crate::domain::catalog::PackageCatalog, AppError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| AppError::InvalidPackageJson)?;
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
        None => return Err(AppError::NoScripts),
        Some(Value::Object(scripts)) => {
            if scripts.is_empty() {
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
        manager: detect_package_manager(map.get("packageManager"), lockfiles),
        scripts,
    })
}
