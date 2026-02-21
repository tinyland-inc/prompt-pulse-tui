pub mod billing;
pub mod buildinfo;
pub mod cache;
pub mod claude;
pub mod claudepersonal;
pub mod k8s;
pub mod sysmetrics;
pub mod tailscale;
pub mod waifu;
pub mod waifu_client;

pub use billing::BillingReport;
pub use cache::CacheReader;
pub use claude::ClaudeUsage;
pub use k8s::K8sStatus;
pub use sysmetrics::SysMetrics;
pub use tailscale::TailscaleStatus;

/// Deserialize JSON `null` as the type's default value.
/// Go serializes nil slices/maps as `null` rather than `[]`/`{}`,
/// but serde's `#[serde(default)]` only handles *missing* fields.
pub fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + serde::Deserialize<'de>,
{
    use serde::Deserialize;
    Option::<T>::deserialize(deserializer).map(|v| v.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestStruct {
        #[serde(default, deserialize_with = "null_to_default")]
        items: Vec<String>,
    }

    #[test]
    fn test_null_to_default_vec_null() {
        let json = r#"{"items": null}"#;
        let s: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(s.items, Vec::<String>::new());
    }

    #[test]
    fn test_null_to_default_vec_missing() {
        let json = r#"{}"#;
        let s: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(s.items, Vec::<String>::new());
    }

    #[test]
    fn test_null_to_default_vec_present() {
        let json = r#"{"items": ["a", "b"]}"#;
        let s: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(s.items, vec!["a", "b"]);
    }

    #[test]
    fn test_null_to_default_empty_array() {
        let json = r#"{"items": []}"#;
        let s: TestStruct = serde_json::from_str(json).unwrap();
        assert!(s.items.is_empty());
    }
}
