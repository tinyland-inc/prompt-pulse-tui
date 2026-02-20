use std::path::{Path, PathBuf};

use anyhow::Result;
use image::{DynamicImage, ImageReader};

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
        if !is_image_file(&path) {
            continue;
        }
        let modified = entry.metadata()?.modified()?;
        if newest.as_ref().map_or(true, |(t, _)| modified > *t) {
            newest = Some((modified, path));
        }
    }

    match newest {
        Some((_, path)) => {
            let img = open_by_magic(&path)?;
            Ok(Some(img))
        }
        None => Ok(None),
    }
}

/// List all image files in the waifu cache directory, sorted by filename.
pub fn list_images(cfg: &TuiConfig) -> Vec<PathBuf> {
    let waifu_dir = waifu_cache_dir(cfg);
    if !waifu_dir.exists() {
        return Vec::new();
    }
    let mut images: Vec<PathBuf> = std::fs::read_dir(&waifu_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| is_image_file(p))
        .collect();
    images.sort();
    images
}

/// Load a specific image by path.
/// Uses magic byte detection so files with non-standard extensions (e.g. `.img`) are handled.
pub fn load_image(path: &Path) -> Result<DynamicImage> {
    open_by_magic(path)
}

/// Format an image filename as a human-readable name.
/// Strips directory and extension, replaces `_` and `-` with spaces.
pub fn format_image_name(path: &Path) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    stem.replace(['_', '-'], " ")
}

/// Open an image file using magic byte detection instead of relying on extension.
/// The Go daemon saves images with `.img` extension which `image::open()` can't recognize.
fn open_by_magic(path: &Path) -> Result<DynamicImage> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    Ok(reader.decode()?)
}

fn is_image_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "img"
    )
}

fn waifu_cache_dir(cfg: &TuiConfig) -> PathBuf {
    cfg.cache_dir().join("waifu")
}
