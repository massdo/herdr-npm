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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn pnpm_field_beats_npm_lockfile() {
        assert_eq!(
            detect_package_manager(
                Some(&json!("pnpm@9.0.0")),
                Lockfiles {
                    pnpm: false,
                    npm: true
                }
            ),
            PackageManager::Pnpm
        );
    }

    #[test]
    fn yarn_declared_is_npm() {
        assert_eq!(
            detect_package_manager(
                Some(&json!("yarn@4.0.0")),
                Lockfiles {
                    pnpm: true,
                    npm: false
                }
            ),
            PackageManager::Npm
        );
    }

    #[test]
    fn pnpm_lockfile_when_field_absent() {
        assert_eq!(
            detect_package_manager(
                None,
                Lockfiles {
                    pnpm: true,
                    npm: true
                }
            ),
            PackageManager::Pnpm
        );
    }

    #[test]
    fn scripts_keep_declaration_order() {
        let json = br#"{"name":"app","scripts":{"prebuild":"a","build":"b","postbuild":"c"}}"#;
        let catalog =
            parse_package_json(Path::new("/work/app"), json, Lockfiles::default()).unwrap();
        let names: Vec<_> = catalog
            .scripts
            .iter()
            .map(|script| script.name.as_str())
            .collect();
        assert_eq!(names, ["prebuild", "build", "postbuild"]);
    }
}
