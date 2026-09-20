use crate::domain::error::AppError;
use crate::domain::ids::TabId;
use crate::domain::pane::{LayoutSnapshot, PaneInfo, ResizeStep};
use crate::domain::{MAX_SIDEBAR_SHARE, MIN_SIDEBAR_SHARE, PREFERRED_OUTER_COLUMNS};

/// Working-pane target: leftmost, then highest, then pane id. herdr-npm and
/// the explorer are never split.
pub fn pick_working_target<'a>(
    panes: &'a [PaneInfo],
    layout: &LayoutSnapshot,
    origin_tab: &TabId,
) -> Result<&'a PaneInfo, AppError> {
    let mut candidates: Vec<(&PaneInfo, (u16, u16, &str))> = Vec::new();
    for pane in panes {
        if pane.tab_id != origin_tab.0 || pane.is_excluded_working_target() {
            continue;
        }
        let Some(layout_pane) = layout
            .panes
            .iter()
            .find(|item| item.pane_id == pane.pane_id)
        else {
            continue;
        };
        candidates.push((
            pane,
            (
                layout_pane.rect.x,
                layout_pane.rect.y,
                pane.pane_id.as_str(),
            ),
        ));
    }
    candidates.sort_by_key(|(_, key)| *key);
    candidates
        .into_iter()
        .map(|(pane, _)| pane)
        .next()
        .ok_or(AppError::NoWorkingTarget)
}

/// Ratio delta that moves the left-docked sidebar toward 32 outer columns,
/// clamped to Herdr's usable share of the local split.
pub fn preferred_left_resize(layout: &LayoutSnapshot, pane_id: &str) -> Option<ResizeStep> {
    let pane_rect = layout
        .panes
        .iter()
        .find(|pane| pane.pane_id == pane_id)?
        .rect;
    let divider_x = i64::from(pane_rect.x) + i64::from(pane_rect.width);
    let split = layout
        .splits
        .iter()
        .filter(|split| split.direction == "right")
        .filter(|split| split.rect.width > 0)
        .filter(|split| {
            let split_divider = i64::from(split.rect.x)
                + (f64::from(split.rect.width) * split.ratio).round() as i64;
            i64::from(split.rect.x) <= i64::from(pane_rect.x)
                && (split_divider - divider_x).abs() <= 2
        })
        .min_by_key(|split| split.rect.width)?;

    let requested = i64::from(PREFERRED_OUTER_COLUMNS);
    let min = (f64::from(split.rect.width) * MIN_SIDEBAR_SHARE).ceil() as i64;
    let max = (f64::from(split.rect.width) * MAX_SIDEBAR_SHARE).floor() as i64;
    let target_w = requested.clamp(min, max.max(min));
    let delta = (target_w - i64::from(pane_rect.width)) as f64 / f64::from(split.rect.width);
    if delta.abs() < 0.005 {
        return None;
    }
    Some(ResizeStep {
        direction: if delta > 0.0 { "right" } else { "left" },
        amount: delta.abs(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::pane::{LayoutPane, LayoutRect, LayoutSplit};
    use std::collections::BTreeMap;

    fn pane(id: &str, tab: &str) -> PaneInfo {
        PaneInfo {
            pane_id: id.into(),
            workspace_id: "w1".into(),
            tab_id: tab.into(),
            focused: false,
            label: None,
            title: None,
            cwd: None,
            foreground_cwd: None,
            tokens: BTreeMap::new(),
        }
    }

    #[test]
    fn working_target_skips_explorer_and_picks_leftmost() {
        let mut explorer = pane("w1:p-exp", "w1:t1");
        explorer.label = Some("Sidebar".into());
        let editor = pane("w1:p-ed", "w1:t1");
        let panes = vec![explorer, editor];
        let layout = LayoutSnapshot {
            workspace_id: "w1".into(),
            tab_id: "w1:t1".into(),
            area: LayoutRect {
                x: 0,
                y: 0,
                width: 120,
                height: 40,
            },
            focused_pane_id: "w1:p-ed".into(),
            panes: vec![
                LayoutPane {
                    pane_id: "w1:p-exp".into(),
                    focused: false,
                    rect: LayoutRect {
                        x: 0,
                        y: 0,
                        width: 32,
                        height: 40,
                    },
                },
                LayoutPane {
                    pane_id: "w1:p-ed".into(),
                    focused: true,
                    rect: LayoutRect {
                        x: 32,
                        y: 0,
                        width: 88,
                        height: 40,
                    },
                },
            ],
            splits: vec![LayoutSplit {
                id: "s".into(),
                direction: "right".into(),
                ratio: 32.0 / 120.0,
                rect: LayoutRect {
                    x: 0,
                    y: 0,
                    width: 120,
                    height: 40,
                },
            }],
        };
        let chosen = pick_working_target(&panes, &layout, &TabId("w1:t1".into())).unwrap();
        assert_eq!(chosen.pane_id, "w1:p-ed");
    }
}
