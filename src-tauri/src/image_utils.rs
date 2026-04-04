// image_utils.rs — Shared image loading utilities for similar image detection
// and content classification.
//
// Handles decoding common image formats via the `image` crate, with a macOS-
// specific fallback for HEIC/HEIF files using the `sips` command-line tool
// (ships with every macOS since 10.5).

use image::DynamicImage;
use std::path::Path;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Image file extensions we can process.
/// Standard formats via the `image` crate, plus HEIC/HEIF via sips fallback.
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "tiff", "tif", "bmp", "webp",
    "heic", "heif",
];

/// Extensions that require sips conversion (not supported by the `image` crate).
const SIPS_EXTENSIONS: &[&str] = &["heic", "heif"];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check whether a file path has a recognised image extension.
pub fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Load an image from disk, returning a decoded `DynamicImage`.
///
/// For HEIC/HEIF files, this shells out to `sips` to convert to a temporary
/// PNG first, then loads the PNG. The temp file is deleted immediately after
/// loading into memory.
pub fn load_image(path: &Path) -> Result<DynamicImage, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if SIPS_EXTENSIONS.contains(&ext.as_str()) {
        load_via_sips(path)
    } else {
        image::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))
    }
}

/// Generate a small thumbnail as a base64-encoded JPEG using macOS `sips`.
///
/// `sips` uses CoreGraphics under the hood — fast, native, handles all macOS
/// image formats including HEIC. Much lighter than qlmanage (no Quick Look daemon).
/// Returns base64 JPEG string or error.
pub fn generate_thumbnail(path: &Path, max_dim: u32) -> Result<String, String> {
    let tmp_path = std::env::temp_dir()
        .join(format!("negativ_thumb_{}.jpg", uuid::Uuid::new_v4()));

    let status = std::process::Command::new("sips")
        .args([
            "--resampleHeightWidthMax", &max_dim.to_string(),
            "-s", "format", "jpeg",
            "-s", "formatOptions", "60", // quality 60 for small thumbs
            &path.to_string_lossy(),
            "--out", &tmp_path.to_string_lossy(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("sips failed: {}", e))?;

    if !status.success() {
        let _ = std::fs::remove_file(&tmp_path);
        return Err("sips thumbnail failed".to_string());
    }

    let data = std::fs::read(&tmp_path).map_err(|e| format!("Read thumb: {}", e))?;
    let _ = std::fs::remove_file(&tmp_path);

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(data))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Convert a HEIC/HEIF file to PNG via macOS `sips`, load the result, then
/// clean up the temporary file.
fn load_via_sips(path: &Path) -> Result<DynamicImage, String> {
    let tmp_dir = std::env::temp_dir().join("negativ_img_convert");
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let tmp_path = tmp_dir.join(format!("{}.png", uuid::Uuid::new_v4()));

    let status = std::process::Command::new("sips")
        .args([
            "-s", "format", "png",
            &path.to_string_lossy(),
            "--out",
            &tmp_path.to_string_lossy(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("Failed to run sips: {}", e))?;

    if !status.success() {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("sips conversion failed for {}", path.display()));
    }

    let result = image::open(&tmp_path)
        .map_err(|e| format!("Failed to open converted PNG: {}", e));

    // Always clean up temp file
    let _ = std::fs::remove_file(&tmp_path);

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_image_file() {
        assert!(is_image_file(Path::new("photo.jpg")));
        assert!(is_image_file(Path::new("photo.JPEG")));
        assert!(is_image_file(Path::new("photo.png")));
        assert!(is_image_file(Path::new("photo.heic")));
        assert!(is_image_file(Path::new("photo.webp")));
        assert!(!is_image_file(Path::new("document.pdf")));
        assert!(!is_image_file(Path::new("video.mp4")));
        assert!(!is_image_file(Path::new("noext")));
    }

    #[test]
    fn test_load_nonexistent() {
        let result = load_image(Path::new("/tmp/does_not_exist_12345.png"));
        assert!(result.is_err());
    }
}
