use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Mirrors Go k8s.ClusterStatus (daemon cache).
#[derive(Debug, Deserialize)]
pub struct K8sStatus {
    #[serde(default, deserialize_with = "crate::data::null_to_default")]
    pub clusters: Vec<ClusterInfo>,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ClusterInfo {
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub connected: bool,
    #[serde(default)]
    pub error: String,
    #[serde(default, deserialize_with = "crate::data::null_to_default")]
    pub nodes: Vec<NodeInfo>,
    #[serde(default, deserialize_with = "crate::data::null_to_default")]
    pub namespaces: Vec<NamespaceInfo>,
    #[serde(default)]
    pub total_pods: i32,
    #[serde(default)]
    pub running_pods: i32,
    #[serde(default)]
    pub pending_pods: i32,
    #[serde(default)]
    pub failed_pods: i32,
}

#[derive(Debug, Deserialize)]
pub struct NodeInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub ready: bool,
    #[serde(default, deserialize_with = "crate::data::null_to_default")]
    pub roles: Vec<String>,
    #[serde(default)]
    pub cpu_capacity: String,
    #[serde(default)]
    pub mem_capacity: String,
    #[serde(default)]
    pub pod_count: i32,
}

#[derive(Debug, Deserialize)]
pub struct NamespaceInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pod_counts: PodCounts,
}

#[derive(Debug, Default, Deserialize)]
pub struct PodCounts {
    #[serde(default)]
    pub total: i32,
    #[serde(default)]
    pub running: i32,
    #[serde(default)]
    pub pending: i32,
    #[serde(default)]
    pub failed: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k8s_null_clusters() {
        let json = r#"{"clusters": null}"#;
        let status: K8sStatus = serde_json::from_str(json).unwrap();
        assert!(status.clusters.is_empty());
    }

    #[test]
    fn test_k8s_null_nodes() {
        let json = r#"{"clusters": [{"context": "test", "nodes": null, "namespaces": []}]}"#;
        let status: K8sStatus = serde_json::from_str(json).unwrap();
        assert!(status.clusters[0].nodes.is_empty());
    }

    #[test]
    fn test_k8s_null_roles() {
        let json = r#"{"clusters": [{"context": "test", "nodes": [{"name": "n1", "roles": null}], "namespaces": []}]}"#;
        let status: K8sStatus = serde_json::from_str(json).unwrap();
        assert!(status.clusters[0].nodes[0].roles.is_empty());
    }
}
