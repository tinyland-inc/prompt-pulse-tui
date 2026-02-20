use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Mirrors the Go tailscale.Status struct (daemon cache).
#[derive(Debug, Deserialize)]
pub struct TailscaleStatus {
    pub self_node: Option<PeerInfo>,
    #[serde(rename = "self")]
    pub self_info: Option<PeerInfo>,
    #[serde(default, deserialize_with = "crate::data::null_to_default")]
    pub peers: Vec<PeerInfo>,
    #[serde(default)]
    pub magic_dns_suffix: String,
    #[serde(default)]
    pub tailnet_name: String,
    #[serde(default)]
    pub online_peers: i32,
    #[serde(default)]
    pub total_peers: i32,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct PeerInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub dns_name: String,
    #[serde(default)]
    pub os: String,
    #[serde(default, deserialize_with = "crate::data::null_to_default")]
    pub tailscale_ips: Vec<String>,
    #[serde(default)]
    pub online: bool,
    pub last_seen: Option<DateTime<Utc>>,
    #[serde(default)]
    pub exit_node: bool,
    #[serde(default)]
    pub exit_node_option: bool,
    #[serde(default, deserialize_with = "crate::data::null_to_default")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub rx_bytes: i64,
    #[serde(default)]
    pub tx_bytes: i64,
}

impl TailscaleStatus {
    /// Only online peers, sorted by hostname.
    pub fn online_peers_sorted(&self) -> Vec<&PeerInfo> {
        let mut peers: Vec<&PeerInfo> = self.peers.iter().filter(|p| p.online).collect();
        peers.sort_by(|a, b| a.hostname.cmp(&b.hostname));
        peers
    }
}
