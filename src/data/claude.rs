use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Mirrors Go claude.UsageReport (daemon cache).
#[derive(Debug, Deserialize)]
pub struct ClaudeUsage {
    #[serde(default, deserialize_with = "crate::data::null_to_default")]
    pub accounts: Vec<AccountUsage>,
    #[serde(default)]
    pub total_cost_usd: f64,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct AccountUsage {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub organization_id: String,
    #[serde(default)]
    pub connected: bool,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub current_month: MonthUsage,
    #[serde(default)]
    pub previous_month: MonthUsage,
    #[serde(default, deserialize_with = "crate::data::null_to_default")]
    pub models: Vec<ModelUsage>,
    #[serde(default, deserialize_with = "crate::data::null_to_default")]
    pub workspaces: Vec<WorkspaceUsage>,
    #[serde(default)]
    pub daily_burn_rate: f64,
    #[serde(default)]
    pub projected_monthly: f64,
    #[serde(default)]
    pub days_remaining: i32,
}

#[derive(Debug, Default, Deserialize)]
pub struct MonthUsage {
    #[serde(default)]
    pub input_tokens: i64,
    #[serde(default)]
    pub output_tokens: i64,
    #[serde(default)]
    pub cache_creation_tokens: i64,
    #[serde(default)]
    pub cache_read_tokens: i64,
    #[serde(default)]
    pub cost_usd: f64,
}

#[derive(Debug, Deserialize)]
pub struct ModelUsage {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub input_tokens: i64,
    #[serde(default)]
    pub output_tokens: i64,
    #[serde(default)]
    pub cost_usd: f64,
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceUsage {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub input_tokens: i64,
    #[serde(default)]
    pub output_tokens: i64,
    #[serde(default)]
    pub cost_usd: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_null_accounts() {
        let json = r#"{"accounts": null, "total_cost_usd": 0}"#;
        let usage: ClaudeUsage = serde_json::from_str(json).unwrap();
        assert!(usage.accounts.is_empty());
    }

    #[test]
    fn test_claude_null_models() {
        let json = r#"{"accounts": [{"name": "test", "models": null, "workspaces": []}]}"#;
        let usage: ClaudeUsage = serde_json::from_str(json).unwrap();
        assert!(usage.accounts[0].models.is_empty());
    }

    #[test]
    fn test_claude_null_workspaces() {
        let json = r#"{"accounts": [{"name": "test", "models": [], "workspaces": null}]}"#;
        let usage: ClaudeUsage = serde_json::from_str(json).unwrap();
        assert!(usage.accounts[0].workspaces.is_empty());
    }
}
