use anyhow::Result;
use image::DynamicImage;

/// An in-memory waifu image entry (no disk cache).
#[derive(Clone)]
pub struct WaifuEntry {
    pub image: DynamicImage,
    pub name: String, // human-readable name from ImageMeta.id
    pub hash: String, // dedup key
}

/// Decode image bytes into a DynamicImage using magic byte detection.
pub fn decode_image_bytes(data: &[u8]) -> Result<DynamicImage> {
    Ok(image::load_from_memory(data)?)
}

/// Format an image name as a human-readable string.
/// Strips extension, replaces `_` and `-` with spaces.
pub fn format_image_name(name: &str) -> String {
    // Strip extension if present.
    let stem = match name.rfind('.') {
        Some(idx) if idx > 0 => &name[..idx],
        _ => name,
    };
    stem.replace(['_', '-'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_image_name_with_ext() {
        assert_eq!(format_image_name("banner-cool_cat.img"), "banner cool cat");
    }

    #[test]
    fn test_format_image_name_no_ext() {
        assert_eq!(format_image_name("no-ext"), "no ext");
    }

    #[test]
    fn test_format_image_name_empty() {
        assert_eq!(format_image_name(""), "");
    }

    #[test]
    fn test_format_image_name_with_path_like() {
        assert_eq!(format_image_name("my_image.png"), "my image");
    }

    #[test]
    fn test_decode_image_bytes_valid_png() {
        // Create a real 1x1 PNG in memory.
        let img = image::RgbImage::new(1, 1);
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let result = decode_image_bytes(buf.get_ref());
        assert!(result.is_ok(), "should decode valid PNG bytes");
    }

    #[test]
    fn test_decode_image_bytes_invalid() {
        let result = decode_image_bytes(b"not an image");
        assert!(result.is_err(), "should fail on invalid bytes");
    }
}
