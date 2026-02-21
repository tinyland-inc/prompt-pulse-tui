use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Persisted state from the Go claudepersonal collector.
/// File: ~/.cache/prompt-pulse/claude-personal.json
#[derive(Debug, Deserialize)]
pub struct ClaudePersonalState {
    pub messages: Vec<PersonalMessage>,
    #[serde(default = "default_window_hours")]
    pub window_hours: i32,
    #[serde(default = "default_message_limit")]
    pub message_limit: i32,
    #[serde(default)]
    pub last_scan: String,
}

#[derive(Debug, Deserialize)]
pub struct PersonalMessage {
    pub ts: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub source: String,
}

fn default_window_hours() -> i32 {
    5
}
fn default_message_limit() -> i32 {
    45
}

/// Computed report for the TUI widget.
#[derive(Debug, Clone)]
pub struct ClaudePersonalReport {
    pub messages_in_window: i32,
    pub message_limit: i32,
    pub window_hours: i32,
    /// Seconds until the oldest message in the window expires (0 if under limit).
    pub next_slot_secs: i64,
}

/// Compute a usage report from the persisted state.
pub fn compute_report(state: &ClaudePersonalState) -> ClaudePersonalReport {
    let now = Utc::now();
    let window = chrono::Duration::hours(state.window_hours as i64);
    let cutoff = now - window;

    let mut in_window: Vec<DateTime<Utc>> = state
        .messages
        .iter()
        .filter_map(|m| DateTime::parse_from_rfc3339(&m.ts).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .filter(|dt| *dt > cutoff)
        .collect();

    in_window.sort();

    let messages_in_window = in_window.len() as i32;

    // Time until oldest message in window expires.
    let next_slot_secs = if messages_in_window >= state.message_limit && !in_window.is_empty() {
        let oldest = in_window[0];
        let expires_at = oldest + window;
        let remaining = expires_at - now;
        remaining.num_seconds().max(0)
    } else {
        0
    };

    ClaudePersonalReport {
        messages_in_window,
        message_limit: state.message_limit,
        window_hours: state.window_hours,
        next_slot_secs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(timestamps: Vec<String>, window_hours: i32, limit: i32) -> ClaudePersonalState {
        ClaudePersonalState {
            messages: timestamps
                .into_iter()
                .map(|ts| PersonalMessage {
                    ts,
                    model: None,
                    source: "test".into(),
                })
                .collect(),
            window_hours,
            message_limit: limit,
            last_scan: String::new(),
        }
    }

    #[test]
    fn test_compute_report_under_limit() {
        let now = chrono::Utc::now();
        let ts = (now - chrono::Duration::minutes(30)).to_rfc3339();
        let state = make_state(vec![ts], 5, 45);
        let report = compute_report(&state);
        assert_eq!(report.messages_in_window, 1);
        assert_eq!(report.next_slot_secs, 0);
    }

    #[test]
    fn test_compute_report_at_limit() {
        let now = chrono::Utc::now();
        let timestamps: Vec<String> = (0..45)
            .map(|i| (now - chrono::Duration::minutes(i * 5)).to_rfc3339())
            .collect();
        let state = make_state(timestamps, 5, 45);
        let report = compute_report(&state);
        assert_eq!(report.messages_in_window, 45);
        assert!(report.next_slot_secs > 0);
    }

    #[test]
    fn test_compute_report_empty() {
        let state = make_state(vec![], 5, 45);
        let report = compute_report(&state);
        assert_eq!(report.messages_in_window, 0);
        assert_eq!(report.next_slot_secs, 0);
    }

    #[test]
    fn test_compute_report_all_expired() {
        let old = (chrono::Utc::now() - chrono::Duration::hours(10)).to_rfc3339();
        let state = make_state(vec![old], 5, 45);
        let report = compute_report(&state);
        assert_eq!(report.messages_in_window, 0);
    }

    #[test]
    fn test_compute_report_mixed() {
        let now = chrono::Utc::now();
        let recent = (now - chrono::Duration::minutes(30)).to_rfc3339();
        let old = (now - chrono::Duration::hours(10)).to_rfc3339();
        let state = make_state(vec![recent, old], 5, 45);
        let report = compute_report(&state);
        assert_eq!(report.messages_in_window, 1);
    }
}
