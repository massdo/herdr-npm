use crate::application::ports::{CreateTab, HerdrPort};
use crate::domain::catalog::PackageCatalog;
use crate::domain::error::AppError;
use crate::domain::pane::CreatedTab;
use crate::domain::run_command::run_invocation;

/// Create a background tab in the frozen catalogue workspace and send the
/// invocation once to its root pane. Never retries.
pub fn run_script<H: HerdrPort>(
    herdr: &H,
    catalog: &PackageCatalog,
    workspace_id: &str,
    script_name: &str,
) -> Result<CreatedTab, AppError> {
    let command = run_invocation(catalog.manager, script_name);
    let created = herdr.create_tab(CreateTab {
        workspace_id: workspace_id.to_string(),
        cwd: catalog.root.clone(),
        label: command.clone(),
        focus: false,
    })?;
    if created.root_pane_id.as_str().is_empty() {
        return Err(AppError::LaunchNotConfirmed { tab_id: None });
    }
    match herdr.send_input(&created.root_pane_id, &command, &["Enter"]) {
        Ok(()) => Ok(created),
        Err(_) => Err(AppError::LaunchNotConfirmed {
            tab_id: Some(created.tab_id.0.clone()),
        }),
    }
}
