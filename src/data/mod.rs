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
