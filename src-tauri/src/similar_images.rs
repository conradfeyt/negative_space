// similar_images.rs — Perceptual hash-based similar image detection.
//
// Unlike the exact duplicate finder (duplicates.rs) which uses BLAKE3 to find
// byte-identical files, this module finds visually similar images even if they
// differ in resolution, compression, format, or minor edits.
//
// Algorithm:
//   1. Walk directories and filter to image files
//   2. Compute a 16x16 gradient (dHash) perceptual hash per image
//   3. Cluster images where Hamming distance < threshold
//   4. Return groups sorted by wasted bytes
//
// Performance: dHash is fast (~5-20ms per image including decode), and Hamming
// distance comparison is a single XOR + popcount instruction per pair.

use img_hash::{HashAlg, HasherConfig, ImageHash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

use crate::commands;
use crate::image_utils;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A single image that is part of a similarity group.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SimilarFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified: String,
    pub parent_dir: String,
    /// Perceptual hash as a base64 string (for debugging/display).
    pub hash_hex: String,
    /// Base64-encoded JPEG thumbnail (140px). Generated during scan.
    pub thumbnail: Option<String>,
}

/// A group of visually similar images.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SimilarGroup {
    /// Unique identifier for this group.
    pub id: String,
    /// Index of the "representative" image (largest file kept as original).
    pub representative_idx: usize,
    /// All images in this group (2 or more).
    pub files: Vec<SimilarFile>,
    /// Average Hamming distance between pairs in this group.
    pub avg_distance: f32,
    /// Wasted bytes: total size minus the largest file.
    pub wasted_bytes: u64,
}

/// Result of a similar image scan.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SimilarScanResult {
    pub groups: Vec<SimilarGroup>,
    pub total_similar_files: u64,
    pub total_wasted_bytes: u64,
    pub images_scanned: u64,
    pub images_skipped: u64,
    pub skipped_paths: Vec<String>,
}

/// Progress event payload.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SimilarScanProgress {
    pub images_processed: u64,
    pub total_images: u64,
    pub current_file: String,
    pub phase: String,
}

/// Options for running a similar image scan.
pub struct SimilarScanOptions<'a> {
    pub threshold: Option<u32>,
    pub min_size_bytes: u64,
    pub fda: bool,
    pub skip_paths: &'a [String],
}

/// Internal struct holding a hashed image during scanning.
struct HashedImage {
    path: String,
    hash: ImageHash,
    hash_b64: String,
    content_hash: String, // BLAKE3 partial hash (first 4KB) to identify exact duplicates
    size: u64,
    modified: String,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_GROUPS: usize = 200;
const DEFAULT_THRESHOLD: u32 = 10;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run a similar image scan.
///
/// `opts.threshold`: maximum Hamming distance to consider images "similar" (default 10).
/// `opts.min_size_bytes`: minimum file size in bytes.
/// `opts.fda`: whether Full Disk Access is available.
/// `opts.skip_paths`: user-configured paths to skip.
pub fn run_similar_scan(
    app: &AppHandle,
    opts: SimilarScanOptions<'_>,
) -> SimilarScanResult {
    let threshold = opts.threshold.unwrap_or(DEFAULT_THRESHOLD);
    let min_size_bytes = opts.min_size_bytes;
    let fda = opts.fda;

    let home = match commands::home_dir() {
        Some(h) => h,
        None => return empty_result(),
    };

    // Build skip prefixes via shared helper.
    let skip_prefixes = commands::build_skip_prefixes(&home, opts.skip_paths, &[]);

    // Safe dirs for similar-image scanner — intentionally limited to
    // user media directories (Pictures, Downloads, Documents, Desktop).
    let safe_dirs = vec![
        format!("{}/Pictures", home),
        format!("{}/Downloads", home),
        format!("{}/Documents", home),
        format!("{}/Desktop", home),
    ];
    // Note: with FDA the scanner walks from home (not a user-specified path),
    // so we pass "~" to get the home directory as the sole root.
    let scan_roots = commands::build_scan_roots(&home, "~", fda, &safe_dirs);

    // ── Phase 1: Discover image files ──────────────────────────────────────
    let _ = app.emit("similar-scan-progress", SimilarScanProgress {
        images_processed: 0,
        total_images: 0,
        current_file: "Discovering images…".to_string(),
        phase: "discovery".to_string(),
    });

    let image_paths = discover_images(&scan_roots, &skip_prefixes, min_size_bytes);
    let total_images = image_paths.len() as u64;

    // ── Phase 2: Compute perceptual hashes ─────────────────────────────────
    let (mut hashed_images, images_skipped) = hash_images(app, &image_paths, total_images);

    // ── Phase 2b: Deduplicate — keep one representative per content hash ──
    deduplicate_by_content(&mut hashed_images);

    // ── Phase 3: Cluster by Hamming distance ───────────────────────────────
    let _ = app.emit("similar-scan-progress", SimilarScanProgress {
        images_processed: total_images,
        total_images,
        current_file: "Comparing hashes…".to_string(),
        phase: "clustering".to_string(),
    });

    let groups = cluster_by_distance(&hashed_images, threshold);

    // ── Phase 4: Build result groups ───────────────────────────────────────
    let mut similar_groups = build_similar_groups(groups, &hashed_images);

    similar_groups.sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes));
    similar_groups.truncate(MAX_GROUPS);

    // ── Phase 5: Generate thumbnails ───────────────────────────────────────
    let _ = app.emit("similar-scan-progress", SimilarScanProgress {
        images_processed: total_images,
        total_images,
        current_file: "Generating thumbnails…".to_string(),
        phase: "thumbnails".to_string(),
    });

    generate_thumbnails(&mut similar_groups);

    let total_similar_files: u64 = similar_groups.iter().map(|g| g.files.len() as u64).sum();
    let total_wasted_bytes: u64 = similar_groups.iter().map(|g| g.wasted_bytes).sum();

    SimilarScanResult {
        groups: similar_groups,
        total_similar_files,
        total_wasted_bytes,
        images_scanned: hashed_images.len() as u64,
        images_skipped,
        skipped_paths: if fda {
            vec![]
        } else {
            vec!["Limited scan — enable Full Disk Access for complete results".to_string()]
        },
    }
}

// ---------------------------------------------------------------------------
// Phase helpers (private)
// ---------------------------------------------------------------------------

/// Phase 1: Walk scan roots and collect image file paths that pass filters.
fn discover_images(
    scan_roots: &[String],
    skip_prefixes: &[String],
    min_size_bytes: u64,
) -> Vec<(String, u64)> {
    let mut image_paths: Vec<(String, u64)> = Vec::new();
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

            image_paths.push((path_str.to_string(), size));
        }
    }
    image_paths
}

/// Phase 2: Compute perceptual hashes and partial content hashes for each image.
/// Returns (hashed_images, images_skipped).
fn hash_images(
    app: &AppHandle,
    image_paths: &[(String, u64)],
    total_images: u64,
) -> (Vec<HashedImage>, u64) {
    let hasher = HasherConfig::new()
        .hash_size(16, 16)
        .hash_alg(HashAlg::Gradient)
        .to_hasher();

    let mut hashed_images: Vec<HashedImage> = Vec::new();
    let mut images_skipped: u64 = 0;

    for (idx, (path_str, file_size)) in image_paths.iter().enumerate() {
        if idx % 10 == 0 {
            let _ = app.emit("similar-scan-progress", SimilarScanProgress {
                images_processed: idx as u64,
                total_images,
                current_file: path_str.clone(),
                phase: "hashing".to_string(),
            });
        }

        let path = Path::new(path_str);

        let img = match image_utils::load_image(path) {
            Ok(img) => img,
            Err(_) => {
                images_skipped += 1;
                continue;
            }
        };

        let hash = hasher.hash_image(&img);
        let hash_b64 = hash.to_base64();
        drop(img); // Free decoded image memory

        // Compute partial content hash (first 4KB) to identify exact duplicates.
        let content_hash = {
            let mut file = match std::fs::File::open(path) {
                Ok(f) => f,
                Err(_) => { images_skipped += 1; continue; }
            };
            let mut buf = [0u8; 4096];
            let bytes_read = std::io::Read::read(&mut file, &mut buf).unwrap_or(0);
            blake3::hash(&buf[..bytes_read]).to_hex().to_string()
        };

        // Get modified timestamp.
        let modified = std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| format_timestamp(d.as_secs()))
            .unwrap_or_default();

        hashed_images.push(HashedImage {
            path: path_str.clone(),
            hash,
            hash_b64,
            content_hash,
            size: *file_size,
            modified,
        });
    }

    (hashed_images, images_skipped)
}

/// Phase 2b: Remove exact duplicates (same content hash), keeping one representative.
/// Exact duplicates belong in the Exact Duplicates tab; here we only want
/// visually-similar-but-different images.
fn deduplicate_by_content(hashed_images: &mut Vec<HashedImage>) {
    let mut seen_content: HashMap<String, usize> = HashMap::new();
    let mut unique_indices: Vec<usize> = Vec::new();
    for (i, hi) in hashed_images.iter().enumerate() {
        if !seen_content.contains_key(&hi.content_hash) {
            seen_content.insert(hi.content_hash.clone(), i);
            unique_indices.push(i);
        }
    }
    let deduped: Vec<HashedImage> = unique_indices
        .into_iter()
        .map(|i| {
            let hi = &hashed_images[i];
            HashedImage {
                path: hi.path.clone(),
                hash: hi.hash.clone(),
                hash_b64: hi.hash_b64.clone(),
                content_hash: hi.content_hash.clone(),
                size: hi.size,
                modified: hi.modified.clone(),
            }
        })
        .collect();
    *hashed_images = deduped;
}

/// Phase 4: Convert clustered indices into SimilarGroup structs.
fn build_similar_groups(
    groups: Vec<Vec<usize>>,
    hashed_images: &[HashedImage],
) -> Vec<SimilarGroup> {
    let mut similar_groups: Vec<SimilarGroup> = Vec::new();

    for group_indices in groups {
        if group_indices.len() < 2 {
            continue;
        }

        let files: Vec<SimilarFile> = group_indices
            .iter()
            .map(|&i| {
                let hi = &hashed_images[i];
                let file_path = Path::new(&hi.path);
                SimilarFile {
                    path: hi.path.clone(),
                    name: file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                    size: hi.size,
                    modified: hi.modified.clone(),
                    parent_dir: file_path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
                    hash_hex: hi.hash_b64.clone(),
                    thumbnail: None, // Generated after truncation
                }
            })
            .collect();

        let representative_idx = files
            .iter()
            .enumerate()
            .max_by_key(|(_, f)| f.size)
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Average Hamming distance within the group.
        let mut total_dist: u64 = 0;
        let mut pair_count: u64 = 0;
        for i in 0..group_indices.len() {
            for j in (i + 1)..group_indices.len() {
                total_dist += hashed_images[group_indices[i]]
                    .hash
                    .dist(&hashed_images[group_indices[j]].hash) as u64;
                pair_count += 1;
            }
        }
        let avg_distance = if pair_count > 0 { total_dist as f32 / pair_count as f32 } else { 0.0 };

        let total_size: u64 = files.iter().map(|f| f.size).sum();
        let max_size = files.iter().map(|f| f.size).max().unwrap_or(0);

        similar_groups.push(SimilarGroup {
            id: uuid::Uuid::new_v4().to_string(),
            representative_idx,
            files,
            avg_distance,
            wasted_bytes: total_size.saturating_sub(max_size),
        });
    }

    similar_groups
}

/// Phase 5: Generate thumbnails for the first 10 files per group (matching frontend card cap).
/// Each file in a similar group is visually different, so each needs its own thumbnail.
fn generate_thumbnails(similar_groups: &mut [SimilarGroup]) {
    let mut thumb_jobs: Vec<(usize, usize, String)> = Vec::new(); // (group_idx, file_idx, path)
    for (gi, group) in similar_groups.iter().enumerate() {
        for (fi, file) in group.files.iter().enumerate().take(10) {
            thumb_jobs.push((gi, fi, file.path.clone()));
        }
    }

    // Generate all thumbnails concurrently on threads.
    let handles: Vec<_> = thumb_jobs.into_iter().map(|(gi, fi, path)| {
        std::thread::spawn(move || {
            let thumb = image_utils::generate_thumbnail(std::path::Path::new(&path), 140).ok();
            (gi, fi, thumb)
        })
    }).collect();

    for handle in handles {
        if let Ok((gi, fi, Some(b64))) = handle.join() {
            if gi < similar_groups.len() && fi < similar_groups[gi].files.len() {
                similar_groups[gi].files[fi].thumbnail = Some(b64);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Clustering via Union-Find
// ---------------------------------------------------------------------------

/// Cluster images by Hamming distance. Returns groups of indices where all
/// pairs within a group have distance <= threshold.
fn cluster_by_distance(images: &[HashedImage], threshold: u32) -> Vec<Vec<usize>> {
    let image_count = images.len();
    if image_count == 0 {
        return vec![];
    }

    let mut parent: Vec<usize> = (0..image_count).collect();
    let mut rank: Vec<usize> = vec![0; image_count];

    // O(n^2) pairwise comparison — fine for up to ~10k images.
    for i in 0..image_count {
        for j in (i + 1)..image_count {
            let dist = images[i].hash.dist(&images[j].hash);
            if dist <= threshold {
                union(&mut parent, &mut rank, i, j);
            }
        }
    }

    let mut groups_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..image_count {
        let root = find(&mut parent, i);
        groups_map.entry(root).or_default().push(i);
    }

    groups_map.into_values().filter(|g| g.len() >= 2).collect()
}

fn find(parent: &mut Vec<usize>, x: usize) -> usize {
    if parent[x] != x {
        parent[x] = find(parent, parent[x]);
    }
    parent[x]
}

fn union(parent: &mut Vec<usize>, rank: &mut Vec<usize>, x: usize, y: usize) {
    let rx = find(parent, x);
    let ry = find(parent, y);
    if rx == ry { return; }
    if rank[rx] < rank[ry] {
        parent[rx] = ry;
    } else if rank[rx] > rank[ry] {
        parent[ry] = rx;
    } else {
        parent[ry] = rx;
        rank[rx] += 1;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn empty_result() -> SimilarScanResult {
    SimilarScanResult {
        groups: vec![],
        total_similar_files: 0,
        total_wasted_bytes: 0,
        images_scanned: 0,
        images_skipped: 0,
        skipped_paths: vec![],
    }
}

/// Format a Unix timestamp as "YYYY-MM-DD HH:MM:SS".
fn format_timestamp(secs: u64) -> String {
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    let mut year = 1970u64;
    let mut remaining_days = days_since_epoch;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year { break; }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u64;
    for (i, &dim) in days_in_months.iter().enumerate() {
        if remaining_days < dim {
            month = i as u64 + 1;
            break;
        }
        remaining_days -= dim;
    }
    let day = remaining_days + 1;

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hours, minutes, seconds)
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
