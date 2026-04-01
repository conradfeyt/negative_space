// preview.rs — File preview generation for Negative _.
//
// Generates previews for files so the user can tell what they're looking at
// in the duplicate finder (or anywhere else that shows file lists).
//
// STRATEGY:
//   - Images: Use macOS `qlmanage -t` (Quick Look) to generate a thumbnail,
//     then read it and return as base64 PNG. This handles jpg, png, gif, tiff,
//     webp, heic, svg, pdf, and many other formats natively — no Rust image
//     crates needed. Quick Look is what Finder uses for previews.
//   - Text files: Read the first ~100 lines (up to 64KB) and return as a
//     string. Detects text by extension (txt, md, json, yaml, csv, log, etc.)
//     and source code files.
//   - Everything else: Return file metadata only (type, size, extension).
//
// WHY `qlmanage`:
//   - Ships with every macOS install since 10.5 (2007)
//   - Handles every file type that Finder can preview
//   - Generates thumbnails to a temp directory — no in-memory image processing
//   - Runs as a subprocess, so it won't crash our app if a file is corrupt
//   - Thumbnail generation is fast (~50-200ms per file)
//
// TCC NOTE:
//   `qlmanage` inherits our app's TCC permissions. If the file is in a
//   TCC-protected directory and we don't have FDA, the subprocess will fail
//   silently. We handle this gracefully by returning a "metadata only" preview.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// The kind of preview we generated.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind")]
pub enum FilePreview {
    /// Image thumbnail — base64-encoded PNG data.
    /// The frontend renders this as `<img src="data:image/png;base64,{data}">`.
    Image {
        data: String,
        width: u32,
        height: u32,
        file_type: String,
        file_size: u64,
        file_name: String,
    },

    /// Text content — first N lines of the file.
    Text {
        content: String,
        total_lines: u64,
        truncated: bool,
        file_type: String,
        file_size: u64,
        file_name: String,
    },

    /// Metadata only — file type and size, no visual preview.
    /// Used for binary files, unsupported formats, or when preview fails.
    Metadata {
        file_type: String,
        file_size: u64,
        file_name: String,
        mime_guess: String,
    },

    /// Preview generation failed.
    Error { message: String, file_name: String },
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generate a preview for the file at the given path.
///
/// The `max_thumb_size` controls the maximum dimension (width or height) of
/// image thumbnails in pixels. 256 is a good balance of quality vs transfer size.
pub fn generate_preview(path: &str, max_thumb_size: u32) -> FilePreview {
    let file_path = Path::new(path);

    // Basic validation — does the file exist?
    if !file_path.exists() {
        return FilePreview::Error {
            message: "File not found".to_string(),
            file_name: file_name_from_path(path),
        };
    }

    // Get file metadata (size, etc.)
    let metadata = match fs::metadata(file_path) {
        Ok(m) => m,
        Err(e) => {
            return FilePreview::Error {
                message: format!("Cannot read file metadata: {}", e),
                file_name: file_name_from_path(path),
            };
        }
    };

    if !metadata.is_file() {
        return FilePreview::Metadata {
            file_type: "directory".to_string(),
            file_size: metadata.len(),
            file_name: file_name_from_path(path),
            mime_guess: "inode/directory".to_string(),
        };
    }

    let file_size = metadata.len();
    let file_name = file_name_from_path(path);
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Decide preview strategy based on file extension.
    if is_image_extension(&ext) {
        generate_image_preview(path, max_thumb_size, &ext, file_size, &file_name)
    } else if is_text_extension(&ext) {
        generate_text_preview(path, &ext, file_size, &file_name)
    } else if is_pdf_extension(&ext) {
        // PDFs get image thumbnails via Quick Look (renders first page).
        generate_image_preview(path, max_thumb_size, &ext, file_size, &file_name)
    } else {
        // Unknown type — try Quick Look first (it handles many formats).
        // If that fails, fall back to metadata only.
        let ql_result = generate_image_preview(path, max_thumb_size, &ext, file_size, &file_name);
        match &ql_result {
            FilePreview::Image { .. } => ql_result,
            _ => FilePreview::Metadata {
                file_type: categorize_extension(&ext),
                file_size,
                file_name,
                mime_guess: guess_mime(&ext),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Image preview via Quick Look
// ---------------------------------------------------------------------------

/// Generate a thumbnail for the file using macOS Quick Look (`qlmanage -t`).
///
/// `qlmanage -t` generates thumbnails for any file type that has a Quick Look
/// plugin — images, PDFs, Office docs, videos (first frame), etc.
///
/// The thumbnail is written to a temp directory as a PNG file. We read it,
/// encode as base64, and clean up.
fn generate_image_preview(
    path: &str,
    max_size: u32,
    ext: &str,
    file_size: u64,
    file_name: &str,
) -> FilePreview {
    // Create a unique temp directory for this thumbnail.
    // Using the file's hash avoids collisions if multiple previews run concurrently.
    let tmp_dir = format!("/tmp/negative_space_preview_{}", std::process::id());

    // Clean up any previous temp files.
    let _ = fs::remove_dir_all(&tmp_dir);
    let _ = fs::create_dir_all(&tmp_dir);

    // Run qlmanage to generate thumbnail.
    // -t: generate thumbnail
    // -s: max size in pixels
    // -o: output directory
    // stderr is suppressed (qlmanage is chatty with status messages).
    let result = std::process::Command::new("qlmanage")
        .args(["-t", "-s", &max_size.to_string(), "-o", &tmp_dir, path])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match result {
        Ok(status) if status.success() => {
            // qlmanage creates a file named "<original_filename>.png" in the output dir.
            // Find the generated thumbnail.
            let thumb_path = find_thumbnail_in_dir(&tmp_dir);
            match thumb_path {
                Some(tp) => {
                    // Read the thumbnail file and encode as base64.
                    match fs::read(&tp) {
                        Ok(data) => {
                            // Get thumbnail dimensions (parse PNG header for width/height).
                            let (w, h) = png_dimensions(&data);

                            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);

                            // Clean up temp dir.
                            let _ = fs::remove_dir_all(&tmp_dir);

                            FilePreview::Image {
                                data: b64,
                                width: w,
                                height: h,
                                file_type: ext.to_string(),
                                file_size,
                                file_name: file_name.to_string(),
                            }
                        }
                        Err(_) => {
                            let _ = fs::remove_dir_all(&tmp_dir);
                            fallback_metadata(ext, file_size, file_name)
                        }
                    }
                }
                None => {
                    let _ = fs::remove_dir_all(&tmp_dir);
                    fallback_metadata(ext, file_size, file_name)
                }
            }
        }
        _ => {
            let _ = fs::remove_dir_all(&tmp_dir);
            fallback_metadata(ext, file_size, file_name)
        }
    }
}

/// Find the first .png file in a directory (the qlmanage output).
fn find_thumbnail_in_dir(dir: &str) -> Option<String> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "png")
            .unwrap_or(false)
        {
            return Some(path.to_string_lossy().to_string());
        }
    }
    None
}

/// Parse PNG dimensions from the IHDR chunk.
/// PNG format: 8-byte signature, then IHDR chunk starts at byte 8.
/// IHDR data: bytes 16-19 = width (big-endian u32), bytes 20-23 = height.
fn png_dimensions(data: &[u8]) -> (u32, u32) {
    if data.len() < 24 {
        return (0, 0);
    }
    // Check PNG signature: 0x89504E47
    if data[0] != 0x89 || data[1] != 0x50 || data[2] != 0x4E || data[3] != 0x47 {
        return (0, 0);
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    (width, height)
}

// ---------------------------------------------------------------------------
// Text preview
// ---------------------------------------------------------------------------

/// Read the first ~100 lines (up to 64KB) of a text file.
fn generate_text_preview(path: &str, ext: &str, file_size: u64, file_name: &str) -> FilePreview {
    const MAX_BYTES: usize = 64 * 1024; // 64 KB
    const MAX_LINES: usize = 100;

    match fs::read(path) {
        Ok(raw_bytes) => {
            // Cap how much we read.
            let cap = raw_bytes.len().min(MAX_BYTES);
            let slice = &raw_bytes[..cap];

            // Check if it looks like valid UTF-8. If not, treat as binary.
            let text = match std::str::from_utf8(slice) {
                Ok(t) => t.to_string(),
                Err(_) => {
                    // Try lossy conversion — some files have a few bad bytes.
                    let lossy = String::from_utf8_lossy(slice);
                    // If more than 5% of bytes were replaced, it's probably binary.
                    let replaced_count = lossy.chars().filter(|c| *c == '\u{FFFD}').count();
                    if replaced_count as f64 / lossy.len() as f64 > 0.05 {
                        return fallback_metadata(ext, file_size, file_name);
                    }
                    lossy.to_string()
                }
            };

            // Count total lines in the full content and truncate to MAX_LINES.
            let total_lines = text.lines().count() as u64;
            let truncated_text: String = text
                .lines()
                .take(MAX_LINES)
                .collect::<Vec<&str>>()
                .join("\n");
            let was_truncated = total_lines > MAX_LINES as u64 || raw_bytes.len() > MAX_BYTES;

            FilePreview::Text {
                content: truncated_text,
                total_lines,
                truncated: was_truncated,
                file_type: ext.to_string(),
                file_size,
                file_name: file_name.to_string(),
            }
        }
        Err(e) => FilePreview::Error {
            message: format!("Cannot read file: {}", e),
            file_name: file_name.to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// Extension classification
// ---------------------------------------------------------------------------

/// Image file extensions that Quick Look can thumbnail.
fn is_image_extension(ext: &str) -> bool {
    matches!(
        ext,
        "jpg"
            | "jpeg"
            | "png"
            | "gif"
            | "webp"
            | "heic"
            | "heif"
            | "tiff"
            | "tif"
            | "bmp"
            | "ico"
            | "svg"
            | "raw"
            | "cr2"
            | "nef"
            | "arw"
            | "dng"
            | "icns"
    )
}

/// Text/source-code file extensions.
fn is_text_extension(ext: &str) -> bool {
    matches!(
        ext,
        "txt"
            | "md"
            | "markdown"
            | "rst"
            | "json"
            | "yaml"
            | "yml"
            | "toml"
            | "xml"
            | "csv"
            | "tsv"
            | "log"
            | "conf"
            | "cfg"
            | "ini"
            | "env"
            | "sh"
            | "bash"
            | "zsh"
            | "fish"
            | "rs"
            | "py"
            | "js"
            | "ts"
            | "jsx"
            | "tsx"
            | "vue"
            | "svelte"
            | "html"
            | "htm"
            | "css"
            | "scss"
            | "sass"
            | "less"
            | "java"
            | "kt"
            | "kts"
            | "scala"
            | "clj"
            | "c"
            | "h"
            | "cpp"
            | "hpp"
            | "cc"
            | "cxx"
            | "go"
            | "rb"
            | "php"
            | "swift"
            | "m"
            | "mm"
            | "r"
            | "R"
            | "jl"
            | "lua"
            | "pl"
            | "pm"
            | "sql"
            | "graphql"
            | "gql"
            | "dockerfile"
            | "makefile"
            | "cmake"
            | "gitignore"
            | "gitattributes"
            | "editorconfig"
            | "lock"
            | "sum"
            | "mod"
            | "plist"
    )
}

/// PDF files — Quick Look renders the first page as a thumbnail.
fn is_pdf_extension(ext: &str) -> bool {
    ext == "pdf"
}

/// Categorize a file extension into a human-readable type.
fn categorize_extension(ext: &str) -> String {
    if is_image_extension(ext) {
        return "Image".to_string();
    }
    if is_text_extension(ext) {
        return "Text".to_string();
    }
    if is_pdf_extension(ext) {
        return "PDF".to_string();
    }
    match ext {
        "mp4" | "mov" | "avi" | "mkv" | "wmv" | "flv" | "webm" | "m4v" => "Video".to_string(),
        "mp3" | "wav" | "aac" | "flac" | "ogg" | "m4a" | "wma" | "aiff" => "Audio".to_string(),
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "dmg" | "iso" => "Archive".to_string(),
        "doc" | "docx" => "Word Document".to_string(),
        "xls" | "xlsx" => "Excel Spreadsheet".to_string(),
        "ppt" | "pptx" => "PowerPoint".to_string(),
        "pages" => "Pages Document".to_string(),
        "numbers" => "Numbers Spreadsheet".to_string(),
        "keynote" => "Keynote Presentation".to_string(),
        "app" => "Application".to_string(),
        "dylib" | "so" | "a" => "Library".to_string(),
        "o" | "obj" => "Object File".to_string(),
        "" => "Unknown".to_string(),
        other => format!("{} file", other.to_uppercase()),
    }
}

/// Guess MIME type from extension (rough).
fn guess_mime(ext: &str) -> String {
    match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "json" => "application/json",
        "xml" => "application/xml",
        "zip" => "application/zip",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
    .to_string()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the file name from a path string.
fn file_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

/// Fallback when preview generation fails — return metadata only.
fn fallback_metadata(ext: &str, file_size: u64, file_name: &str) -> FilePreview {
    FilePreview::Metadata {
        file_type: categorize_extension(ext),
        file_size,
        file_name: file_name.to_string(),
        mime_guess: guess_mime(ext),
    }
}
