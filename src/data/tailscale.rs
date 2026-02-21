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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tailscale_null_peers() {
        let json = r#"{"peers": null, "tailnet_name": "test"}"#;
        let status: TailscaleStatus = serde_json::from_str(json).unwrap();
        assert!(status.peers.is_empty());
        assert_eq!(status.tailnet_name, "test");
    }

    #[test]
    fn test_tailscale_null_tags() {
        let json =
            r#"{"peers": [{"hostname": "test", "tags": null, "tailscale_ips": ["100.1.2.3"]}]}"#;
        let status: TailscaleStatus = serde_json::from_str(json).unwrap();
        assert!(status.peers[0].tags.is_empty());
    }

    #[test]
    fn test_tailscale_null_ips() {
        let json = r#"{"peers": [{"hostname": "test", "tailscale_ips": null}]}"#;
        let status: TailscaleStatus = serde_json::from_str(json).unwrap();
        assert!(status.peers[0].tailscale_ips.is_empty());
    }

    #[test]
    fn test_online_peers_sorted() {
        let json = r#"{
            "peers": [
                {"hostname": "zebra", "online": true},
                {"hostname": "apple", "online": false},
                {"hostname": "banana", "online": true}
            ]
        }"#;
        let status: TailscaleStatus = serde_json::from_str(json).unwrap();
        let online = status.online_peers_sorted();
        assert_eq!(online.len(), 2);
        assert_eq!(online[0].hostname, "banana");
        assert_eq!(online[1].hostname, "zebra");
    }
}
