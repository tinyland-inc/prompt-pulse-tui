use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;

/// TUI-specific configuration, loaded from the same config.toml as the Go daemon.
#[derive(Debug, Deserialize)]
pub struct TuiConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub collectors: CollectorsConfig,
    #[serde(default)]
    pub image: ImageConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct GeneralConfig {
    #[serde(default)]
    pub cache_dir: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct CollectorsConfig {
    #[serde(default)]
    pub sysmetrics: CollectorToggle,
    #[serde(default)]
    pub tailscale: CollectorToggle,
    #[serde(default)]
    pub kubernetes: CollectorToggle,
    #[serde(default)]
    pub claude: CollectorToggle,
    #[serde(default)]
    pub billing: CollectorToggle,
    #[serde(default)]
    pub waifu: WaifuCollectorConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct CollectorToggle {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct ImageConfig {
    #[serde(default)]
    pub protocol: String,
    #[serde(default)]
    pub waifu_enabled: bool,
    #[serde(default)]
    pub waifu_category: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct WaifuCollectorConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub category: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct ThemeConfig {
    #[serde(default)]
    pub name: String,
}

fn default_true() -> bool {
    true
}

impl TuiConfig {
    /// Load config from the standard path (~/.config/prompt-pulse/config.toml).
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let contents = std::fs::read_to_string(&path)?;
            let cfg: TuiConfig = toml::from_str(&contents)?;
            Ok(cfg)
        } else {
            Ok(Self::default())
        }
    }

    pub fn config_path() -> PathBuf {
        // Respect XDG_CONFIG_HOME (used by Go daemon and home-manager).
        // On macOS, dirs::config_dir() returns ~/Library/Application Support/
        // but the Go daemon writes to ~/.config/ (XDG convention).
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .ok()
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".config"))
            })
            .or_else(|| dirs::config_dir())
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("prompt-pulse").join("config.toml")
    }

    /// Resolve the cache directory (daemon writes JSON here).
    pub fn cache_dir(&self) -> PathBuf {
        if !self.general.cache_dir.is_empty() {
            PathBuf::from(&self.general.cache_dir)
        } else {
            // Respect XDG_CACHE_HOME (used by Go daemon and home-manager).
            let base = std::env::var("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .ok()
                .or_else(|| {
                    std::env::var("HOME")
                        .ok()
                        .map(|h| PathBuf::from(h).join(".cache"))
                })
                .or_else(|| dirs::cache_dir())
                .unwrap_or_else(|| PathBuf::from("/tmp"));
            base.join("prompt-pulse")
        }
    }

    /// Get the waifu mirror endpoint URL (from collectors.waifu.endpoint).
    pub fn waifu_endpoint(&self) -> Option<&str> {
        let ep = &self.collectors.waifu.endpoint;
        if ep.is_empty() {
            None
        } else {
            Some(ep.as_str())
        }
    }

    /// Get the waifu category (from collectors.waifu.category, fallback to image.waifu_category).
    pub fn waifu_category(&self) -> &str {
        let cat = &self.collectors.waifu.category;
        if !cat.is_empty() {
            cat
        } else if !self.image.waifu_category.is_empty() {
            &self.image.waifu_category
        } else {
            "sfw"
        }
    }
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            collectors: CollectorsConfig::default(),
            image: ImageConfig::default(),
            theme: ThemeConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = TuiConfig::default();
        assert!(cfg.general.cache_dir.is_empty());
        assert!(!cfg.image.waifu_enabled);
        assert!(cfg.collectors.waifu.endpoint.is_empty());
    }

    #[test]
    fn test_waifu_endpoint_empty_returns_none() {
        let cfg = TuiConfig::default();
        assert!(cfg.waifu_endpoint().is_none());
    }

    #[test]
    fn test_waifu_endpoint_set() {
        let mut cfg = TuiConfig::default();
        cfg.collectors.waifu.endpoint = "https://waifu.example.com".into();
        assert_eq!(cfg.waifu_endpoint(), Some("https://waifu.example.com"));
    }

    #[test]
    fn test_waifu_category_default_fallback() {
        let cfg = TuiConfig::default();
        assert_eq!(cfg.waifu_category(), "sfw");
    }

    #[test]
    fn test_waifu_category_collector_priority() {
        let mut cfg = TuiConfig::default();
        cfg.collectors.waifu.category = "nsfw".into();
        cfg.image.waifu_category = "sfw".into();
        assert_eq!(cfg.waifu_category(), "nsfw");
    }

    #[test]
    fn test_waifu_category_image_fallback() {
        let mut cfg = TuiConfig::default();
        cfg.image.waifu_category = "waifu".into();
        assert_eq!(cfg.waifu_category(), "waifu");
    }

    #[test]
    fn test_toml_parse_minimal() {
        let cfg: TuiConfig = toml::from_str("").unwrap();
        assert!(cfg.waifu_endpoint().is_none());
    }

    #[test]
    fn test_toml_parse_full() {
        let toml_str = r#"
[general]
cache_dir = "/tmp/test"
[collectors.waifu]
endpoint = "https://waifu.example.com"
category = "sfw"
[image]
waifu_enabled = true
"#;
        let cfg: TuiConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.cache_dir(), std::path::PathBuf::from("/tmp/test"));
        assert_eq!(cfg.waifu_endpoint(), Some("https://waifu.example.com"));
        assert!(cfg.image.waifu_enabled);
    }

    #[test]
    fn test_toml_parse_real_daemon_config() {
        let toml_str = r#"
[general]
daemon_poll_interval = "30s"
cache_dir = "/Users/jsullivan2/.cache/prompt-pulse"

[collectors.sysmetrics]
enabled = true

[collectors.tailscale]
enabled = true

[collectors.kubernetes]
enabled = true

[collectors.claude]
enabled = true

[collectors.billing]
enabled = true
interval = "15m"

[collectors.billing.civo]
enabled = true
region = "nyc1"

[collectors.billing.digitalocean]
enabled = true

[collectors.waifu]
enabled = true
endpoint = "https://waifu.ephemera.tinyland.dev"
category = "nsfw"
interval = "30m"
max_images = 20

[image]
waifu_enabled = true
protocol = "auto"

[theme]
name = "default"

[shell]
tui_keybinding = "ctrl-p"
show_banner_on_startup = true
instant_banner = true
"#;
        let cfg: TuiConfig = toml::from_str(toml_str).unwrap();
        assert!(cfg.image.waifu_enabled);
        assert_eq!(
            cfg.waifu_endpoint(),
            Some("https://waifu.ephemera.tinyland.dev")
        );
        assert_eq!(cfg.waifu_category(), "nsfw");
    }

    /// Diagnostic test: load the REAL config from disk and verify waifu init path.
    /// This catches config parsing issues that unit tests with hardcoded TOML miss.
    #[test]
    fn test_real_config_waifu_diagnostic() {
        let cfg = TuiConfig::load().unwrap();
        eprintln!("  image.waifu_enabled: {}", cfg.image.waifu_enabled);
        eprintln!(
            "  collectors.waifu.enabled: {}",
            cfg.collectors.waifu.enabled
        );
        eprintln!(
            "  collectors.waifu.endpoint: {:?}",
            cfg.collectors.waifu.endpoint
        );
        eprintln!("  cache_dir: {:?}", cfg.cache_dir());
        eprintln!("  waifu_endpoint(): {:?}", cfg.waifu_endpoint());
        eprintln!("  waifu_category(): {:?}", cfg.waifu_category());

        // In live-only mode, waifu requires an endpoint (no disk cache).
        if cfg.image.waifu_enabled {
            assert!(
                cfg.waifu_endpoint().is_some(),
                "waifu is enabled but no endpoint configured"
            );
        }
    }
}
