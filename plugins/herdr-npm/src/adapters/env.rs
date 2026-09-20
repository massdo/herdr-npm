use std::env;
use std::path::PathBuf;

use serde::Deserialize;

use crate::domain::error::AppError;
use crate::domain::ids::{PaneId, TabId, WorkspaceId};
use crate::domain::pane::OriginContext;

/// Values read once at process start, then passed as ordinary data.
#[derive(Debug, Clone)]
pub struct ProcessEnv {
    pub socket_path: PathBuf,
    pub own_pane_id: Option<PaneId>,
    pub state_dir: PathBuf,
    pub tui_origin: TuiOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TuiOrigin {
    pub workspace_id: Option<String>,
    pub tab_id: Option<String>,
    pub pane_id: Option<String>,
    pub foreground_cwd: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct PluginContextJson {
    workspace_id: Option<String>,
    tab_id: Option<String>,
    focused_pane_id: Option<String>,
    focused_pane_cwd: Option<String>,
}

pub fn load() -> Result<ProcessEnv, AppError> {
    Ok(ProcessEnv {
        socket_path: socket_path()?,
        own_pane_id: env_string("HERDR_PANE_ID").map(PaneId),
        state_dir: state_dir(),
        tui_origin: TuiOrigin {
            workspace_id: env_string(crate::domain::ORIGIN_WORKSPACE_ENV),
            tab_id: env_string(crate::domain::ORIGIN_TAB_ENV),
            pane_id: env_string(crate::domain::ORIGIN_PANE_ENV),
            foreground_cwd: env_string(crate::domain::ORIGIN_FOREGROUND_CWD_ENV).map(PathBuf::from),
            cwd: env_string(crate::domain::ORIGIN_CWD_ENV).map(PathBuf::from),
        },
    })
}

pub fn origin_from_env() -> Result<OriginContext, AppError> {
    let context = plugin_context();
    let workspace = first_non_empty(&[
        env_string("HERDR_WORKSPACE_ID"),
        env_string("HERDR_ACTIVE_WORKSPACE_ID"),
        context.as_ref().and_then(|item| item.workspace_id.clone()),
    ]);
    let tab = first_non_empty(&[
        env_string("HERDR_TAB_ID"),
        env_string("HERDR_ACTIVE_TAB_ID"),
        context.as_ref().and_then(|item| item.tab_id.clone()),
    ]);
    let pane = first_non_empty(&[
        env_string("HERDR_ACTIVE_PANE_ID"),
        context
            .as_ref()
            .and_then(|item| item.focused_pane_id.clone()),
        env_string("HERDR_PANE_ID"),
    ]);
    match (workspace, tab, pane) {
        (Some(workspace_id), Some(tab_id), Some(pane_id)) => Ok(OriginContext {
            workspace_id: WorkspaceId(workspace_id),
            tab_id: TabId(tab_id),
            pane_id: PaneId(pane_id),
            foreground_cwd: first_non_empty(&[
                env_string("HERDR_ACTIVE_PANE_CWD"),
                context
                    .as_ref()
                    .and_then(|item| item.focused_pane_cwd.clone()),
            ])
            .map(PathBuf::from),
            cwd: None,
        }),
        _ => Err(AppError::OriginMissing),
    }
}

pub fn socket_path() -> Result<PathBuf, AppError> {
    if let Some(path) = env::var_os("HERDR_SOCKET_PATH") {
        return Ok(PathBuf::from(path));
    }
    let home = env::var_os("HOME").ok_or(AppError::Io {
        message: "HOME is unset and HERDR_SOCKET_PATH is missing".into(),
    })?;
    Ok(PathBuf::from(home).join(".config/herdr/herdr.sock"))
}

fn state_dir() -> PathBuf {
    if let Some(path) = env::var_os("HERDR_PLUGIN_STATE_DIR") {
        return PathBuf::from(path);
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".local/state/herdr/plugins/herdr-npm")
}

fn plugin_context() -> Option<PluginContextJson> {
    let raw = env::var("HERDR_PLUGIN_CONTEXT_JSON").ok()?;
    serde_json::from_str(&raw).ok()
}

fn env_string(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.is_empty())
}

fn first_non_empty(values: &[Option<String>]) -> Option<String> {
    values.iter().cloned().flatten().next()
}
