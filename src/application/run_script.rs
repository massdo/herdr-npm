use crate::application::ports::{CreateTab, HerdrPort};
use crate::domain::catalog::{PackageCatalog, ProjectCatalog, RunIntent};
use crate::domain::error::AppError;
use crate::domain::pane::CreatedTab;
use crate::domain::run_command::run_invocation;

pub fn run_intent<H: HerdrPort>(
    herdr: &H,
    project: &ProjectCatalog,
    workspace_id: &str,
    intent: &RunIntent,
) -> Result<CreatedTab, AppError> {
    let catalog = project
        .package(&intent.package_root)
        .filter(|package| {
            package
                .scripts
                .iter()
                .any(|script| script.name == intent.script_name)
        })
        .ok_or_else(|| AppError::ScriptUnavailable {
            package_root: intent.package_root.clone(),
            script_name: intent.script_name.clone(),
        })?;
    run_script(herdr, catalog, workspace_id, &intent.script_name)
}

/// Create a background tab in the frozen catalogue workspace and send the
/// invocation once to its root pane. Never retries.
pub fn run_script<H: HerdrPort>(
    herdr: &H,
    catalog: &PackageCatalog,
    workspace_id: &str,
    script_name: &str,
) -> Result<CreatedTab, AppError> {
    let command = run_invocation(catalog.manager, script_name);
    let created = herdr
        .create_tab(CreateTab {
            workspace_id: workspace_id.to_string(),
            cwd: catalog.root.clone(),
            label: command.clone(),
            focus: false,
        })
        .map_err(|error| match error {
            AppError::Uncertain { .. } => AppError::LaunchNotConfirmed { tab_id: None },
            other => other,
        })?;
    if created.root_pane_id.as_str().is_empty() {
        return Err(AppError::LaunchNotConfirmed {
            tab_id: Some(created.tab_id.0),
        });
    }
    match herdr.send_input(&created.root_pane_id, &command, &["Enter"]) {
        Ok(()) => Ok(created),
        Err(_) => Err(AppError::LaunchNotConfirmed {
            tab_id: Some(created.tab_id.0.clone()),
        }),
    }
}
