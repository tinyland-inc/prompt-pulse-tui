use std::path::PathBuf;

use serde::Deserialize;

use crate::config::TuiConfig;

/// Compile-time build metadata baked in via build.rs.
pub struct TuiBuildInfo {
    pub version: &'static str,
    pub git_sha: &'static str,
    pub dirty: bool,
}

impl TuiBuildInfo {
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            git_sha: env!("BUILD_GIT_SHA"),
            dirty: env!("BUILD_DIRTY") == "true",
        }
    }

    pub fn sha_display(&self) -> String {
        if self.dirty {
            format!("{}*", self.git_sha)
        } else {
            self.git_sha.to_string()
        }
    }
}

/// Runtime component version info read from the daemon's cache files.
#[derive(Debug, Default)]
pub struct ComponentVersions {
    pub daemon: Option<DaemonVersion>,
    pub hm_generation: Option<String>,
    pub nix_version: Option<String>,
    pub flake_inputs: Vec<FlakeInput>,
}

#[derive(Debug, Deserialize)]
pub struct DaemonVersion {
    pub version: String,
    pub git_sha: String,
    pub go_version: String,
}

#[derive(Debug, Clone)]
pub struct FlakeInput {
    pub name: String,
    pub rev: String,
}

/// Read daemon version from its status file.
pub fn read_daemon_version(cfg: &TuiConfig) -> Option<DaemonVersion> {
    let path = cfg.cache_dir().join("daemon-status.json");
    let contents = std::fs::read_to_string(&path).ok()?;
    // The daemon status file has a "version" object.
    let v: serde_json::Value = serde_json::from_str(&contents).ok()?;
    Some(DaemonVersion {
        version: v.get("version")?.as_str()?.to_string(),
        git_sha: v
            .get("git_sha")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string(),
        go_version: v
            .get("go_version")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// Read home-manager generation number from the profile.
pub fn read_hm_generation() -> Option<String> {
    let home = dirs::home_dir()?;
    let profile = home.join(".local/state/nix/profiles/home-manager");
    let target = std::fs::read_link(&profile).ok()?;
    // Profile symlink target looks like: home-manager-42-link
    let name = target.file_name()?.to_str()?;
    // Extract generation number.
    name.strip_prefix("home-manager-")
        .and_then(|s| s.strip_suffix("-link"))
        .map(|s| s.to_string())
}

/// Read Nix version.
pub fn read_nix_version() -> Option<String> {
    let output = std::process::Command::new("nix")
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let ver = String::from_utf8(output.stdout).ok()?;
    Some(ver.trim().to_string())
}

/// Read flake.lock and extract input revisions.
pub fn read_flake_inputs() -> Vec<FlakeInput> {
    // Try to find flake.lock relative to the crush-dots repo.
    let candidates = [
        dirs::home_dir().map(|h| h.join("git/crush-dots/flake.lock")),
        Some(PathBuf::from("/etc/crush-dots/flake.lock")),
    ];

    for candidate in candidates.iter().flatten() {
        if let Some(inputs) = parse_flake_lock(candidate) {
            return inputs;
        }
    }
    Vec::new()
}

fn parse_flake_lock(path: &PathBuf) -> Option<Vec<FlakeInput>> {
    let contents = std::fs::read_to_string(path).ok()?;
    let lock: serde_json::Value = serde_json::from_str(&contents).ok()?;
    let nodes = lock.get("nodes")?.as_object()?;

    let mut inputs = Vec::new();
    // Key inputs we care about.
    let interesting = [
        "nixpkgs",
        "nixpkgs-unstable",
        "home-manager",
        "sops-nix",
        "fenix",
    ];

    for name in &interesting {
        if let Some(node) = nodes.get(*name) {
            if let Some(locked) = node.get("locked") {
                let rev = locked
                    .get("rev")
                    .and_then(|r| r.as_str())
                    .unwrap_or("")
                    .to_string();
                if !rev.is_empty() {
                    inputs.push(FlakeInput {
                        name: name.to_string(),
                        rev: rev[..8.min(rev.len())].to_string(),
                    });
                }
            }
        }
    }
    Some(inputs)
}

/// Collect all component version info.
pub fn collect_versions(cfg: &TuiConfig) -> ComponentVersions {
    ComponentVersions {
        daemon: read_daemon_version(cfg),
        hm_generation: read_hm_generation(),
        nix_version: read_nix_version(),
        flake_inputs: read_flake_inputs(),
    }
}
