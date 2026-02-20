use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Mirrors Go k8s.ClusterStatus (daemon cache).
#[derive(Debug, Deserialize)]
pub struct K8sStatus {
    #[serde(default)]
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
    #[serde(default)]
    pub nodes: Vec<NodeInfo>,
    #[serde(default)]
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
    #[serde(default)]
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
