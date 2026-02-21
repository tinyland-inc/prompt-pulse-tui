use std::collections::HashMap;
use sysinfo::{
    Components, CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System,
};

/// Real-time system metrics collected in-process (not from daemon cache).
pub struct SysMetrics {
    sys: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    /// Previous network counters for rate computation.
    prev_net: HashMap<String, (u64, u64)>,
}

/// Snapshot of system metrics for rendering.
pub struct SysSnapshot {
    pub hostname: String,
    pub os_name: String,
    pub kernel_version: String,
    pub cpu_brand: String,
    pub uptime_secs: u64,
    pub cpu_count: usize,
    pub cpu_usage: Vec<f32>,
    pub cpu_total: f32,
    pub mem_total: u64,
    pub mem_used: u64,
    pub mem_available: u64,
    pub mem_percent: f64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub disks: Vec<DiskInfo>,
    pub networks: Vec<NetInfo>,
    pub load_avg: [f64; 3],
    pub temperatures: Vec<TempInfo>,
    pub battery: Option<BatteryInfo>,
    pub nix_packages: usize,
    pub local_ip: String,
    pub process_count: usize,
    pub arch: String,
    pub cpu_freq_mhz: u64,
    pub cpu_freqs: Vec<u64>, // per-core frequency in MHz
}

pub struct TempInfo {
    pub label: String,
    pub temp_c: f32,
    pub max_c: f32,
}

pub struct BatteryInfo {
    pub percent: f32,
    pub charging: bool,
    pub source: String,                 // "AC Power" or "Battery Power"
    pub time_remaining: Option<String>, // e.g. "2:30" or "calculating"
}

pub struct DiskInfo {
    pub mount: String,
    pub fs_type: String,
    pub total: u64,
    pub used: u64,
    pub percent: f64,
    pub is_removable: bool,
}

pub struct NetInfo {
    pub name: String,
    pub kind: NetKind,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_rate: u64, // bytes/sec since last refresh
    pub tx_rate: u64, // bytes/sec since last refresh
}

#[derive(Debug, Clone, Copy)]
pub enum NetKind {
    Wifi,
    Ethernet,
    Virtual,
    Unknown,
}

impl NetKind {
    pub fn icon(&self) -> &str {
        match self {
            Self::Wifi => "W",
            Self::Ethernet => "E",
            Self::Virtual => "V",
            Self::Unknown => "?",
        }
    }
}

fn classify_interface(name: &str) -> NetKind {
    let n = name.to_lowercase();
    // macOS: en0 is typically Wi-Fi, en1+ can be Ethernet or Thunderbolt
    // Linux: wlan*/wlp* is Wi-Fi, eth*/enp* is Ethernet
    if n.starts_with("wlan") || n.starts_with("wlp") || n == "en0" {
        NetKind::Wifi
    } else if n.starts_with("eth") || n.starts_with("enp") || n.starts_with("en") {
        NetKind::Ethernet
    } else if n.starts_with("veth")
        || n.starts_with("docker")
        || n.starts_with("br-")
        || n.starts_with("cali")
    {
        NetKind::Virtual
    } else {
        NetKind::Unknown
    }
}

impl SysMetrics {
    /// Create a SysMetrics with minimal system data for headless testing.
    /// Does NOT perform expensive CPU refresh or full system enumeration.
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            sys: System::new(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            prev_net: HashMap::new(),
        }
    }

    pub fn collect() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_cpu_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();
        // Capture initial network counters.
        let prev_net: HashMap<String, (u64, u64)> = networks
            .iter()
            .map(|(name, data)| {
                (
                    name.clone(),
                    (data.total_received(), data.total_transmitted()),
                )
            })
            .collect();
        Self {
            sys,
            disks,
            networks,
            components,
            prev_net,
        }
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.disks.refresh();
        // Snapshot previous counters before refresh.
        self.prev_net = self
            .networks
            .iter()
            .map(|(name, data)| {
                (
                    name.clone(),
                    (data.total_received(), data.total_transmitted()),
                )
            })
            .collect();
        self.networks.refresh();
        self.components.refresh();
    }

    pub fn snapshot(&self) -> SysSnapshot {
        let cpu_usage: Vec<f32> = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        let cpu_total = if cpu_usage.is_empty() {
            0.0
        } else {
            cpu_usage.iter().sum::<f32>() / cpu_usage.len() as f32
        };

        let load = System::load_average();

        let disks: Vec<DiskInfo> = self
            .disks
            .iter()
            .filter(|d| {
                let mp = d.mount_point().to_string_lossy();
                // Filter to meaningful mounts.
                mp == "/"
                    || mp.starts_with("/home")
                    || mp.starts_with("/Users")
                    || mp == "/System/Volumes/Data"
                    || mp.starts_with("/Volumes")
            })
            .map(|d| {
                let total = d.total_space();
                let avail = d.available_space();
                let used = total.saturating_sub(avail);
                let percent = if total > 0 {
                    (used as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                DiskInfo {
                    mount: d.mount_point().to_string_lossy().to_string(),
                    fs_type: d.file_system().to_string_lossy().to_string(),
                    total,
                    used,
                    percent,
                    is_removable: d.is_removable(),
                }
            })
            .collect();

        let networks: Vec<NetInfo> = self
            .networks
            .iter()
            .filter(|(name, _)| !name.starts_with("lo") && !name.starts_with("utun"))
            .map(|(name, data)| {
                let rx = data.total_received();
                let tx = data.total_transmitted();
                let (prev_rx, prev_tx) = self
                    .prev_net
                    .get(name.as_str())
                    .copied()
                    .unwrap_or((rx, tx));
                NetInfo {
                    name: name.clone(),
                    kind: classify_interface(name),
                    rx_bytes: rx,
                    tx_bytes: tx,
                    rx_rate: rx.saturating_sub(prev_rx),
                    tx_rate: tx.saturating_sub(prev_tx),
                }
            })
            .collect();

        let cpu_brand = self
            .sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_default();

        let temperatures: Vec<TempInfo> = self
            .components
            .iter()
            .filter(|c| c.temperature().is_finite() && c.temperature() > 0.0)
            .map(|c| TempInfo {
                label: c.label().to_string(),
                temp_c: c.temperature(),
                max_c: c.max(),
            })
            .collect();

        SysSnapshot {
            hostname: System::host_name().unwrap_or_else(|| "unknown".into()),
            os_name: System::long_os_version()
                .unwrap_or_else(|| System::name().unwrap_or_default()),
            kernel_version: System::kernel_version().unwrap_or_default(),
            cpu_brand,
            uptime_secs: System::uptime(),
            cpu_count: self.sys.cpus().len(),
            cpu_usage,
            cpu_total,
            mem_total: self.sys.total_memory(),
            mem_used: self.sys.used_memory(),
            mem_available: self.sys.available_memory(),
            mem_percent: if self.sys.total_memory() > 0 {
                (self.sys.used_memory() as f64 / self.sys.total_memory() as f64) * 100.0
            } else {
                0.0
            },
            swap_total: self.sys.total_swap(),
            swap_used: self.sys.used_swap(),
            disks,
            networks,
            load_avg: [load.one, load.five, load.fifteen],
            temperatures,
            battery: get_battery_info(),
            nix_packages: get_nix_package_count(),
            local_ip: get_local_ip(),
            process_count: self.sys.processes().len(),
            arch: std::env::consts::ARCH.to_string(),
            cpu_freq_mhz: self.sys.cpus().first().map(|c| c.frequency()).unwrap_or(0),
            cpu_freqs: self.sys.cpus().iter().map(|c| c.frequency()).collect(),
        }
    }
}

/// Get battery info via `pmset -g batt` on macOS, or from /sys/class on Linux.
fn get_battery_info() -> Option<BatteryInfo> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("pmset")
            .args(["-g", "batt"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        // Line 1: "Now drawing from 'AC Power'" or "Now drawing from 'Battery Power'"
        let source = if text.contains("AC Power") {
            "AC Power".to_string()
        } else {
            "Battery".to_string()
        };
        // Line 2: "-InternalBattery-0 (id=...)	85%; charging; 2:30 remaining"
        for line in text.lines() {
            if line.contains("InternalBattery") {
                // Parse "85%"
                if let Some(pct_str) = line.split('\t').nth(1) {
                    if let Some(pct) = pct_str.split('%').next() {
                        if let Ok(percent) = pct.trim().parse::<f32>() {
                            let charging =
                                pct_str.contains("charging") && !pct_str.contains("not charging");
                            // Parse time remaining: "2:30 remaining" or "(no estimate)"
                            let time_remaining = if pct_str.contains("remaining") {
                                pct_str
                                    .split(';')
                                    .find(|s| s.contains("remaining"))
                                    .map(|s| s.trim().replace(" remaining", ""))
                            } else if pct_str.contains("(no estimate)") {
                                Some("calculating".into())
                            } else {
                                None
                            };
                            return Some(BatteryInfo {
                                percent,
                                charging,
                                source,
                                time_remaining,
                            });
                        }
                    }
                }
            }
        }
        // AC-only machines (Mac Mini, Mac Pro) have no battery line
        None
    }
    #[cfg(target_os = "linux")]
    {
        let capacity = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity").ok()?;
        let status = std::fs::read_to_string("/sys/class/power_supply/BAT0/status").ok()?;
        let percent: f32 = capacity.trim().parse().ok()?;
        let charging = status.trim() == "Charging";
        let source = if charging { "AC Power" } else { "Battery" }.to_string();
        // Try to read power_now and energy_now for time estimate.
        let time_remaining = (|| -> Option<String> {
            let energy = std::fs::read_to_string("/sys/class/power_supply/BAT0/energy_now").ok()?;
            let power = std::fs::read_to_string("/sys/class/power_supply/BAT0/power_now").ok()?;
            let energy: f64 = energy.trim().parse().ok()?;
            let power: f64 = power.trim().parse().ok()?;
            if power <= 0.0 {
                return None;
            }
            let hours = energy / power;
            let h = hours as u64;
            let m = ((hours - h as f64) * 60.0) as u64;
            Some(format!("{h}:{m:02}"))
        })();
        Some(BatteryInfo {
            percent,
            charging,
            source,
            time_remaining,
        })
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

/// Count installed Nix profile packages (from `nix profile list`).
/// Uses a cached value to avoid calling the command on every snapshot.
fn get_nix_package_count() -> usize {
    use std::sync::OnceLock;
    static COUNT: OnceLock<usize> = OnceLock::new();
    *COUNT.get_or_init(|| {
        // Count entries in ~/.nix-profile/bin/ as a fast proxy.
        let home = std::env::var("HOME").unwrap_or_default();
        let bin_dir = format!("{home}/.nix-profile/bin");
        std::fs::read_dir(bin_dir)
            .map(|entries| entries.count())
            .unwrap_or(0)
    })
}

/// Get primary local IP address by connecting to a public DNS address.
/// Cached since the local IP rarely changes during a session.
fn get_local_ip() -> String {
    use std::sync::OnceLock;
    static IP: OnceLock<String> = OnceLock::new();
    IP.get_or_init(|| {
        std::net::UdpSocket::bind("0.0.0.0:0")
            .and_then(|sock| {
                sock.connect("8.8.8.8:80")?;
                sock.local_addr()
            })
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|_| "unknown".into())
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_wifi() {
        assert!(matches!(classify_interface("wlan0"), NetKind::Wifi));
        assert!(matches!(classify_interface("wlp2s0"), NetKind::Wifi));
        assert!(matches!(classify_interface("en0"), NetKind::Wifi));
    }

    #[test]
    fn test_classify_ethernet() {
        assert!(matches!(classify_interface("eth0"), NetKind::Ethernet));
        assert!(matches!(classify_interface("enp3s0"), NetKind::Ethernet));
        assert!(matches!(classify_interface("en1"), NetKind::Ethernet));
    }

    #[test]
    fn test_classify_virtual() {
        assert!(matches!(classify_interface("veth12345"), NetKind::Virtual));
        assert!(matches!(classify_interface("docker0"), NetKind::Virtual));
        assert!(matches!(classify_interface("br-abc123"), NetKind::Virtual));
        assert!(matches!(classify_interface("cali987"), NetKind::Virtual));
    }

    #[test]
    fn test_classify_unknown() {
        assert!(matches!(classify_interface("lo"), NetKind::Unknown));
        assert!(matches!(classify_interface("tun0"), NetKind::Unknown));
    }
}
