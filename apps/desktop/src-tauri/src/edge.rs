use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ScreenEdge {
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct EdgeConfig {
    pub peer_id: String,
    pub edge: ScreenEdge,
}
#[derive(Default)]
pub(crate) struct EdgeState(pub Mutex<Option<EdgeConfig>>);

/// Leave native title-bar controls and the taskbar/Dock unobstructed.
pub(crate) fn bounds(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    edge: ScreenEdge,
) -> (f64, f64, f64, f64) {
    let strip = 24.0_f64.min(width);
    let gutter = 64.0_f64.min(height / 4.0);
    let left = match edge {
        ScreenEdge::Left => x,
        ScreenEdge::Right => x + width - strip,
    };
    (left, y + gutter, strip, height - 2.0 * gutter)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn right_strip_does_not_cover_close_button_or_taskbar() {
        assert_eq!(
            bounds(0., 0., 1920., 1080., ScreenEdge::Right),
            (1896., 64., 24., 952.)
        );
    }
    #[test]
    fn negative_monitor_origin_and_small_screens_remain_in_bounds() {
        assert_eq!(
            bounds(-800., -100., 800., 200., ScreenEdge::Left),
            (-800., -50., 24., 100.)
        );
    }
    #[test]
    fn secondary_window_has_event_permissions_without_main_window_privileges() {
        let cap: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/edge-drop.json")).unwrap();
        assert_eq!(cap["windows"], serde_json::json!(["edge-drop"]));
        assert_eq!(
            cap["permissions"],
            serde_json::json!(["core:event:allow-listen", "core:event:allow-unlisten"])
        );
    }
}
