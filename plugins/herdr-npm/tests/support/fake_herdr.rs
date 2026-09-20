use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenFail {
    None,
    Uncertain,
}

#[derive(Debug)]
struct Inner {
    panes: Vec<PaneInfo>,
    layouts: BTreeMap<String, LayoutSnapshot>,
    calls: Vec<RecordedCall>,
    next_pane: u32,
    open_fail: OpenFail,
    list_unreadable: bool,
    list_delay: Duration,
    list_inflight: u32,
    max_list_inflight: u32,
    focused: Option<String>,
    exited: Vec<String>,
    diverted_tab: Option<String>,
    created_tabs: Vec<FakeTab>,
    next_tab: u32,
    create_fail: bool,
    forced_tab_id: Option<String>,
    missing_root: bool,
    send_timeout: bool,
    shell_path: Option<PathBuf>,
    argv_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct FakeTab {
    pub tab_id: String,
    pub root_pane_id: String,
    pub workspace_id: String,
    pub cwd: PathBuf,
    pub label: String,
    pub focus: bool,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            panes: Vec::new(),
            layouts: BTreeMap::new(),
            calls: Vec::new(),
            next_pane: 1,
            open_fail: OpenFail::None,
            list_unreadable: false,
            list_delay: Duration::ZERO,
            list_inflight: 0,
            max_list_inflight: 0,
            focused: None,
            exited: Vec::new(),
            diverted_tab: None,
            created_tabs: Vec::new(),
            next_tab: 1,
            create_fail: false,
            forced_tab_id: None,
            missing_root: false,
            send_timeout: false,
            shell_path: None,
            argv_file: None,
        }
    }
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

    pub fn layout_for_pane(&self, pane_id: &str) -> Option<LayoutSnapshot> {
        let inner = self.inner.lock().expect("fake herdr lock");
        let tab = inner
            .panes
            .iter()
            .find(|pane| pane.pane_id == pane_id)
            .map(|pane| pane.tab_id.clone())?;
        inner.layouts.get(&tab).cloned()
    }

    pub fn focused(&self) -> Option<String> {
        self.inner.lock().expect("fake herdr lock").focused.clone()
    }

    pub fn exited(&self) -> Vec<String> {
        self.inner.lock().expect("fake herdr lock").exited.clone()
    }

    pub fn max_list_inflight(&self) -> u32 {
        self.inner
            .lock()
            .expect("fake herdr lock")
            .max_list_inflight
    }

    pub fn set_list_delay(&self, delay: Duration) {
        self.inner.lock().expect("fake herdr lock").list_delay = delay;
    }

    pub fn set_list_unreadable(&self) {
        self.inner.lock().expect("fake herdr lock").list_unreadable = true;
    }

    pub fn set_open_uncertain(&self) {
        self.inner.lock().expect("fake herdr lock").open_fail = OpenFail::Uncertain;
    }

    pub fn rename_pane(&self, from: &str, to: &str) {
        let mut inner = self.inner.lock().expect("fake herdr lock");
        for pane in &mut inner.panes {
            if pane.pane_id == from {
                pane.pane_id = to.to_string();
            }
        }
        if inner.focused.as_deref() == Some(from) {
            inner.focused = Some(to.to_string());
        }
        for layout in inner.layouts.values_mut() {
            if layout.focused_pane_id == from {
                layout.focused_pane_id = to.to_string();
            }
            for pane in &mut layout.panes {
                if pane.pane_id == from {
                    pane.pane_id = to.to_string();
                }
            }
        }
    }

    pub fn set_focus(&self, pane_id: &str) {
        let mut inner = self.inner.lock().expect("fake herdr lock");
        inner.focused = Some(pane_id.to_string());
        for pane in &mut inner.panes {
            pane.focused = pane.pane_id == pane_id;
        }
    }

    pub fn divert_focus_on_list(&self, tab: &str) {
        self.inner.lock().expect("fake herdr lock").diverted_tab = Some(tab.to_string());
    }

    pub fn set_create_fail(&self) {
        self.inner.lock().expect("fake herdr lock").create_fail = true;
    }

    pub fn set_forced_tab_id(&self, tab_id: &str) {
        self.inner.lock().expect("fake herdr lock").forced_tab_id = Some(tab_id.to_string());
    }

    pub fn set_missing_root(&self) {
        self.inner.lock().expect("fake herdr lock").missing_root = true;
    }

    pub fn set_send_timeout(&self) {
        self.inner.lock().expect("fake herdr lock").send_timeout = true;
    }

    pub fn set_shell_exec(&self, path_prefix: PathBuf, argv_file: PathBuf) {
        let mut inner = self.inner.lock().expect("fake herdr lock");
        inner.shell_path = Some(path_prefix);
        inner.argv_file = Some(argv_file);
    }

    pub fn created_tabs(&self) -> Vec<FakeTab> {
        self.inner
            .lock()
            .expect("fake herdr lock")
            .created_tabs
            .clone()
    }

    pub fn remove_pane(&self, pane_id: &str) {
        let mut inner = self.inner.lock().expect("fake herdr lock");
        inner.panes.retain(|pane| pane.pane_id != pane_id);
        for layout in inner.layouts.values_mut() {
            layout.panes.retain(|pane| pane.pane_id != pane_id);
        }
    }

    pub fn seed_single_tab(&self, workspace: &str, tab: &str, pane: &str) {
        let mut inner = self.inner.lock().expect("fake herdr lock");
        *inner = Inner::default();
        inner.panes = vec![working_pane(workspace, tab, pane, true)];
        inner.layouts.insert(
            tab.to_string(),
            full_tab_layout(workspace, tab, pane, 120, 40, 0),
        );
        inner.focused = Some(pane.to_string());
        inner.next_pane = 2;
    }

    pub fn ensure_working_pane(&self, workspace: &str, tab: &str, pane: &str) {
        let mut inner = self.inner.lock().expect("fake herdr lock");
        if inner.panes.iter().any(|item| item.pane_id == pane) {
            return;
        }
        inner.panes.push(working_pane(workspace, tab, pane, false));
        inner.layouts.insert(
            tab.to_string(),
            full_tab_layout(workspace, tab, pane, 120, 40, 0),
        );
    }

    pub fn seed_half_height(&self, workspace: &str, tab: &str, editor: &str, bottom: &str) {
        let mut inner = self.inner.lock().expect("fake herdr lock");
        *inner = Inner::default();
        inner.panes = vec![
            working_pane(workspace, tab, editor, true),
            working_pane(workspace, tab, bottom, false),
        ];
        inner.layouts.insert(
            tab.to_string(),
            LayoutSnapshot {
                workspace_id: workspace.to_string(),
                tab_id: tab.to_string(),
                area: LayoutRect {
                    x: 0,
                    y: 0,
                    width: 120,
                    height: 40,
                },
                focused_pane_id: editor.to_string(),
                panes: vec![
                    LayoutPane {
                        pane_id: editor.to_string(),
                        focused: true,
                        rect: LayoutRect {
                            x: 0,
                            y: 0,
                            width: 120,
                            height: 20,
                        },
                    },
                    LayoutPane {
                        pane_id: bottom.to_string(),
                        focused: false,
                        rect: LayoutRect {
                            x: 0,
                            y: 20,
                            width: 120,
                            height: 20,
                        },
                    },
                ],
                splits: vec![LayoutSplit {
                    id: "vertical".into(),
                    direction: "down".into(),
                    ratio: 0.5,
                    rect: LayoutRect {
                        x: 0,
                        y: 0,
                        width: 120,
                        height: 40,
                    },
                }],
            },
        );
        inner.focused = Some(editor.to_string());
        inner.next_pane = 3;
    }

    pub fn seed_explorer_only(&self, workspace: &str, tab: &str, explorer: &str) {
        let mut inner = self.inner.lock().expect("fake herdr lock");
        *inner = Inner::default();
        let mut pane = working_pane(workspace, tab, explorer, true);
        pane.label = Some("Sidebar".into());
        pane.tokens
            .insert("herdr-sidebar-explorer".into(), "1".into());
        inner.panes = vec![pane];
        inner.layouts.insert(
            tab.to_string(),
            full_tab_layout(workspace, tab, explorer, 32, 40, 0),
        );
        inner.focused = Some(explorer.to_string());
        inner.next_pane = 2;
    }

    pub fn add_recognised_sidebar(&self, tab: &str, split_from: &str) -> String {
        let mut inner = self.inner.lock().expect("fake herdr lock");
        let workspace = inner
            .panes
            .iter()
            .find(|pane| pane.tab_id == tab)
            .map(|pane| pane.workspace_id.clone())
            .unwrap_or_else(|| "w1".into());
        inner.next_pane += 1;
        let id = format!("{tab}-sidebar");
        let mut pane = working_pane(&workspace, tab, &id, false);
        pane.label = Some(SIDEBAR_LABEL.into());
        pane.tokens
            .insert(SIDEBAR_TOKEN_KEY.into(), SIDEBAR_TOKEN_VALUE.into());
        inner.panes.push(pane);
        if let Some(layout) = inner.layouts.get_mut(tab) {
            dock_left(layout, split_from, &id);
        }
        id
    }

    pub fn add_token_pane(&self, workspace: &str, tab: &str, pane_id: &str) {
        let mut inner = self.inner.lock().expect("fake herdr lock");
        if !inner.layouts.contains_key(tab) {
            inner.layouts.insert(
                tab.to_string(),
                full_tab_layout(workspace, tab, pane_id, 120, 40, 0),
            );
        }
        let mut pane = working_pane(workspace, tab, pane_id, false);
        pane.tokens
            .insert(SIDEBAR_TOKEN_KEY.into(), SIDEBAR_TOKEN_VALUE.into());
        inner.panes.push(pane);
    }

    pub fn add_labelled_npm(&self, tab: &str) -> String {
        let mut inner = self.inner.lock().expect("fake herdr lock");
        let workspace = inner
            .panes
            .iter()
            .find(|pane| pane.tab_id == tab)
            .map(|pane| pane.workspace_id.clone())
            .unwrap_or_else(|| "w1".into());
        let id = format!("{tab}-npm-foreign");
        let mut pane = working_pane(&workspace, tab, &id, false);
        pane.label = Some(SIDEBAR_LABEL.into());
        inner.panes.push(pane);
        if let Some(layout) = inner.layouts.get_mut(tab) {
            layout.panes.push(LayoutPane {
                pane_id: id.clone(),
                focused: false,
                rect: LayoutRect {
                    x: 80,
                    y: 0,
                    width: 40,
                    height: layout.area.height,
                },
            });
        }
        id
    }

    pub fn other_pane_rects(&self, tab: &str, exclude: &[&str]) -> Vec<(String, LayoutRect)> {
        self.inner
            .lock()
            .expect("fake herdr lock")
            .layouts
            .get(tab)
            .map(|layout| {
                layout
                    .panes
                    .iter()
                    .filter(|pane| !exclude.contains(&pane.pane_id.as_str()))
                    .map(|pane| (pane.pane_id.clone(), pane.rect))
                    .collect()
            })
            .unwrap_or_default()
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

fn working_pane(workspace: &str, tab: &str, pane: &str, focused: bool) -> PaneInfo {
    PaneInfo {
        pane_id: pane.to_string(),
        workspace_id: workspace.to_string(),
        tab_id: tab.to_string(),
        focused,
        label: Some("editor".into()),
        title: None,
        cwd: Some("/work/app".into()),
        foreground_cwd: Some("/work/app".into()),
        tokens: BTreeMap::new(),
    }
}

fn full_tab_layout(
    workspace: &str,
    tab: &str,
    pane: &str,
    width: u16,
    height: u16,
    y: u16,
) -> LayoutSnapshot {
    LayoutSnapshot {
        workspace_id: workspace.to_string(),
        tab_id: tab.to_string(),
        area: LayoutRect {
            x: 0,
            y: 0,
            width,
            height,
        },
        focused_pane_id: pane.to_string(),
        panes: vec![LayoutPane {
            pane_id: pane.to_string(),
            focused: true,
            rect: LayoutRect {
                x: 0,
                y,
                width,
                height,
            },
        }],
        splits: Vec::new(),
    }
}

fn dock_left(layout: &mut LayoutSnapshot, target: &str, sidebar: &str) {
    let Some(target_pane) = layout.panes.iter_mut().find(|pane| pane.pane_id == target) else {
        return;
    };
    let height = target_pane.rect.height;
    let y = target_pane.rect.y;
    let total = target_pane.rect.width;
    let side_w = 32.min(total.saturating_sub(1)).max(1);
    target_pane.rect.x = side_w;
    target_pane.rect.width = total.saturating_sub(side_w);
    layout.panes.insert(
        0,
        LayoutPane {
            pane_id: sidebar.to_string(),
            focused: false,
            rect: LayoutRect {
                x: 0,
                y,
                width: side_w,
                height,
            },
        },
    );
}

impl HerdrPort for FakeHerdr {
    fn list_panes(&self, workspace_id: Option<&str>) -> Result<Vec<PaneInfo>, AppError> {
        self.record("pane.list", workspace_id.unwrap_or(""));
        let delay;
        {
            let mut inner = self.inner.lock().expect("fake herdr lock");
            if inner.list_unreadable {
                return Err(AppError::SnapshotUnreadable {
                    detail: "synthetic unreadable snapshot".into(),
                });
            }
            inner.list_inflight += 1;
            inner.max_list_inflight = inner.max_list_inflight.max(inner.list_inflight);
            delay = inner.list_delay;
        }
        if !delay.is_zero() {
            std::thread::sleep(delay);
        }
        let mut inner = self.inner.lock().expect("fake herdr lock");
        inner.list_inflight = inner.list_inflight.saturating_sub(1);
        if let Some(tab) = inner.diverted_tab.clone() {
            inner.focused = inner
                .panes
                .iter()
                .find(|pane| pane.tab_id == tab)
                .map(|pane| pane.pane_id.clone());
        }
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
        self.layout_for_pane(pane_id.as_str())
            .ok_or_else(|| AppError::SnapshotUnreadable {
                detail: "no layout".into(),
            })
    }

    fn open_plugin_pane(&self, request: OpenPluginPane) -> Result<OpenedPane, AppError> {
        self.record("plugin.pane.open", request.target_pane_id.as_str());
        let mut inner = self.inner.lock().expect("fake herdr lock");
        if inner.open_fail == OpenFail::Uncertain {
            return Err(AppError::uncertain(
                "plugin.pane.open",
                "socket timed out after the request was written; inspect the layout before retrying",
            ));
        }
        let target = inner
            .panes
            .iter()
            .find(|pane| pane.pane_id == request.target_pane_id.0)
            .cloned()
            .ok_or_else(|| AppError::herdr("plugin.pane.open", "not_found", "target missing"))?;
        inner.next_pane += 1;
        let id = format!("{}:p{}", target.tab_id, inner.next_pane);
        let pane = PaneInfo {
            pane_id: id.clone(),
            workspace_id: target.workspace_id.clone(),
            tab_id: target.tab_id.clone(),
            focused: request.focus,
            label: Some(SIDEBAR_LABEL.into()),
            title: Some(SIDEBAR_LABEL.into()),
            cwd: None,
            foreground_cwd: None,
            tokens: BTreeMap::new(),
        };
        inner.panes.push(pane);
        if let Some(layout) = inner.layouts.get_mut(&target.tab_id) {
            let Some(target_rect) = layout
                .panes
                .iter()
                .find(|item| item.pane_id == target.pane_id)
                .map(|item| item.rect)
            else {
                return Ok(OpenedPane {
                    pane_id: PaneId(id),
                });
            };
            let height = target_rect.height;
            let y = target_rect.y;
            let half = (target_rect.width / 2).max(1);
            let right_x = target_rect.x + half;
            let right_w = target_rect.width.saturating_sub(half);
            if let Some(target_pane) = layout
                .panes
                .iter_mut()
                .find(|item| item.pane_id == target.pane_id)
            {
                target_pane.rect.width = half;
            }
            layout.panes.push(LayoutPane {
                pane_id: id.clone(),
                focused: request.focus,
                rect: LayoutRect {
                    x: right_x,
                    y,
                    width: right_w.max(1),
                    height,
                },
            });
            layout.splits.push(LayoutSplit {
                id: format!("split_{id}"),
                direction: "right".into(),
                ratio: 0.5,
                rect: LayoutRect {
                    x: target_rect.x,
                    y,
                    width: half + right_w.max(1),
                    height,
                },
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
        let mut inner = self.inner.lock().expect("fake herdr lock");
        for layout in inner.layouts.values_mut() {
            let left = layout
                .panes
                .iter()
                .position(|pane| pane.pane_id == source.0);
            let right = layout
                .panes
                .iter()
                .position(|pane| pane.pane_id == target.0);
            if let (Some(a), Some(b)) = (left, right) {
                let rect_a = layout.panes[a].rect;
                layout.panes[a].rect = layout.panes[b].rect;
                layout.panes[b].rect = rect_a;
            }
        }
        Ok(())
    }

    fn focus_pane(&self, pane_id: &PaneId) -> Result<(), AppError> {
        self.record("pane.focus", pane_id.as_str());
        let mut inner = self.inner.lock().expect("fake herdr lock");
        inner.focused = Some(pane_id.0.clone());
        for pane in &mut inner.panes {
            pane.focused = pane.pane_id == pane_id.0;
        }
        Ok(())
    }

    fn resize_pane(&self, pane_id: &PaneId, direction: &str, amount: f64) -> Result<(), AppError> {
        self.record(
            "pane.resize",
            format!("{} {direction} {amount}", pane_id.as_str()),
        );
        let mut inner = self.inner.lock().expect("fake herdr lock");
        for layout in inner.layouts.values_mut() {
            let Some(index) = layout
                .panes
                .iter()
                .position(|pane| pane.pane_id == pane_id.0)
            else {
                continue;
            };
            let split_w = layout
                .splits
                .iter()
                .filter(|split| split.direction == "right")
                .map(|split| split.rect.width)
                .max()
                .unwrap_or(layout.area.width);
            let delta = (amount * f64::from(split_w)).round() as i32;
            let pane = &mut layout.panes[index];
            let new_w = if direction == "right" {
                i32::from(pane.rect.width) + delta
            } else {
                i32::from(pane.rect.width) - delta
            }
            .clamp(1, i32::from(split_w.saturating_sub(1))) as u16;
            pane.rect.width = new_w;
        }
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
            pane.label = Some(SIDEBAR_LABEL.into());
        }
        Ok(())
    }

    fn close_plugin_pane(&self, pane_id: &PaneId) -> Result<(), AppError> {
        self.record("plugin.pane.close", pane_id.as_str());
        let mut inner = self.inner.lock().expect("fake herdr lock");
        inner.panes.retain(|pane| pane.pane_id != pane_id.0);
        inner.exited.push(pane_id.0.clone());
        for layout in inner.layouts.values_mut() {
            layout.panes.retain(|pane| pane.pane_id != pane_id.0);
        }
        Ok(())
    }

    fn create_tab(&self, request: CreateTab) -> Result<CreatedTab, AppError> {
        self.record(
            "tab.create",
            format!(
                "{} {} focus={}",
                request.workspace_id,
                request.cwd.display(),
                request.focus
            ),
        );
        let mut inner = self.inner.lock().expect("fake herdr lock");
        if inner.create_fail {
            return Err(AppError::herdr("tab.create", "failed", "cannot create tab"));
        }
        let tab_id = inner
            .forced_tab_id
            .clone()
            .unwrap_or_else(|| format!("tab-{}", inner.next_tab));
        inner.next_tab += 1;
        inner.forced_tab_id = None;
        let root_pane_id = if inner.missing_root {
            String::new()
        } else {
            format!("{tab_id}:p1")
        };
        inner.created_tabs.push(FakeTab {
            tab_id: tab_id.clone(),
            root_pane_id: root_pane_id.clone(),
            workspace_id: request.workspace_id.clone(),
            cwd: request.cwd.clone(),
            label: request.label.clone(),
            focus: request.focus,
        });
        Ok(CreatedTab {
            tab_id: tab_id.into(),
            root_pane_id: root_pane_id.into(),
        })
    }

    fn send_input(&self, pane_id: &PaneId, text: &str, keys: &[&str]) -> Result<(), AppError> {
        self.record(
            "pane.send_input",
            format!("{} {text} keys={}", pane_id.as_str(), keys.join(",")),
        );
        let inner = self.inner.lock().expect("fake herdr lock");
        if inner.send_timeout {
            return Err(AppError::uncertain(
                "pane.send_input",
                "socket timed out after the request was written; inspect the layout before retrying",
            ));
        }
        let cwd = inner
            .created_tabs
            .iter()
            .find(|tab| tab.root_pane_id == pane_id.as_str())
            .map(|tab| tab.cwd.clone());
        let shell_path = inner.shell_path.clone();
        let argv_file = inner.argv_file.clone();
        drop(inner);
        if let (Some(path_prefix), Some(argv_file), Some(cwd)) = (shell_path, argv_file, cwd) {
            let path = format!(
                "{}:{}",
                path_prefix.display(),
                std::env::var("PATH").unwrap_or_default()
            );
            let status = Command::new("sh")
                .arg("-c")
                .arg(text)
                .current_dir(cwd)
                .env("PATH", path)
                .env("HERDR_NPM_ARGV", &argv_file)
                .status()
                .map_err(|error| AppError::Io {
                    message: error.to_string(),
                })?;
            if !status.success() {
                return Err(AppError::Io {
                    message: format!("fake manager exited {status}"),
                });
            }
        }
        Ok(())
    }
}
