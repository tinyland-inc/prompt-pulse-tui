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

/// Result of a live fetch: raw image bytes + metadata (no disk IO).
pub struct FetchResult {
    pub data: Vec<u8>,
    pub name: String, // from ImageMeta.id
    pub hash: String, // dedup key
}

/// Fetch a random image from the waifu mirror API.
/// Returns raw image bytes + metadata. No disk writes.
pub async fn fetch_random(endpoint: &str, category: &str) -> Result<FetchResult> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .danger_accept_invalid_certs(true)
        .build()?;

    // Step 1: Get random image metadata.
    let url = format!("{}/api/random?category={}", endpoint, category);
    let meta: ImageMeta = client.get(&url).send().await?.json().await?;

    // Step 2: Download image bytes.
    // The API returns a relative URL (/api/image/...), prepend the endpoint.
    let image_url = if meta.url.starts_with('/') {
        format!("{}{}", endpoint, meta.url)
    } else {
        meta.url.clone()
    };
    let data = client.get(&image_url).send().await?.bytes().await?;

    Ok(FetchResult {
        data: data.to_vec(),
        name: meta.id,
        hash: meta.hash,
    })
}
