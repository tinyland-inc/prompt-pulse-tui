use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Deserialize;

/// JSON response from the waifu mirror's /api/random endpoint.
#[derive(Debug, Deserialize)]
pub struct ImageMeta {
    pub url: String,
    pub id: String,
    pub width: i32,
    pub height: i32,
    pub hash: String,
}

/// Fetch a random image from the waifu mirror API and save to cache.
/// Returns the local path of the cached image.
pub async fn fetch_random(endpoint: &str, category: &str, cache_dir: &Path) -> Result<PathBuf> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    // Step 1: Get random image metadata.
    let url = format!("{}/api/random?category={}", endpoint, category);
    let meta: ImageMeta = client.get(&url).send().await?.json().await?;

    // Step 2: Determine filename from server-provided hash.
    let ext = Path::new(&meta.id)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("webp");
    let filename = format!("{}.{}", meta.hash, ext);
    let dest = cache_dir.join(&filename);

    // Skip download if already cached (dedup by hash).
    if dest.exists() {
        return Ok(dest);
    }

    // Step 3: Download image bytes.
    // The API returns a relative URL (/api/image/...), prepend the endpoint.
    let image_url = if meta.url.starts_with('/') {
        format!("{}{}", endpoint, meta.url)
    } else {
        meta.url.clone()
    };
    let data = client.get(&image_url).send().await?.bytes().await?;

    // Step 4: Atomic write to cache.
    std::fs::create_dir_all(cache_dir)?;
    let tmp = cache_dir.join(format!("{}.tmp", filename));
    std::fs::write(&tmp, &data)?;
    std::fs::rename(&tmp, &dest)?;

    Ok(dest)
}
