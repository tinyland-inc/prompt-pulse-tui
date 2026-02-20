use std::path::PathBuf;

use anyhow::Result;
use image::DynamicImage;

use crate::config::TuiConfig;

/// Load the most recent cached waifu image from the cache directory.
pub fn load_cached_waifu(cfg: &TuiConfig) -> Result<Option<DynamicImage>> {
    let waifu_dir = waifu_cache_dir(cfg);
    if !waifu_dir.exists() {
        return Ok(None);
    }

    // Find the newest image file in the waifu cache.
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(&waifu_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif") {
            continue;
        }
        let modified = entry.metadata()?.modified()?;
        if newest.as_ref().map_or(true, |(t, _)| modified > *t) {
            newest = Some((modified, path));
        }
    }

    match newest {
        Some((_, path)) => {
            let img = image::open(&path)?;
            Ok(Some(img))
        }
        None => Ok(None),
    }
}

fn waifu_cache_dir(cfg: &TuiConfig) -> PathBuf {
    cfg.cache_dir().join("waifu")
}
