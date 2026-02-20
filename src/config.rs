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
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("prompt-pulse")
            .join("config.toml")
    }

    /// Resolve the cache directory (daemon writes JSON here).
    pub fn cache_dir(&self) -> PathBuf {
        if !self.general.cache_dir.is_empty() {
            PathBuf::from(&self.general.cache_dir)
        } else {
            dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("prompt-pulse")
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
