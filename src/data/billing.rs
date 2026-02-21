use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Mirrors Go billing.BillingReport (daemon cache).
#[derive(Debug, Deserialize)]
pub struct BillingReport {
    #[serde(default, deserialize_with = "crate::data::null_to_default")]
    pub providers: Vec<ProviderBilling>,
    #[serde(default)]
    pub total_monthly_usd: f64,
    #[serde(default)]
    pub budget_usd: f64,
    #[serde(default)]
    pub budget_percent: f64,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderBilling {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub connected: bool,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub month_to_date: f64,
    #[serde(default)]
    pub balance: f64,
    #[serde(default, deserialize_with = "crate::data::null_to_default")]
    pub resources: Vec<ResourceCost>,
}

#[derive(Debug, Deserialize)]
pub struct ResourceCost {
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type", default)]
    pub resource_type: String,
    #[serde(default)]
    pub monthly_cost: f64,
    #[serde(default)]
    pub hourly_cost: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_billing_null_providers() {
        let json = r#"{"providers": null, "total_monthly_usd": 0}"#;
        let report: BillingReport = serde_json::from_str(json).unwrap();
        assert!(report.providers.is_empty());
    }

    #[test]
    fn test_billing_null_resources() {
        let json = r#"{"providers": [{"name": "test", "resources": null}]}"#;
        let report: BillingReport = serde_json::from_str(json).unwrap();
        assert!(report.providers[0].resources.is_empty());
    }
}
