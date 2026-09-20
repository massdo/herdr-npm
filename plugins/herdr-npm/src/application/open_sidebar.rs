use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::application::ports::{HerdrPort, OpenPluginPane};
use crate::domain::error::AppError;
use crate::domain::geometry::{pick_working_target, preferred_left_resize};
use crate::domain::ids::PaneId;
use crate::domain::pane::OriginContext;
use crate::domain::{
    ORIGIN_CWD_ENV, ORIGIN_FOREGROUND_CWD_ENV, ORIGIN_PANE_ENV, ORIGIN_TAB_ENV,
    ORIGIN_WORKSPACE_ENV, PANE_ENTRYPOINT, PLUGIN_ID,
};

/// Open an empty sidebar to the left of the working-pane target.
/// Two-state toggle, identity races and the lock belong to a later lot.
pub fn open_empty_sidebar<H: HerdrPort>(
    herdr: &H,
    origin: &OriginContext,
) -> Result<PaneId, AppError> {
    let panes = herdr.list_panes(Some(origin.workspace_id.as_str()))?;
    let origin_pane = panes
        .iter()
        .find(|pane| pane.pane_id == origin.pane_id.0)
        .ok_or(AppError::OriginChanged)?;
    if origin_pane.tab_id != origin.tab_id.0 || origin_pane.workspace_id != origin.workspace_id.0 {
        return Err(AppError::OriginChanged);
    }

    let layout = herdr.pane_layout(&origin.pane_id)?;
    let target = pick_working_target(&panes, &layout, &origin.tab_id)?;
    let env = origin_env(origin, origin_pane);
    let opened = herdr.open_plugin_pane(OpenPluginPane {
        plugin_id: PLUGIN_ID.to_string(),
        entrypoint: PANE_ENTRYPOINT.to_string(),
        target_pane_id: target.id(),
        workspace_id: origin.workspace_id.0.clone(),
        focus: false,
        env,
    })?;

    let cleanup = |herdr: &H, pane_id: &PaneId, error: AppError| -> AppError {
        if matches!(error, AppError::Uncertain { .. }) {
            return error;
        }
        match herdr.close_plugin_pane(pane_id) {
            Ok(()) => error,
            Err(AppError::Uncertain { .. }) => AppError::uncertain(
                "plugin.pane.close",
                "sidebar opened but a later step failed and close was not confirmed",
            ),
            Err(_) => error,
        }
    };

    if let Err(error) = herdr.swap_panes(&opened.pane_id, &target.id()) {
        return Err(cleanup(herdr, &opened.pane_id, error));
    }
    if let Err(error) = herdr.report_sidebar_identity(&opened.pane_id) {
        return Err(cleanup(herdr, &opened.pane_id, error));
    }

    match herdr.pane_layout(&opened.pane_id) {
        Ok(layout) => {
            if let Some(step) = preferred_left_resize(&layout, opened.pane_id.as_str())
                && let Err(error) = herdr.resize_pane(&opened.pane_id, step.direction, step.amount)
            {
                return Err(cleanup(herdr, &opened.pane_id, error));
            }
        }
        Err(error) => return Err(cleanup(herdr, &opened.pane_id, error)),
    }

    if let Err(error) = herdr.focus_pane(&opened.pane_id) {
        return Err(cleanup(herdr, &opened.pane_id, error));
    }
    Ok(opened.pane_id)
}

fn origin_env(
    origin: &OriginContext,
    origin_pane: &crate::domain::pane::PaneInfo,
) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert(
        ORIGIN_WORKSPACE_ENV.to_string(),
        origin.workspace_id.0.clone(),
    );
    env.insert(ORIGIN_TAB_ENV.to_string(), origin.tab_id.0.clone());
    env.insert(ORIGIN_PANE_ENV.to_string(), origin.pane_id.0.clone());
    let foreground = origin
        .foreground_cwd
        .clone()
        .or_else(|| origin_pane.foreground_cwd.as_ref().map(PathBuf::from));
    let cwd = origin
        .cwd
        .clone()
        .or_else(|| origin_pane.cwd.as_ref().map(PathBuf::from));
    if let Some(path) = foreground {
        env.insert(
            ORIGIN_FOREGROUND_CWD_ENV.to_string(),
            path.display().to_string(),
        );
    }
    if let Some(path) = cwd {
        env.insert(ORIGIN_CWD_ENV.to_string(), path.display().to_string());
    }
    env
}
