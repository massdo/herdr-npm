use crate::domain::error::AppError;
use crate::domain::ids::PaneId;
use crate::domain::pane::{OriginContext, PaneInfo};

/// Two-state sidebar decision for the captured origin tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarDecision {
    Open,
    Close { pane_id: PaneId },
}

/// Inspect `pane.list` for the origin tab. Recognition is the session token
/// only; the visible `npm` label is ignored.
pub fn decide_sidebar(
    panes: &[PaneInfo],
    origin: &OriginContext,
) -> Result<SidebarDecision, AppError> {
    let origin_pane = panes
        .iter()
        .find(|pane| pane.pane_id == origin.pane_id.0)
        .ok_or(AppError::OriginChanged)?;
    if origin_pane.tab_id != origin.tab_id.0 || origin_pane.workspace_id != origin.workspace_id.0 {
        return Err(AppError::OriginChanged);
    }

    let recognised: Vec<&PaneInfo> = panes
        .iter()
        .filter(|pane| {
            pane.workspace_id == origin.workspace_id.0
                && pane.tab_id == origin.tab_id.0
                && pane.is_herdr_npm_sidebar()
        })
        .collect();

    match recognised.as_slice() {
        [] => Ok(SidebarDecision::Open),
        [one] => Ok(SidebarDecision::Close { pane_id: one.id() }),
        _ => Err(AppError::SeveralSidebars),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::{TabId, WorkspaceId};
    use crate::domain::{SIDEBAR_TOKEN_KEY, SIDEBAR_TOKEN_VALUE};
    use std::collections::BTreeMap;

    fn pane(id: &str, tab: &str, token: bool, label: Option<&str>) -> PaneInfo {
        let mut tokens = BTreeMap::new();
        if token {
            tokens.insert(SIDEBAR_TOKEN_KEY.into(), SIDEBAR_TOKEN_VALUE.into());
        }
        PaneInfo {
            pane_id: id.into(),
            workspace_id: "w1".into(),
            tab_id: tab.into(),
            focused: false,
            label: label.map(str::to_string),
            title: None,
            cwd: None,
            foreground_cwd: None,
            tokens,
        }
    }

    fn origin(tab: &str, pane_id: &str) -> OriginContext {
        OriginContext {
            workspace_id: WorkspaceId("w1".into()),
            tab_id: TabId(tab.into()),
            pane_id: PaneId(pane_id.into()),
            foreground_cwd: None,
            cwd: None,
        }
    }

    #[test]
    fn no_token_means_open() {
        let panes = vec![pane("editor", "t1", false, Some("editor"))];
        assert_eq!(
            decide_sidebar(&panes, &origin("t1", "editor")).unwrap(),
            SidebarDecision::Open
        );
    }

    #[test]
    fn npm_label_without_token_is_not_recognised() {
        let panes = vec![pane("foreign", "t1", false, Some("npm"))];
        assert_eq!(
            decide_sidebar(&panes, &origin("t1", "foreign")).unwrap(),
            SidebarDecision::Open
        );
    }

    #[test]
    fn one_token_means_close() {
        let panes = vec![
            pane("editor", "t1", false, None),
            pane("sb", "t1", true, Some("npm")),
        ];
        assert_eq!(
            decide_sidebar(&panes, &origin("t1", "editor")).unwrap(),
            SidebarDecision::Close {
                pane_id: PaneId("sb".into()),
            }
        );
    }

    #[test]
    fn token_on_another_tab_is_ignored() {
        let panes = vec![
            pane("editor", "t1", false, None),
            pane("sb", "t2", true, Some("npm")),
        ];
        assert_eq!(
            decide_sidebar(&panes, &origin("t1", "editor")).unwrap(),
            SidebarDecision::Open
        );
    }

    #[test]
    fn several_tokens_are_an_error() {
        let panes = vec![
            pane("editor", "t1", false, None),
            pane("sb1", "t1", true, None),
            pane("sb2", "t1", true, None),
        ];
        assert_eq!(
            decide_sidebar(&panes, &origin("t1", "editor")).unwrap_err(),
            AppError::SeveralSidebars
        );
    }

    #[test]
    fn missing_origin_pane_is_changed() {
        let panes = vec![pane("other", "t1", false, None)];
        assert_eq!(
            decide_sidebar(&panes, &origin("t1", "editor")).unwrap_err(),
            AppError::OriginChanged
        );
    }
}
