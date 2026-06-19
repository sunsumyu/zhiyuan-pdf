use serde::{Deserialize, Serialize};

const COMMIT_SUPPRESS_MS: f64 = 120.0;
const SCROLL_DEBOUNCE_MS: f64 = 56.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderQueueAction {
    pub action: String,
    pub source: String,
    pub suppress: bool,
    pub scroll_debounce_ms: f64,
    pub pending_queue_effect: String,
    pub reject_reason: Option<String>,
}

pub fn resolve_queue_action(
    source: String,
    executing: bool,
    now_ms: f64,
    last_commit_ms: f64,
) -> RenderQueueAction {
    let normalized_source = normalize_render_source(source);
    let since_commit_ms = if now_ms.is_finite() && last_commit_ms.is_finite() {
        now_ms - last_commit_ms
    } else {
        COMMIT_SUPPRESS_MS
    };

    if normalized_source == "scroll"
        && since_commit_ms >= 0.0
        && since_commit_ms < COMMIT_SUPPRESS_MS
    {
        return RenderQueueAction {
            action: "suppress".to_string(),
            source: normalized_source,
            suppress: true,
            scroll_debounce_ms: SCROLL_DEBOUNCE_MS,
            pending_queue_effect: "none".to_string(),
            reject_reason: Some("recentCommit".to_string()),
        };
    }

    if !executing {
        return RenderQueueAction {
            action: "dispatch".to_string(),
            source: normalized_source,
            suppress: false,
            scroll_debounce_ms: SCROLL_DEBOUNCE_MS,
            pending_queue_effect: "none".to_string(),
            reject_reason: None,
        };
    }

    if normalized_source == "navigation" {
        RenderQueueAction {
            action: "replacePendingNavigation".to_string(),
            source: normalized_source,
            suppress: false,
            scroll_debounce_ms: SCROLL_DEBOUNCE_MS,
            pending_queue_effect: "replaceAll".to_string(),
            reject_reason: None,
        }
    } else {
        RenderQueueAction {
            action: "replacePendingNonNavigation".to_string(),
            source: normalized_source,
            suppress: false,
            scroll_debounce_ms: SCROLL_DEBOUNCE_MS,
            pending_queue_effect: "replaceNonNavigation".to_string(),
            reject_reason: None,
        }
    }
}

fn normalize_render_source(source: String) -> String {
    match source.as_str() {
        "navigation" | "scroll" | "zoom" | "editor" | "mutation" | "default" => source,
        _ => "default".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suppresses_scroll_immediately_after_commit() {
        let action = resolve_queue_action("scroll".to_string(), false, 150.0, 100.0);
        assert_eq!(action.action, "suppress");
        assert!(action.suppress);
        assert_eq!(action.scroll_debounce_ms, 56.0);
        assert_eq!(action.reject_reason.as_deref(), Some("recentCommit"));
    }

    #[test]
    fn dispatches_when_idle() {
        let action = resolve_queue_action("zoom".to_string(), false, 500.0, 0.0);
        assert_eq!(action.action, "dispatch");
        assert!(!action.suppress);
    }

    #[test]
    fn replaces_navigation_while_executing() {
        let action = resolve_queue_action("navigation".to_string(), true, 500.0, 0.0);
        assert_eq!(action.action, "replacePendingNavigation");
        assert_eq!(action.pending_queue_effect, "replaceAll");
    }

    #[test]
    fn replaces_non_navigation_while_executing() {
        let action = resolve_queue_action("scroll".to_string(), true, 500.0, 0.0);
        assert_eq!(action.action, "replacePendingNonNavigation");
        assert_eq!(action.pending_queue_effect, "replaceNonNavigation");
    }
}
