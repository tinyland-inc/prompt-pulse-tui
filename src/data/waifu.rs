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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_format_image_name_with_ext() {
        assert_eq!(
            format_image_name(Path::new("banner-cool_cat.img")),
            "banner cool cat"
        );
    }

    #[test]
    fn test_format_image_name_path() {
        assert_eq!(
            format_image_name(Path::new("/path/to/my_image.png")),
            "my image"
        );
    }

    #[test]
    fn test_format_image_name_no_ext() {
        assert_eq!(format_image_name(Path::new("no-ext")), "no ext");
    }

    #[test]
    fn test_format_image_name_empty() {
        assert_eq!(format_image_name(Path::new("")), "");
    }

    #[test]
    fn test_is_image_file_extensions() {
        let tmp = tempfile::TempDir::new().unwrap();
        for ext in &["png", "jpg", "jpeg", "webp", "gif", "img"] {
            let path = tmp.path().join(format!("test.{ext}"));
            std::fs::write(&path, b"fake").unwrap();
            assert!(is_image_file(&path), "should accept .{ext}");
        }
    }

    #[test]
    fn test_is_image_file_rejects_non_images() {
        let tmp = tempfile::TempDir::new().unwrap();
        for ext in &["txt", "json", "toml", "rs"] {
            let path = tmp.path().join(format!("test.{ext}"));
            std::fs::write(&path, b"fake").unwrap();
            assert!(!is_image_file(&path), "should reject .{ext}");
        }
    }

    #[test]
    fn test_list_images_sorted() {
        let cache_dir = tempfile::TempDir::new().unwrap();
        let waifu_dir = cache_dir.path().join("waifu");
        std::fs::create_dir(&waifu_dir).unwrap();
        for name in &["c.png", "a.png", "b.png", "readme.txt"] {
            std::fs::write(waifu_dir.join(name), b"fake").unwrap();
        }
        let mut cfg = crate::config::TuiConfig::default();
        cfg.general.cache_dir = cache_dir.path().to_string_lossy().into_owned();
        let images = list_images(&cfg);
        let names: Vec<&str> = images
            .iter()
            .filter_map(|p| p.file_name().and_then(|f| f.to_str()))
            .collect();
        assert_eq!(names, vec!["a.png", "b.png", "c.png"]);
    }

    #[test]
    fn test_list_images_empty_dir() {
        let cache_dir = tempfile::TempDir::new().unwrap();
        let waifu_dir = cache_dir.path().join("waifu");
        std::fs::create_dir(&waifu_dir).unwrap();
        let mut cfg = crate::config::TuiConfig::default();
        cfg.general.cache_dir = cache_dir.path().to_string_lossy().into_owned();
        let images = list_images(&cfg);
        assert!(images.is_empty());
    }

    #[test]
    fn test_load_image_magic_bytes() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Create a real 1x1 PNG with .img extension
        let img = image::RgbImage::new(1, 1);
        let path = tmp.path().join("test.img");
        img.save_with_format(&path, image::ImageFormat::Png)
            .unwrap();
        let loaded = load_image(&path);
        assert!(
            loaded.is_ok(),
            "should detect PNG magic bytes despite .img extension"
        );
    }
}
