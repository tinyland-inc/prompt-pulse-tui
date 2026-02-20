use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::data::claudepersonal::{self, ClaudePersonalReport, ClaudePersonalState};
use crate::data::{BillingReport, ClaudeUsage, K8sStatus, TailscaleStatus};

const MAX_CACHE_AGE: Duration = Duration::from_secs(300); // 5 minutes

/// Reads JSON cache files written by the Go daemon.
pub struct CacheReader {
    dir: PathBuf,
}

impl CacheReader {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn read_tailscale(&self) -> Option<TailscaleStatus> {
        self.read_json("tailscale")
    }

    pub fn read_claude(&self) -> Option<ClaudeUsage> {
        self.read_json("claude")
    }

    pub fn read_billing(&self) -> Option<BillingReport> {
        self.read_json("billing")
    }

    pub fn read_k8s(&self) -> Option<K8sStatus> {
        self.read_json("k8s")
    }

    /// Read the claude personal state file (written by Go collector, no max age).
    pub fn read_claude_personal(&self) -> Option<ClaudePersonalReport> {
        let state: ClaudePersonalState = self.read_json("claude-personal")?;
        Some(claudepersonal::compute_report(&state))
    }

    fn read_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let path = self.dir.join(format!("{key}.json"));
        let meta = std::fs::metadata(&path).ok()?;
        let modified = meta.modified().ok()?;
        if SystemTime::now()
            .duration_since(modified)
            .unwrap_or(Duration::MAX)
            > MAX_CACHE_AGE
        {
            return None;
        }
        let data = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }
}
