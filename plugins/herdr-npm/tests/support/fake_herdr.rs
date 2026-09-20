use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use herdr_npm::application::ports::{CreateTab, HerdrPort, OpenPluginPane};
use herdr_npm::domain::error::AppError;
use herdr_npm::domain::ids::PaneId;
use herdr_npm::domain::pane::{
    CreatedTab, LayoutPane, LayoutRect, LayoutSnapshot, LayoutSplit, OpenedPane, PaneInfo,
};
use herdr_npm::domain::{SIDEBAR_LABEL, SIDEBAR_TOKEN_KEY, SIDEBAR_TOKEN_VALUE};

#[derive(Debug, Clone, Default)]
pub struct RecordedCall {
    pub method: String,
    pub detail: String,
}

#[derive(Debug, Default)]
struct Inner {
    panes: Vec<PaneInfo>,
    layout: Option<LayoutSnapshot>,
    calls: Vec<RecordedCall>,
    next_pane: u32,
    #[allow(dead_code)]
    fail_open: bool,
}

#[derive(Debug, Clone, Default)]
pub struct FakeHerdr {
    inner: Arc<Mutex<Inner>>,
}

impl FakeHerdr {
    pub fn calls(&self) -> Vec<RecordedCall> {
        self.inner.lock().expect("fake herdr lock").calls.clone()
    }

    pub fn panes(&self) -> Vec<PaneInfo> {
        self.inner.lock().expect("fake herdr lock").panes.clone()
    }

    pub fn layout(&self) -> Option<herdr_npm::domain::pane::LayoutSnapshot> {
        self.inner.lock().expect("fake herdr lock").layout.clone()
    }

    pub fn seed_single_tab(&self, workspace: &str, tab: &str, pane: &str) {
        let mut inner = self.inner.lock().expect("fake herdr lock");
        inner.panes = vec![PaneInfo {
            pane_id: pane.to_string(),
            workspace_id: workspace.to_string(),
            tab_id: tab.to_string(),
            focused: true,
            label: Some("editor".into()),
            title: None,
            cwd: Some("/work/app".into()),
            foreground_cwd: Some("/work/app".into()),
            tokens: BTreeMap::new(),
        }];
        inner.layout = Some(LayoutSnapshot {
            workspace_id: workspace.to_string(),
            tab_id: tab.to_string(),
            area: LayoutRect {
                x: 0,
                y: 0,
                width: 120,
                height: 40,
            },
            focused_pane_id: pane.to_string(),
            panes: vec![LayoutPane {
                pane_id: pane.to_string(),
                focused: true,
                rect: LayoutRect {
                    x: 0,
                    y: 0,
                    width: 120,
                    height: 40,
                },
            }],
            splits: Vec::new(),
        });
        inner.next_pane = 2;
    }

    fn record(&self, method: &str, detail: impl Into<String>) {
        self.inner
            .lock()
            .expect("fake herdr lock")
            .calls
            .push(RecordedCall {
                method: method.to_string(),
                detail: detail.into(),
            });
    }
}

impl HerdrPort for FakeHerdr {
    fn list_panes(&self, workspace_id: Option<&str>) -> Result<Vec<PaneInfo>, AppError> {
        self.record("pane.list", workspace_id.unwrap_or(""));
        let inner = self.inner.lock().expect("fake herdr lock");
        if inner.panes.is_empty() {
            return Err(AppError::SnapshotUnreadable {
                detail: "no pane snapshot".into(),
            });
        }
        Ok(inner
            .panes
            .iter()
            .filter(|pane| workspace_id.is_none_or(|id| pane.workspace_id == id))
            .cloned()
            .collect())
    }

    fn pane_layout(&self, pane_id: &PaneId) -> Result<LayoutSnapshot, AppError> {
        self.record("pane.layout", pane_id.as_str());
        self.inner
            .lock()
            .expect("fake herdr lock")
            .layout
            .clone()
            .ok_or_else(|| AppError::SnapshotUnreadable {
                detail: "no layout".into(),
            })
    }

    fn open_plugin_pane(&self, request: OpenPluginPane) -> Result<OpenedPane, AppError> {
        self.record("plugin.pane.open", request.target_pane_id.as_str());
        let mut inner = self.inner.lock().expect("fake herdr lock");
        if inner.fail_open {
            return Err(AppError::herdr("plugin.pane.open", "failed", "open failed"));
        }
        let id = format!("w1:p{}", inner.next_pane);
        inner.next_pane += 1;
        let pane = PaneInfo {
            pane_id: id.clone(),
            workspace_id: request.workspace_id,
            tab_id: inner
                .panes
                .first()
                .map(|pane| pane.tab_id.clone())
                .unwrap_or_else(|| "w1:t1".into()),
            focused: request.focus,
            label: Some(SIDEBAR_LABEL.into()),
            title: Some(SIDEBAR_LABEL.into()),
            cwd: None,
            foreground_cwd: None,
            tokens: BTreeMap::new(),
        };
        inner.panes.push(pane);
        if let Some(layout) = inner.layout.as_mut() {
            layout.panes.insert(
                0,
                LayoutPane {
                    pane_id: id.clone(),
                    focused: request.focus,
                    rect: LayoutRect {
                        x: 0,
                        y: 0,
                        width: 32,
                        height: layout.area.height,
                    },
                },
            );
            layout.splits.push(LayoutSplit {
                id: "split_sidebar".into(),
                direction: "right".into(),
                ratio: 32.0 / f64::from(layout.area.width),
                rect: layout.area,
            });
        }
        Ok(OpenedPane {
            pane_id: PaneId(id),
        })
    }

    fn swap_panes(&self, source: &PaneId, target: &PaneId) -> Result<(), AppError> {
        self.record(
            "pane.swap",
            format!("{} {}", source.as_str(), target.as_str()),
        );
        Ok(())
    }

    fn focus_pane(&self, pane_id: &PaneId) -> Result<(), AppError> {
        self.record("plugin.pane.focus", pane_id.as_str());
        Ok(())
    }

    fn resize_pane(&self, pane_id: &PaneId, direction: &str, amount: f64) -> Result<(), AppError> {
        self.record(
            "pane.resize",
            format!("{} {direction} {amount}", pane_id.as_str()),
        );
        Ok(())
    }

    fn report_sidebar_identity(&self, pane_id: &PaneId) -> Result<(), AppError> {
        self.record("pane.report_metadata", pane_id.as_str());
        let mut inner = self.inner.lock().expect("fake herdr lock");
        if let Some(pane) = inner
            .panes
            .iter_mut()
            .find(|pane| pane.pane_id == pane_id.0)
        {
            pane.tokens
                .insert(SIDEBAR_TOKEN_KEY.into(), SIDEBAR_TOKEN_VALUE.into());
        }
        Ok(())
    }

    fn close_plugin_pane(&self, pane_id: &PaneId) -> Result<(), AppError> {
        self.record("plugin.pane.close", pane_id.as_str());
        let mut inner = self.inner.lock().expect("fake herdr lock");
        inner.panes.retain(|pane| pane.pane_id != pane_id.0);
        Ok(())
    }

    fn create_tab(&self, request: CreateTab) -> Result<CreatedTab, AppError> {
        self.record("tab.create", request.label);
        Err(AppError::herdr(
            "tab.create",
            "not_implemented",
            "run lot has not implemented launch yet",
        ))
    }

    fn send_input(&self, pane_id: &PaneId, text: &str, _keys: &[&str]) -> Result<(), AppError> {
        self.record("pane.send_input", format!("{} {text}", pane_id.as_str()));
        Err(AppError::herdr(
            "pane.send_input",
            "not_implemented",
            "run lot has not implemented launch yet",
        ))
    }
}
