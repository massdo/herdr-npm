use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::application::ports::{CreateTab, HerdrPort, OpenPluginPane};
use crate::domain::error::AppError;
use crate::domain::ids::PaneId;
use crate::domain::pane::{CreatedTab, LayoutSnapshot, OpenedPane, PaneInfo};
use crate::domain::{PLUGIN_ID, SIDEBAR_TOKEN_KEY, SIDEBAR_TOKEN_VALUE};

const IPC_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;

/// Newline-delimited JSON client. Adapted from herdr-sidebar `ipc.rs` (MIT).
pub struct HerdrSocket {
    path: PathBuf,
}

impl HerdrSocket {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn call(&self, method: &str, params: Value) -> Result<Value, AppError> {
        let id = format!("herdr-npm:{method}");
        let request = json!({
            "id": id,
            "method": method,
            "params": params,
        });
        let line = match roundtrip(&self.path, &request.to_string()) {
            Ok(line) => line,
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                return Err(AppError::uncertain(
                    method,
                    "socket timed out after the request was written; inspect the layout before retrying",
                ));
            }
            Err(error) => {
                return Err(AppError::herdr(method, "transport", error.to_string()));
            }
        };
        let value: Value = serde_json::from_str(line.trim()).map_err(|error| {
            AppError::uncertain(method, format!("response is not JSON: {error}"))
        })?;
        let got_id = value.get("id").and_then(Value::as_str);
        if got_id != Some(id.as_str()) {
            return Err(AppError::uncertain(
                method,
                format!("response id {got_id:?} does not match request"),
            ));
        }
        if let Some(error) = value.get("error") {
            let code = error
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("herdr error")
                .to_string();
            return Err(AppError::herdr(method, code, message));
        }
        value
            .get("result")
            .cloned()
            .ok_or_else(|| AppError::uncertain(method, "response has neither result nor error"))
    }
}

impl HerdrPort for HerdrSocket {
    fn list_panes(&self, workspace_id: Option<&str>) -> Result<Vec<PaneInfo>, AppError> {
        let params = match workspace_id {
            Some(id) => json!({ "workspace_id": id }),
            None => json!({}),
        };
        let result = self.call("pane.list", params)?;
        parse_pane_list(result)
    }

    fn pane_layout(&self, pane_id: &PaneId) -> Result<LayoutSnapshot, AppError> {
        let result = self.call("pane.layout", json!({ "pane_id": pane_id.as_str() }))?;
        let layout = result
            .get("layout")
            .cloned()
            .ok_or_else(|| AppError::SnapshotUnreadable {
                detail: "pane.layout result has no layout".into(),
            })?;
        serde_json::from_value(layout).map_err(|error| AppError::SnapshotUnreadable {
            detail: error.to_string(),
        })
    }

    fn open_plugin_pane(&self, request: OpenPluginPane) -> Result<OpenedPane, AppError> {
        // workspace_id without a resolved target makes Herdr 0.9.1 reject the
        // split ("use target_pane_id"). The origin tab is implied by the target.
        let result = self.call(
            "plugin.pane.open",
            json!({
                "plugin_id": request.plugin_id,
                "entrypoint": request.entrypoint,
                "placement": "split",
                "direction": "right",
                "target_pane_id": request.target_pane_id.as_str(),
                "focus": request.focus,
                "env": request.env,
            }),
        )?;
        let pane_id = result
            .get("plugin_pane")
            .and_then(|pane| pane.get("pane"))
            .and_then(|pane| pane.get("pane_id"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AppError::uncertain(
                    "plugin.pane.open",
                    "response has no plugin_pane.pane.pane_id",
                )
            })?;
        Ok(OpenedPane {
            pane_id: PaneId(pane_id.to_string()),
        })
    }

    fn swap_panes(&self, source: &PaneId, target: &PaneId) -> Result<(), AppError> {
        self.call(
            "pane.swap",
            json!({
                "source_pane_id": source.as_str(),
                "target_pane_id": target.as_str(),
            }),
        )?;
        Ok(())
    }

    fn focus_pane(&self, pane_id: &PaneId) -> Result<(), AppError> {
        self.call("plugin.pane.focus", json!({ "pane_id": pane_id.as_str() }))?;
        Ok(())
    }

    fn resize_pane(&self, pane_id: &PaneId, direction: &str, amount: f64) -> Result<(), AppError> {
        self.call(
            "pane.resize",
            json!({
                "pane_id": pane_id.as_str(),
                "direction": direction,
                "amount": amount,
            }),
        )?;
        Ok(())
    }

    fn report_sidebar_identity(&self, pane_id: &PaneId) -> Result<(), AppError> {
        let mut tokens = BTreeMap::new();
        tokens.insert(
            SIDEBAR_TOKEN_KEY.to_string(),
            SIDEBAR_TOKEN_VALUE.to_string(),
        );
        self.call(
            "pane.report_metadata",
            json!({
                "pane_id": pane_id.as_str(),
                "source": PLUGIN_ID,
                "tokens": tokens,
            }),
        )?;
        Ok(())
    }

    fn close_plugin_pane(&self, pane_id: &PaneId) -> Result<(), AppError> {
        self.call("plugin.pane.close", json!({ "pane_id": pane_id.as_str() }))?;
        Ok(())
    }

    fn create_tab(&self, request: CreateTab) -> Result<CreatedTab, AppError> {
        let result = self.call(
            "tab.create",
            json!({
                "workspace_id": request.workspace_id,
                "cwd": request.cwd,
                "label": request.label,
                "focus": request.focus,
            }),
        )?;
        let tab_id = result
            .get("tab")
            .and_then(|tab| tab.get("tab_id"))
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::uncertain("tab.create", "response has no tab.tab_id"))?;
        let root_pane_id = result
            .get("root_pane")
            .and_then(|pane| pane.get("pane_id"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AppError::uncertain("tab.create", "response has no root_pane.pane_id")
            })?;
        Ok(CreatedTab {
            tab_id: tab_id.to_string().into(),
            root_pane_id: root_pane_id.to_string().into(),
        })
    }

    fn send_input(&self, pane_id: &PaneId, text: &str, keys: &[&str]) -> Result<(), AppError> {
        self.call(
            "pane.send_input",
            json!({
                "pane_id": pane_id.as_str(),
                "text": text,
                "keys": keys,
            }),
        )?;
        Ok(())
    }
}

#[derive(Deserialize)]
struct PaneListResult {
    panes: Vec<PaneInfo>,
}

fn parse_pane_list(result: Value) -> Result<Vec<PaneInfo>, AppError> {
    serde_json::from_value::<PaneListResult>(result)
        .map(|body| body.panes)
        .map_err(|error| AppError::SnapshotUnreadable {
            detail: error.to_string(),
        })
}

fn roundtrip(path: &Path, request: &str) -> std::io::Result<String> {
    let stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(IPC_TIMEOUT))?;
    stream.set_write_timeout(Some(IPC_TIMEOUT))?;
    let mut stream = stream;
    stream.write_all(request.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    let mut line = String::new();
    BufReader::new(stream.take(MAX_RESPONSE_BYTES)).read_line(&mut line)?;
    if line.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "empty socket response",
        ));
    }
    Ok(line)
}
