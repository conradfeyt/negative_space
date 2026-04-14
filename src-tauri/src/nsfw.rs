// nsfw.rs — Sensitive content detection using CoreML + Vision.
//
// Scans user image directories, classifies each image through the bundled
// OpenNSFW2 CoreML model (via the Swift bridge), and returns flagged files
// above a configurable threshold. Thumbnails are generated for flagged images.
//
// The Swift bridge (`msw_classify_nsfw`) handles the actual CoreML inference.
// This module handles discovery, batching, progress events, dismissed-path
// persistence, and result assembly.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};
use walkdir::WalkDir;

static NSFW_CANCELLED: AtomicBool = AtomicBool::new(false);

fn chrono_now() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, d.subsec_millis())
}

use crate::commands;
use crate::image_utils;

/// Extract EXIF DateTimeOriginal from a JPEG/TIFF file.
/// Returns an ISO-ish string like "2024-03-15T14:30:00" or None.
fn extract_exif_date(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut buf = std::io::BufReader::new(file);
    let reader = exif::Reader::new().read_from_container(&mut buf).ok()?;

    let tags = [
        exif::Tag::DateTimeOriginal,
        exif::Tag::DateTimeDigitized,
        exif::Tag::DateTime,
    ];
    for tag in &tags {
        if let Some(field) = reader.get_field(*tag, exif::In::PRIMARY) {
            let val = field.display_value().to_string();
            if val.is_empty() || val == "unknown" {
                continue;
            }
            // display_value() returns "2024-12-26 13-14-54" (dashes throughout).
            // We need ISO 8601: "2024-12-26T13:14:54".
            // Split on space to get date and time parts, then fix the time separators.
            let parts: Vec<&str> = val.splitn(2, ' ').collect();
            if parts.len() == 2 {
                let date_part = parts[0]; // "2024-12-26" or "2024:12:26"
                let time_part = parts[1].replace('-', ":"); // "13-14-54" → "13:14:54"
                let date_fixed = date_part.replace(':', "-"); // handle "2024:12:26" → "2024-12-26"
                return Some(format!("{}T{}", date_fixed, time_part));
            }
            // Single part — just normalize colons in date portion
            return Some(val.replacen(':', "-", 2));
        }
    }
    None
}

/// Extract date for HEIC files using macOS mdls (Spotlight metadata).
fn extract_mdls_date(path: &Path) -> Option<String> {
    let output = std::process::Command::new("mdls")
        .args(["-name", "kMDItemContentCreationDate", "-raw"])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if val.is_empty() || val == "(null)" {
        return None;
    }
    // mdls returns "2024-03-15 14:30:00 +0000" → take the date+time part
    Some(val.split(" +").next().unwrap_or(&val).replace(' ', "T"))
}

/// Get the best available date for an image (EXIF > mdls > None).
fn get_image_date(path: &Path) -> Option<String> {
    if let Some(d) = extract_exif_date(path) {
        return Some(d);
    }
    extract_mdls_date(path)
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NsfwFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified: String,
    pub parent_dir: String,
    pub score: f64,
    pub thumbnail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo_asset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_taken: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_labels: Option<Vec<DetectedLabel>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DetectedLabel {
    pub label: String,
    pub confidence: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NsfwScanResult {
    pub flagged: Vec<NsfwFile>,
    pub images_scanned: u64,
    pub images_skipped: u64,
    pub threshold: f64,
    pub scan_duration_ms: u64,
    pub warnings: Vec<String>,
    pub model: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NsfwScanProgress {
    pub images_processed: u64,
    pub total_images: u64,
    pub current_file: String,
    pub phase: String,
    #[serde(default)]
    pub images_discovered: u64,
    #[serde(default)]
    pub thumbnails_processed: u64,
    #[serde(default)]
    pub total_thumbnails: u64,
}

// ---------------------------------------------------------------------------
// FFI declarations
// ---------------------------------------------------------------------------

#[cfg(has_swift_bridge)]
extern "C" {
    fn msw_classify_nsfw(json_input: *const std::os::raw::c_char) -> *mut std::os::raw::c_char;
    fn msw_detect_nsfw(json_input: *const std::os::raw::c_char) -> *mut std::os::raw::c_char;
    fn msw_photos_auth_status() -> *mut std::os::raw::c_char;
    fn msw_request_photos_access() -> *mut std::os::raw::c_char;
    fn msw_enumerate_photo_paths(json_input: *const std::os::raw::c_char) -> *mut std::os::raw::c_char;
    fn msw_delete_photo_assets(json_input: *const std::os::raw::c_char) -> *mut std::os::raw::c_char;
    fn msw_photos_thumbnail(json_input: *const std::os::raw::c_char) -> *mut std::os::raw::c_char;
    fn msw_free_string(ptr: *mut std::os::raw::c_char);
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const BATCH_SIZE: usize = 50;
const DEFAULT_THRESHOLD: f64 = 0.5;

const EXPOSED_LABELS: &[&str] = &[
    "FEMALE_BREAST_EXPOSED",
    "BUTTOCKS_EXPOSED",
    "FEMALE_GENITALIA_EXPOSED",
    "MALE_GENITALIA_EXPOSED",
    "ANUS_EXPOSED",
    "MALE_BREAST_EXPOSED",
    "BELLY_EXPOSED",
    "ARMPITS_EXPOSED",
];

// ---------------------------------------------------------------------------
// Public API (Tauri commands)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn cancel_nsfw_scan() {
    NSFW_CANCELLED.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub async fn scan_nsfw(
    app: AppHandle,
    path: Option<String>,
    threshold: Option<f64>,
    min_size_mb: Option<u64>,
    has_fda: Option<bool>,
    skip_paths: Option<Vec<String>>,
    model: Option<String>,
) -> Result<NsfwScanResult, String> {
    NSFW_CANCELLED.store(false, Ordering::SeqCst);
    let start = std::time::Instant::now();
    let threshold = threshold.unwrap_or(DEFAULT_THRESHOLD);
    let fda = has_fda.unwrap_or(false);
    let skips = skip_paths.unwrap_or_default();
    let scan_path = path.unwrap_or_else(|| "~".to_string());
    let _ = model; // model choice removed — always run both models
    let min_bytes = min_size_mb
        .map(|mb| mb * 1024 * 1024)
        .unwrap_or(10 * 1024); // 10 KB default

    let home = commands::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?;

    let skip_prefixes = commands::build_skip_prefixes(&home, &skips, &[]);

    // Resolve scan roots: if user picked a specific directory, use it directly.
    // Only fall back to safe_dirs when scanning "~" without FDA.
    let resolved_path = if scan_path.starts_with("~/") {
        format!("{}{}", home, &scan_path[1..])
    } else if scan_path == "~" {
        home.clone()
    } else {
        scan_path.clone()
    };

    let scan_roots = if scan_path != "~" && Path::new(&resolved_path).is_dir() {
        vec![resolved_path]
    } else if fda {
        vec![home.clone()]
    } else {
        vec![
            format!("{}/Pictures", home),
            format!("{}/Downloads", home),
            format!("{}/Documents", home),
            format!("{}/Desktop", home),
        ]
        .into_iter()
        .filter(|d| Path::new(d).is_dir())
        .collect()
    };

    // Load dismissed paths
    let dismissed = load_dismissed(&home);

    // Set up scan log
    let log_path = format!(
        "{}/Library/Application Support/com.conradfe.negativespace/nsfw_scan.log",
        home
    );
    let _ = std::fs::create_dir_all(std::path::Path::new(&log_path).parent().unwrap());
    let log_file = std::fs::File::create(&log_path).ok();
    macro_rules! scan_log {
        ($($arg:tt)*) => {
            if let Some(ref f) = log_file {
                use std::io::Write;
                let _ = writeln!(&*f, "[{}] {}", chrono_now(), format!($($arg)*));
            }
        };
    }

    scan_log!("=== NSFW Scan Start ===");
    scan_log!("scan_path={}, threshold={}, min_bytes={}, fda={}", scan_path, threshold, min_bytes, fda);
    scan_log!("scan_roots={:?}", scan_roots);

    // Phase 1: Discover
    let _ = app.emit("nsfw-scan-progress", NsfwScanProgress {
        images_processed: 0,
        total_images: 0,
        current_file: "Discovering images…".to_string(),
        phase: "discovery".to_string(),
        images_discovered: 0,
        thumbnails_processed: 0,
        total_thumbnails: 0,
    });

    let mut warnings: Vec<String> = Vec::new();
    let model_path = resolve_model_path(&app, "OpenNSFW2.mlmodelc");
    let nudenet_model_path = resolve_model_path(&app, "NudeNet320n.mlmodelc");
    scan_log!("OpenNSFW2 path: {} (exists: {})", model_path, Path::new(&model_path).exists());
    scan_log!("NudeNet path: {} (exists: {})", nudenet_model_path, Path::new(&nudenet_model_path).exists());
    if !Path::new(&nudenet_model_path).exists() {
        warnings.push("NudeNet model not found — label enrichment unavailable".to_string());
    }

    // Check if any scan root contains a .photoslibrary
    let has_photos_library = scan_roots.iter().any(|root| {
        root.contains(".photoslibrary")
            || std::fs::read_dir(root)
                .into_iter()
                .flatten()
                .flatten()
                .any(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .ends_with(".photoslibrary")
                })
    });

    // (path, score, size, modified, detected_labels)
    let mut all_scores: Vec<(String, f64, u64, String, Option<Vec<DetectedLabel>>)> = Vec::new();
    let mut images_skipped: u64 = 0;
    let mut icloud_skipped: u64 = 0;
    let mut total_images: u64 = 0;

    // ── Phase 1a: Enumerate Photos Library paths ──────────────────────────
    // (path, size, asset_id)
    let mut photos_paths: Vec<(String, u64, Option<String>)> = Vec::new();
    #[cfg(has_swift_bridge)]
    if has_photos_library {
        scan_log!("Photos library detected — enumerating via PhotoKit");
        let _ = app.emit("nsfw-scan-progress", NsfwScanProgress {
            images_processed: 0,
            total_images: 0,
            current_file: "Requesting Photos access…".to_string(),
            phase: "discovery".to_string(),
            images_discovered: 0,
            thumbnails_processed: 0,
            total_thumbnails: 0,
        });

        let auth_status = call_swift_photos_auth();
        scan_log!("Photos auth status: {}", auth_status);

        if auth_status != "authorized" && auth_status != "limited" {
            let new_status = call_swift_request_photos_access();
            scan_log!("Photos auth after request: {}", new_status);
            if new_status != "authorized" && new_status != "limited" {
                warnings.push(
                    "Photos access denied — go to System Settings → Privacy & Security → \
                     Photos and grant access to Negativ_."
                        .to_string(),
                );
            }
        }

        let final_status = call_swift_photos_auth();
        if final_status == "authorized" || final_status == "limited" {
            let _ = app.emit("nsfw-scan-progress", NsfwScanProgress {
                images_processed: 0,
                total_images: 0,
                current_file: "Enumerating Photos library…".to_string(),
                phase: "discovery".to_string(),
                images_discovered: 0,
                thumbnails_processed: 0,
                total_thumbnails: 0,
            });

            scan_log!("Calling msw_enumerate_photo_paths with min_size={}", min_bytes);
            let result_str = call_swift_enumerate_photos(min_bytes);
            scan_log!("PhotoKit enumerate result (first 500 chars): {}", &result_str[..result_str.len().min(500)]);

            if let Ok(result) = serde_json::from_str::<serde_json::Value>(&result_str) {
                let photos_total = result["total_assets"].as_u64().unwrap_or(0);
                let photos_cloud = result["skipped_cloud"].as_u64().unwrap_or(0);
                icloud_skipped += photos_cloud;

                if let Some(entries) = result["entries"].as_array() {
                    for entry in entries {
                        if let Some(path_str) = entry["path"].as_str() {
                            if !dismissed.contains(&path_str.to_string()) {
                                let asset_id = entry["id"].as_str().map(|s| s.to_string());
                                let file_size = std::fs::metadata(path_str)
                                    .map(|m| m.len())
                                    .unwrap_or(0);
                                photos_paths.push((path_str.to_string(), file_size, asset_id));
                            }
                        }
                    }
                }
                scan_log!("PhotoKit: total_assets={}, enumerated={}, skipped_cloud={}", photos_total, photos_paths.len(), photos_cloud);

                if let Some(err) = result["error"].as_str() {
                    if !err.is_empty() {
                        scan_log!("PhotoKit warning: {}", err);
                        warnings.push(err.to_string());
                    }
                }
            } else {
                scan_log!("Failed to parse PhotoKit enumerate result");
                warnings.push("Failed to enumerate Photos library".to_string());
            }
        }
    }

    // ── Phase 1b: Discover filesystem images ────────────────────────────
    let fs_skip_prefixes = {
        let mut sp = skip_prefixes.clone();
        for root in &scan_roots {
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten() {
                    if entry.file_name().to_string_lossy().ends_with(".photoslibrary") {
                        sp.push(entry.path().to_string_lossy().to_string());
                    }
                }
            }
        }
        sp
    };

    let fs_paths = discover_images(&app, &scan_roots, &fs_skip_prefixes, min_bytes, &dismissed);
    scan_log!("Filesystem discovery: {} images found", fs_paths.len());
    for (i, (p, s)) in fs_paths.iter().enumerate().take(20) {
        scan_log!("  [{}] {} ({} bytes)", i, p, s);
    }
    if fs_paths.len() > 20 {
        scan_log!("  ... and {} more", fs_paths.len() - 20);
    }

    // ── Merge all image paths ───────────────────────────────────────────
    let mut photo_asset_ids: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut image_paths: Vec<(String, u64)> = Vec::new();
    for (path, size, asset_id) in photos_paths {
        if let Some(id) = asset_id {
            photo_asset_ids.insert(path.clone(), id);
        }
        image_paths.push((path, size));
    }
    image_paths.extend(fs_paths);
    total_images += image_paths.len() as u64;
    scan_log!("Total images to classify: {}", total_images);

    // ── Phase 2: Classify in batches ────────────────────────────────────
    #[cfg(has_swift_bridge)]
    if !image_paths.is_empty() {
        let num_batches = (image_paths.len() + BATCH_SIZE - 1) / BATCH_SIZE;
        scan_log!("Classifying {} images in {} batches (OpenNSFW2 + NudeNet)", total_images, num_batches);

        let mut opennsfw_hits: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        let mut opennsfw_all: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        let mut nudenet_flags: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        let mut nudenet_labels: std::collections::HashMap<String, Vec<DetectedLabel>> = std::collections::HashMap::new();
        let mut batch_skipped_total: u64 = 0;
        let has_nudenet = Path::new(&nudenet_model_path).exists();

        for (batch_idx, batch) in image_paths.chunks(BATCH_SIZE).enumerate() {
            if NSFW_CANCELLED.load(Ordering::SeqCst) {
                scan_log!("Scan cancelled by user during classification");
                return Err("Scan cancelled".to_string());
            }
            let processed = (batch_idx * BATCH_SIZE) as u64;
            let _ = app.emit("nsfw-scan-progress", NsfwScanProgress {
                images_processed: processed,
                total_images,
                current_file: batch.first().map(|(p, _)| p.clone()).unwrap_or_default(),
                phase: "classifying".to_string(),
                images_discovered: total_images,
                thumbnails_processed: 0,
                total_thumbnails: 0,
            });

            let paths_json: Vec<&str> = batch.iter().map(|(p, _)| p.as_str()).collect();

            // ── OpenNSFW2: binary detection ──
            let input = serde_json::json!({
                "paths": paths_json,
                "model_path": model_path,
            });
            let results_json = call_swift_nsfw(&input.to_string());
            if let Ok(scores) = serde_json::from_str::<Vec<NsfwScoreRaw>>(&results_json) {
                let batch_skipped = batch.len() - scores.len();
                scan_log!("OpenNSFW2 batch {}/{}: {} scored, {} skipped", batch_idx + 1, num_batches, scores.len(), batch_skipped);
                for score in scores {
                    opennsfw_all.insert(score.path.clone(), score.score);
                    if score.score >= threshold {
                        scan_log!("  OPENNSFW2 FLAGGED: {} → {:.4}", score.path, score.score);
                        opennsfw_hits.insert(score.path, score.score);
                    }
                }
            } else {
                scan_log!("OpenNSFW2 batch {}/{}: FAILED to parse", batch_idx + 1, num_batches);
            }

            // ── NudeNet: detection + label enrichment for every image ──
            if has_nudenet {
                let input = serde_json::json!({
                    "paths": paths_json,
                    "model_path": nudenet_model_path,
                });
                let results_json = call_swift_nudenet(&input.to_string());
                if let Ok(detections) = serde_json::from_str::<Vec<NudeNetResultRaw>>(&results_json) {
                    let batch_skipped = batch.len() - detections.len();
                    scan_log!("NudeNet batch {}/{}: {} detected, {} skipped", batch_idx + 1, num_batches, detections.len(), batch_skipped);
                    batch_skipped_total += batch_skipped as u64;
                    for det in detections {
                        let labels: Vec<DetectedLabel> = det.detections.iter()
                            .map(|d| DetectedLabel { label: d.label.clone(), confidence: d.confidence })
                            .collect();

                        // Check if NudeNet independently flags this image
                        let exposed: Vec<&NudeNetDetection> = det.detections.iter()
                            .filter(|d| EXPOSED_LABELS.contains(&d.label.as_str()))
                            .collect();
                        if let Some(best) = exposed.iter().max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal)) {
                            if best.confidence >= threshold {
                                scan_log!("  NUDENET FLAGGED: {} → {:.4}", det.path, best.confidence);
                                nudenet_flags.insert(det.path.clone(), best.confidence);
                            }
                        }

                        // Store labels for ALL images (enrichment)
                        if !labels.is_empty() {
                            nudenet_labels.insert(det.path, labels);
                        }
                    }
                } else {
                    scan_log!("NudeNet batch {}/{}: FAILED to parse", batch_idx + 1, num_batches);
                    batch_skipped_total += batch.len() as u64;
                }
            }
        }

        images_skipped += batch_skipped_total;

        // Merge: union of flagged paths, max score, labels from NudeNet for all
        let mut flagged_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
        for p in opennsfw_hits.keys() { flagged_paths.insert(p.clone()); }
        for p in nudenet_flags.keys() { flagged_paths.insert(p.clone()); }

        scan_log!("Merge: {} OpenNSFW2 hits, {} NudeNet hits, {} with labels, {} union",
            opennsfw_hits.len(), nudenet_flags.len(), nudenet_labels.len(), flagged_paths.len());

        for path in flagged_paths {
            let opennsfw_score = opennsfw_all.get(&path).copied().unwrap_or(0.0);
            let nudenet_score = nudenet_flags.get(&path).copied().unwrap_or(0.0);
            let score = opennsfw_score.max(nudenet_score);
            let mut labels = nudenet_labels.remove(&path).unwrap_or_default();
            if opennsfw_score > 0.0 {
                labels.insert(0, DetectedLabel {
                    label: "NSFW_SCORE".to_string(),
                    confidence: opennsfw_score,
                });
            }
            let detected_labels = if labels.is_empty() { None } else { Some(labels) };

            let file_size = image_paths.iter()
                .find(|(p, _)| *p == path)
                .map(|(_, s)| *s)
                .unwrap_or(0);
            let modified = image_paths.iter()
                .find(|(p, _)| *p == path)
                .and_then(|(p, _)| std::fs::metadata(p).ok())
                .and_then(|m| m.modified().ok())
                .map(|t| commands::format_system_time(t))
                .unwrap_or_default();
            all_scores.push((path, score, file_size, modified, detected_labels));
        }
    }

    #[cfg(not(has_swift_bridge))]
    {
        let _ = &model_path;
        let _ = &nudenet_model_path;
        let _ = &image_paths;
        let _ = &model_path;
        let _ = &nudenet_model_path;
        images_skipped = total_images;
        scan_log!("Swift bridge not available — skipping classification");
    }

    scan_log!("Classification complete: {} flagged, {} skipped out of {} total", all_scores.len(), images_skipped, total_images);

    // all_scores already filtered to threshold
    let flagged_scores = all_scores;

    // Phase 3: Generate thumbnails for flagged
    let total_flagged = flagged_scores.len() as u64;
    let _ = app.emit("nsfw-scan-progress", NsfwScanProgress {
        images_processed: total_images,
        total_images,
        current_file: "Generating thumbnails…".to_string(),
        phase: "thumbnails".to_string(),
        images_discovered: total_images,
        thumbnails_processed: 0,
        total_thumbnails: total_flagged,
    });

    let mut flagged: Vec<NsfwFile> = Vec::with_capacity(flagged_scores.len());
    for (path, score, size, created, labels) in &flagged_scores {
        let is_photos = path.starts_with("photos://") || path.contains(".photoslibrary/");
        let name = if is_photos {
            path.rsplit('/').next().unwrap_or("Photo").to_string()
        } else {
            Path::new(path).file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        };

        // Get filesystem modified time
        let fs_modified = if !created.is_empty() {
            created.clone()
        } else {
            std::fs::metadata(path)
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| commands::format_system_time(t))
                .unwrap_or_default()
        };

        // Extract EXIF/mdls date for all accessible files
        let date_taken = if path.starts_with("photos://") {
            None
        } else {
            get_image_date(Path::new(path))
        };

        // Use the best date as `modified` if fs date is empty
        let modified = if fs_modified.is_empty() {
            date_taken.clone().unwrap_or_default()
        } else {
            fs_modified.clone()
        };

        let parent_dir = if is_photos {
            "Photos Library".to_string()
        } else {
            Path::new(path).parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
        };

        flagged.push(NsfwFile {
            path: path.clone(),
            name,
            size: *size,
            modified,
            parent_dir,
            score: *score,
            thumbnail: None,
            photo_asset_id: photo_asset_ids.get(path).cloned(),
            date_taken,
            detected_labels: labels.clone(),
        });
    }

    // Sort by score descending
    flagged.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Generate thumbnails
    let total_thumbs = flagged.len() as u64;
    scan_log!("Generating thumbnails for {} flagged images", total_thumbs);
    for i in 0..flagged.len() {
        let path = &flagged[i].path;
        scan_log!("  [{}] {}", i, path);

        if i % 5 == 0 {
            let _ = app.emit("nsfw-scan-progress", NsfwScanProgress {
                images_processed: total_images,
                total_images,
                current_file: flagged[i].name.clone(),
                phase: "thumbnails".to_string(),
                images_discovered: total_images,
                thumbnails_processed: i as u64,
                total_thumbnails: total_thumbs,
            });
        }
        if path.starts_with("photos://") {
            // Extract identifier from photos://IDENTIFIER/filename
            // localIdentifier looks like "UUID/L0/001", filename is the last component
            let identifier = path
                .strip_prefix("photos://")
                .map(|s| {
                    // Remove the last path component (filename)
                    s.rsplitn(2, '/').last().unwrap_or(s)
                })
                .unwrap_or("");
            scan_log!("  Thumbnail for identifier='{}' (from path='{}')", identifier, path);
            #[cfg(has_swift_bridge)]
            {
                let input = serde_json::json!({ "identifier": identifier, "size": 200 });
                let input_str = std::ffi::CString::new(input.to_string()).unwrap();
                let result_ptr = unsafe { msw_photos_thumbnail(input_str.as_ptr()) };
                let result = unsafe { std::ffi::CStr::from_ptr(result_ptr).to_string_lossy().to_string() };
                unsafe { msw_free_string(result_ptr) };
                scan_log!("  Result length: {} chars, starts_with data: {}", result.len(), result.starts_with("data:"));
                if !result.is_empty() {
                    flagged[i].thumbnail = Some(result);
                }
            }
        } else {
            if let Ok(thumb) = image_utils::generate_thumbnail(Path::new(path), 200) {
                flagged[i].thumbnail = Some(thumb);
            }
        }
    }

    let elapsed = start.elapsed().as_millis() as u64;

    scan_log!("=== Scan Complete ===");
    scan_log!("Duration: {}ms, scanned: {}, classification_skipped: {}, icloud_skipped: {}, flagged: {}, warnings: {}",
        elapsed, total_images.saturating_sub(images_skipped), images_skipped, icloud_skipped, flagged.len(), warnings.len());
    scan_log!("Log saved to: {}", log_path);

    Ok(NsfwScanResult {
        flagged,
        images_scanned: total_images.saturating_sub(images_skipped),
        images_skipped: images_skipped + icloud_skipped,
        threshold,
        scan_duration_ms: elapsed,
        warnings,
        model: "dual".to_string(),
    })
}

#[tauri::command]
pub async fn dismiss_nsfw_paths(paths: Vec<String>) -> Result<(), String> {
    let home = commands::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?;
    let mut dismissed = load_dismissed(&home);
    for p in paths {
        if !dismissed.contains(&p) {
            dismissed.push(p);
        }
    }
    save_dismissed(&home, &dismissed)
}

#[tauri::command]
pub async fn clear_nsfw_dismissed() -> Result<(), String> {
    let home = commands::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?;
    save_dismissed(&home, &[])
}

#[tauri::command]
pub async fn delete_photo_assets(asset_ids: Vec<String>) -> Result<u64, String> {
    call_swift_delete_photos(&asset_ids)
}

#[cfg(has_swift_bridge)]
fn call_swift_delete_photos(asset_ids: &[String]) -> Result<u64, String> {
    use std::ffi::{CStr, CString};
    let input = serde_json::to_string(asset_ids)
        .map_err(|e| format!("JSON error: {}", e))?;
    let c_input = CString::new(input)
        .map_err(|_| "CString error".to_string())?;
    let ptr = unsafe { msw_delete_photo_assets(c_input.as_ptr()) };
    let result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().to_string();
    unsafe { msw_free_string(ptr) };

    let parsed: serde_json::Value = serde_json::from_str(&result)
        .map_err(|e| format!("Parse error: {}", e))?;
    if let Some(err) = parsed["error"].as_str() {
        if parsed["deleted"].as_u64().unwrap_or(0) == 0 {
            return Err(err.to_string());
        }
    }
    Ok(parsed["deleted"].as_u64().unwrap_or(0))
}

#[cfg(not(has_swift_bridge))]
fn call_swift_delete_photos(_asset_ids: &[String]) -> Result<u64, String> {
    Err("Swift bridge not available".to_string())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn discover_images(
    app: &AppHandle,
    scan_roots: &[String],
    skip_prefixes: &[String],
    min_size_bytes: u64,
    dismissed: &[String],
) -> Vec<(String, u64)> {
    let mut image_paths: Vec<(String, u64)> = Vec::new();
    let mut emit_counter: u64 = 0;
    for root in scan_roots {
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let path_str = path.to_string_lossy();

            if skip_prefixes.iter().any(|sp| path_str.starts_with(sp.as_str())) {
                continue;
            }
            if path_str.contains("/.git/") || path_str.ends_with("/.git") {
                continue;
            }
            if !entry.file_type().is_file() || !image_utils::is_image_file(path) {
                continue;
            }

            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if size < min_size_bytes {
                continue;
            }

            let ps = path_str.to_string();
            if dismissed.contains(&ps) {
                continue;
            }

            image_paths.push((ps, size));

            emit_counter += 1;
            if emit_counter % 100 == 0 {
                let _ = app.emit("nsfw-scan-progress", NsfwScanProgress {
                    images_processed: 0,
                    total_images: 0,
                    current_file: format!("Found {} images…", emit_counter),
                    phase: "discovery".to_string(),
                    images_discovered: emit_counter,
                    thumbnails_processed: 0,
                    total_thumbnails: 0,
                });
            }
        }
    }
    image_paths
}

fn resolve_model_path(app: &AppHandle, model_name: &str) -> String {
    if let Ok(resource_dir) = app.path().resource_dir() {
        let model_path = resource_dir.join("resources").join(model_name);
        if model_path.exists() {
            return model_path.to_string_lossy().to_string();
        }
        let alt_path = resource_dir.join(model_name);
        if alt_path.exists() {
            return alt_path.to_string_lossy().to_string();
        }
    }
    let dev_path = std::env::current_dir()
        .unwrap_or_default()
        .join("resources")
        .join(model_name);
    dev_path.to_string_lossy().to_string()
}

#[cfg(has_swift_bridge)]
fn call_swift_nsfw(input: &str) -> String {
    use std::ffi::{CStr, CString};
    let c_input = match CString::new(input) {
        Ok(c) => c,
        Err(_) => return "[]".to_string(),
    };
    let result_ptr = unsafe { msw_classify_nsfw(c_input.as_ptr()) };
    if result_ptr.is_null() {
        return "[]".to_string();
    }
    let result = unsafe { CStr::from_ptr(result_ptr) }
        .to_string_lossy()
        .to_string();
    unsafe { msw_free_string(result_ptr) };
    result
}

#[cfg(not(has_swift_bridge))]
fn call_swift_nsfw(_input: &str) -> String {
    "[]".to_string()
}

#[cfg(has_swift_bridge)]
fn call_swift_nudenet(input: &str) -> String {
    use std::ffi::{CStr, CString};
    let c_input = match CString::new(input) {
        Ok(c) => c,
        Err(_) => return "[]".to_string(),
    };
    let result_ptr = unsafe { msw_detect_nsfw(c_input.as_ptr()) };
    if result_ptr.is_null() {
        return "[]".to_string();
    }
    let result = unsafe { CStr::from_ptr(result_ptr) }
        .to_string_lossy()
        .to_string();
    unsafe { msw_free_string(result_ptr) };
    result
}

#[cfg(not(has_swift_bridge))]
fn call_swift_nudenet(_input: &str) -> String {
    "[]".to_string()
}

#[cfg(has_swift_bridge)]
fn call_swift_photos_auth() -> String {
    use std::ffi::CStr;
    let ptr = unsafe { msw_photos_auth_status() };
    if ptr.is_null() {
        return "unknown".to_string();
    }
    let json_str = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().to_string();
    unsafe { msw_free_string(ptr) };
    serde_json::from_str::<serde_json::Value>(&json_str)
        .ok()
        .and_then(|v| v["status"].as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(not(has_swift_bridge))]
fn call_swift_photos_auth() -> String {
    "unavailable".to_string()
}

#[cfg(has_swift_bridge)]
fn call_swift_request_photos_access() -> String {
    use std::ffi::CStr;
    let ptr = unsafe { msw_request_photos_access() };
    if ptr.is_null() {
        return "denied".to_string();
    }
    let result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().to_string();
    unsafe { msw_free_string(ptr) };
    result
}

#[cfg(not(has_swift_bridge))]
fn call_swift_request_photos_access() -> String {
    "denied".to_string()
}

#[cfg(has_swift_bridge)]
fn call_swift_enumerate_photos(min_size: u64) -> String {
    use std::ffi::{CStr, CString};
    let input = serde_json::json!({ "min_size": min_size });
    let c_input = match CString::new(input.to_string()) {
        Ok(c) => c,
        Err(_) => return "{\"paths\":[],\"count\":0,\"error\":\"CString error\"}".to_string(),
    };
    let ptr = unsafe { msw_enumerate_photo_paths(c_input.as_ptr()) };
    if ptr.is_null() {
        return "{\"paths\":[],\"count\":0,\"error\":\"null pointer\"}".to_string();
    }
    let result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().to_string();
    unsafe { msw_free_string(ptr) };
    result
}

#[cfg(not(has_swift_bridge))]
fn call_swift_enumerate_photos(_min_size: u64) -> String {
    "{\"paths\":[],\"count\":0}".to_string()
}

#[derive(Deserialize)]
struct NsfwScoreRaw {
    path: String,
    score: f64,
}

#[derive(Deserialize)]
struct NudeNetDetection {
    label: String,
    confidence: f64,
}

#[derive(Deserialize)]
struct NudeNetResultRaw {
    path: String,
    detections: Vec<NudeNetDetection>,
}

// ---------------------------------------------------------------------------
// Dismissed paths persistence
// ---------------------------------------------------------------------------

fn dismissed_file_path(home: &str) -> String {
    format!(
        "{}/Library/Application Support/com.conradfe.negativespace/nsfw_dismissed.json",
        home
    )
}

fn load_dismissed(home: &str) -> Vec<String> {
    let path = dismissed_file_path(home);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_dismissed(home: &str, paths: &[String]) -> Result<(), String> {
    let file_path = dismissed_file_path(home);
    let dir = Path::new(&file_path).parent()
        .ok_or_else(|| "Invalid dismissed file path".to_string())?;
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;
    let json = serde_json::to_string_pretty(paths)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    std::fs::write(&file_path, json)
        .map_err(|e| format!("Failed to write dismissed file: {}", e))
}
