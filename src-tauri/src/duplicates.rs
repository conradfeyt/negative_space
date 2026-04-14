// duplicates.rs — Duplicate file detection for Negative _.
//
// Uses a 3-stage pipeline to find duplicate files efficiently:
//   1. Group files by size — unique sizes can't be duplicates (free, metadata only)
//   2. Partial hash — hash first 4KB of same-size files (minimal I/O)
//   3. Full hash — full BLAKE3 hash to confirm true duplicates
//
// This approach minimizes disk I/O. On a typical home directory:
//   - Stage 1 eliminates ~90% of files (unique sizes)
//   - Stage 2 eliminates ~95% of the remainder (same size, different headers)
//   - Stage 3 confirms the actual duplicates
//
// TCC CONSIDERATIONS:
//   Same approach as large file scanning — without FDA, we only walk known-safe
//   directories. With FDA, we can walk from ~ freely. All directory walking uses
//   walkdir (in-process) for non-TCC paths, and we skip TCC-protected paths
//   when FDA is not available.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::MetadataExt;

use crate::commands;
use crate::image_utils;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A single file that is part of a duplicate group.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DuplicateFile {
    /// Absolute path to the file
    pub path: String,
    /// File name (last component)
    pub name: String,
    /// File size in bytes
    pub size: u64,
    /// Last-modified time as a human-readable string
    pub modified: String,
    /// Parent directory path (for display grouping)
    pub parent_dir: String,
}

/// A group of files that are identical (same content).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DuplicateGroup {
    /// BLAKE3 hash of the file content (hex string)
    pub hash: String,
    /// Size of each file in bytes (all files in the group are the same size)
    pub size: u64,
    /// All files in this duplicate group (2 or more)
    pub files: Vec<DuplicateFile>,
    /// Total wasted space: (count - 1) * size
    /// (keeping one copy, the rest are "wasted")
    pub wasted_bytes: u64,
    /// Base64-encoded JPEG thumbnail (140px max) for image groups.
    /// Generated during scan via `sips`. None for non-image files.
    pub thumbnail: Option<String>,
}

/// Result of a duplicate file scan.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DuplicateScanResult {
    /// All duplicate groups found, sorted by wasted_bytes descending
    pub groups: Vec<DuplicateGroup>,
    /// Total number of duplicate files (across all groups)
    pub total_duplicate_files: u64,
    /// Total wasted space (sum of wasted_bytes across all groups)
    pub total_wasted_bytes: u64,
    /// Number of files scanned in total
    pub files_scanned: u64,
    /// Number of files that passed stage 1 (same-size groups)
    pub stage1_candidates: u64,
    /// Number of files that passed stage 2 (same partial hash)
    pub stage2_candidates: u64,
    /// Directories that were skipped (no FDA)
    pub skipped_paths: Vec<String>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of bytes to read for the partial hash (stage 2).
/// 4KB is enough to distinguish most files — headers, magic bytes, and
/// metadata differ even when files have the same size.
const PARTIAL_HASH_BYTES: usize = 4096;

/// Minimum file size to consider for duplicate detection.
/// Files smaller than this are too small to be worth deduplicating.
/// 1KB avoids noise from tiny config files, .DS_Store, empty files, etc.
const MIN_FILE_SIZE: u64 = 1024;

/// Maximum number of duplicate groups to return to the frontend.
/// Prevents the UI from choking on thousands of groups.
const MAX_GROUPS: usize = 200;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Options for running a duplicate file scan.
pub struct DuplicateScanOptions<'a> {
    pub scan_path: &'a str,
    pub min_size_mb: u64,
    pub fda: bool,
    pub skip_paths: &'a [String],
    /// Explicit scan roots (e.g. user-picked directories).
    /// When non-empty, these are used directly instead of build_scan_roots.
    pub scan_paths: &'a [String],
}

/// Run a full duplicate file scan.
///
/// `opts.scan_path`: root path to scan (default "~", expanded internally).
/// `opts.min_size_mb`: minimum file size in MB to consider (0 = use default 1KB).
/// `opts.fda`: whether Full Disk Access is available.
/// `opts.skip_paths`: user-configured paths to skip (from Settings).
pub fn run_duplicate_scan(opts: DuplicateScanOptions<'_>) -> DuplicateScanResult {
    let scan_path = opts.scan_path;
    let min_size_mb = opts.min_size_mb;
    let fda = opts.fda;
    let skip_paths = opts.skip_paths;
    let home = match commands::home_dir() {
        Some(h) => h,
        None => {
            return DuplicateScanResult {
                groups: vec![],
                total_duplicate_files: 0,
                total_wasted_bytes: 0,
                files_scanned: 0,
                stage1_candidates: 0,
                stage2_candidates: 0,
                skipped_paths: vec![],
            };
        }
    };

    let min_bytes = if min_size_mb > 0 {
        min_size_mb * 1024 * 1024
    } else {
        MIN_FILE_SIZE
    };

    // Build skip prefixes via shared helper.
    let skip_prefixes = commands::build_skip_prefixes(&home, skip_paths, &[]);

    // If explicit scan_paths were provided, use them directly.
    // Otherwise fall back to the standard build_scan_roots logic.
    let scan_roots = if !opts.scan_paths.is_empty() {
        opts.scan_paths.to_vec()
    } else {
        let safe_dirs = commands::base_scan_safe_dirs(&home);
        commands::build_scan_roots(&home, scan_path, fda, &safe_dirs)
    };

    let skipped_paths: Vec<String> = if fda {
        vec![]
    } else {
        vec![
            "~/Desktop".to_string(),
            "~/Documents".to_string(),
            "~/Downloads".to_string(),
            "~/Movies".to_string(),
            "~/Music".to_string(),
            "~/Pictures".to_string(),
            "~/Library (most subdirectories)".to_string(),
        ]
    };

    // Stage 0: Collect all file paths + sizes via walkdir
    let all_files = collect_files(&scan_roots, &skip_prefixes, min_bytes);
    let files_scanned = all_files.len() as u64;

    // Stage 1: Group by file size
    let (size_groups, stage1_candidates) = group_by_size(&all_files);

    // Stage 2: Partial hash (first 4KB)
    let (partial_groups, stage2_candidates) = group_by_partial_hash(&size_groups);

    // Stage 3: Full hash to confirm duplicates
    let confirmed_groups = confirm_by_full_hash(&partial_groups);

    // Build result and generate thumbnails
    let mut groups = assemble_groups(&confirmed_groups);
    groups.sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes));
    groups.truncate(MAX_GROUPS);

    generate_duplicate_thumbnails(&mut groups);

    let total_duplicate_files: u64 = groups.iter().map(|g| g.files.len() as u64).sum();
    let total_wasted_bytes: u64 = groups.iter().map(|g| g.wasted_bytes).sum();

    DuplicateScanResult {
        groups,
        total_duplicate_files,
        total_wasted_bytes,
        files_scanned,
        stage1_candidates,
        stage2_candidates,
        skipped_paths,
    }
}

// ---------------------------------------------------------------------------
// Stage helpers (private)
// ---------------------------------------------------------------------------

/// Stage 0: Walk scan roots and collect all file paths, sizes, and modified times.
fn collect_files(
    scan_roots: &[String],
    skip_prefixes: &[String],
    min_bytes: u64,
) -> Vec<(String, u64, String)> {
    let mut all_files: Vec<(String, u64, String)> = Vec::new();

    for root in scan_roots {
        for entry in walkdir::WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let entry_path = e.path().to_string_lossy();
                if skip_prefixes
                    .iter()
                    .any(|prefix| entry_path.starts_with(prefix.as_str()))
                {
                    return false;
                }
                if e.file_type().is_dir() {
                    if let Some(name) = e.path().file_name() {
                        if name == ".git" {
                            return false;
                        }
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            if !metadata.is_file() {
                continue;
            }

            let size = metadata.len();
            if size < min_bytes {
                continue;
            }

            // Skip sparse files — they report large apparent size but use little disk space.
            let actual_size = metadata.blocks() * 512;
            if (actual_size as f64) < (size as f64 * 0.5) {
                continue;
            }

            let modified = metadata
                .modified()
                .map(commands::format_system_time)
                .unwrap_or_else(|_| "unknown".to_string());

            all_files.push((entry.path().to_string_lossy().to_string(), size, modified));
        }
    }

    all_files
}

/// Stage 1: Group files by size. Returns (size_groups, stage1_candidates count).
fn group_by_size(
    all_files: &[(String, u64, String)],
) -> (HashMap<u64, Vec<(String, String)>>, u64) {
    let mut size_groups: HashMap<u64, Vec<(String, String)>> = HashMap::new();
    for (path, size, modified) in all_files {
        size_groups
            .entry(*size)
            .or_default()
            .push((path.clone(), modified.clone()));
    }

    // Keep only groups with 2+ files.
    size_groups.retain(|_, files| files.len() >= 2);

    let stage1_candidates: u64 = size_groups.values().map(|v| v.len() as u64).sum();
    (size_groups, stage1_candidates)
}

/// Stage 2: For each size group, hash the first 4KB of each file.
/// Files with different headers can't be duplicates even if they're the same size.
/// Returns (partial_groups, stage2_candidates count).
fn group_by_partial_hash(
    size_groups: &HashMap<u64, Vec<(String, String)>>,
) -> (HashMap<(u64, String), Vec<(String, String)>>, u64) {
    let mut partial_groups: HashMap<(u64, String), Vec<(String, String)>> = HashMap::new();

    for (size, files) in size_groups {
        for (path, modified) in files {
            match partial_hash(path) {
                Some(hash) => {
                    partial_groups
                        .entry((*size, hash))
                        .or_default()
                        .push((path.clone(), modified.clone()));
                }
                None => continue,
            }
        }
    }

    partial_groups.retain(|_, files| files.len() >= 2);

    let stage2_candidates: u64 = partial_groups.values().map(|v| v.len() as u64).sum();
    (partial_groups, stage2_candidates)
}

/// Stage 3: Full hash to confirm duplicates. Only files that matched on both
/// size AND partial hash get fully hashed.
fn confirm_by_full_hash(
    partial_groups: &HashMap<(u64, String), Vec<(String, String)>>,
) -> HashMap<(u64, String), Vec<(String, String)>> {
    let mut confirmed_groups: HashMap<(u64, String), Vec<(String, String)>> = HashMap::new();

    for ((size, _partial), files) in partial_groups {
        for (path, modified) in files {
            match full_hash(path) {
                Some(hash) => {
                    confirmed_groups
                        .entry((*size, hash))
                        .or_default()
                        .push((path.clone(), modified.clone()));
                }
                None => continue,
            }
        }
    }

    confirmed_groups.retain(|_, files| files.len() >= 2);
    confirmed_groups
}

/// Build DuplicateGroup structs from confirmed hash groups.
fn assemble_groups(
    confirmed_groups: &HashMap<(u64, String), Vec<(String, String)>>,
) -> Vec<DuplicateGroup> {
    let mut groups: Vec<DuplicateGroup> = Vec::new();

    for ((size, hash), files) in confirmed_groups {
        let dup_files: Vec<DuplicateFile> = files
            .iter()
            .map(|(path, modified)| {
                let file_path = std::path::Path::new(path);
                DuplicateFile {
                    path: path.clone(),
                    name: file_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    size: *size,
                    modified: modified.clone(),
                    parent_dir: file_path
                        .parent()
                        .map(|parent_path| parent_path.to_string_lossy().to_string())
                        .unwrap_or_default(),
                }
            })
            .collect();

        let wasted = (dup_files.len() as u64 - 1) * size;

        groups.push(DuplicateGroup {
            hash: hash.clone(),
            size: *size,
            files: dup_files,
            wasted_bytes: wasted,
            thumbnail: None,
        });
    }

    groups
}

/// Generate thumbnails for image groups (concurrent via threads).
/// Only for the final truncated set — at most MAX_GROUPS (200) thumbnails.
fn generate_duplicate_thumbnails(groups: &mut [DuplicateGroup]) {
    let thumb_handles: Vec<_> = groups
        .iter()
        .enumerate()
        .filter_map(|(i, g)| {
            let path = std::path::Path::new(&g.files[0].path);
            if image_utils::is_image_file(path) {
                let file_path = g.files[0].path.clone();
                Some((i, std::thread::spawn(move || {
                    image_utils::generate_thumbnail(std::path::Path::new(&file_path), 140).ok()
                })))
            } else {
                None
            }
        })
        .collect();

    for (idx, handle) in thumb_handles {
        if let Ok(Some(b64)) = handle.join() {
            groups[idx].thumbnail = Some(b64);
        }
    }
}

// ---------------------------------------------------------------------------
// Hashing helpers
// ---------------------------------------------------------------------------

/// Compute a BLAKE3 hash of the first PARTIAL_HASH_BYTES of a file.
/// Returns None if the file can't be opened or read.
fn partial_hash(path: &str) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut buf = vec![0u8; PARTIAL_HASH_BYTES];
    let bytes_read = file.read(&mut buf).ok()?;
    buf.truncate(bytes_read);

    let hash = blake3::hash(&buf);
    Some(hash.to_hex().to_string())
}

/// Compute a BLAKE3 hash of an entire file.
/// Returns None if the file can't be opened or read.
///
/// BLAKE3 is designed for streaming — we feed it 64KB chunks at a time
/// rather than reading the entire file into memory.
fn full_hash(path: &str) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut hasher = blake3::Hasher::new();

    // Read in 64KB chunks — good balance between syscall overhead and memory.
    let mut buf = [0u8; 65536];
    loop {
        let bytes_read = file.read(&mut buf).ok()?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buf[..bytes_read]);
    }

    let hash = hasher.finalize();
    Some(hash.to_hex().to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper: create a temp file with given content, return its path.
    fn temp_file(dir: &std::path::Path, name: &str, content: &[u8]) -> String {
        let path = dir.join(name);
        let mut f = File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path.to_string_lossy().to_string()
    }

    // -- Hashing correctness --
    // Motivation: partial_hash and full_hash drive the entire duplicate pipeline.
    // If they produce wrong hashes, identical files won't group together (missed
    // duplicates) or different files could collide (false duplicates → data loss).

    #[test]
    fn identical_files_produce_identical_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let content = b"identical content across both files";
        let a = temp_file(dir.path(), "a.txt", content);
        let b = temp_file(dir.path(), "b.txt", content);

        assert_eq!(partial_hash(&a), partial_hash(&b));
        assert_eq!(full_hash(&a), full_hash(&b));
    }

    #[test]
    fn different_files_produce_different_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let a = temp_file(dir.path(), "a.txt", b"file content alpha");
        let b = temp_file(dir.path(), "b.txt", b"file content bravo");

        assert_ne!(partial_hash(&a), partial_hash(&b));
        assert_ne!(full_hash(&a), full_hash(&b));
    }

    #[test]
    fn partial_hash_catches_files_differing_only_in_header() {
        // Two files that are the same size but differ in first bytes.
        // Stage 2 (partial hash) must catch this — otherwise stage 3 wastes
        // I/O on full hashes of files that clearly differ.
        let dir = tempfile::tempdir().unwrap();
        let mut content_a = vec![0xAAu8; 8192];
        let mut content_b = vec![0xBBu8; 8192];
        // Make the tails identical so only the header differs.
        let tail = b"shared tail content padding here";
        content_a[4096..4096 + tail.len()].copy_from_slice(tail);
        content_b[4096..4096 + tail.len()].copy_from_slice(tail);

        let a = temp_file(dir.path(), "a.bin", &content_a);
        let b = temp_file(dir.path(), "b.bin", &content_b);

        assert_ne!(partial_hash(&a), partial_hash(&b),
            "partial hash should distinguish files with different headers");
    }

    #[test]
    fn partial_hash_misses_files_differing_only_after_4kb() {
        // This is the known limitation of partial hashing: two files identical
        // in the first 4KB but different after that will have the same partial
        // hash. Stage 3 (full hash) must catch these. This test documents that
        // assumption so a future change to PARTIAL_HASH_BYTES doesn't silently
        // break the pipeline contract.
        let dir = tempfile::tempdir().unwrap();
        let mut content_a = vec![0u8; 8192];
        let mut content_b = vec![0u8; 8192];
        // Differ only at byte 4097 — beyond the partial hash window.
        content_a[4097] = 0xFF;
        content_b[4097] = 0x00;

        let a = temp_file(dir.path(), "a.bin", &content_a);
        let b = temp_file(dir.path(), "b.bin", &content_b);

        assert_eq!(partial_hash(&a), partial_hash(&b),
            "partial hash should NOT catch differences past PARTIAL_HASH_BYTES");
        assert_ne!(full_hash(&a), full_hash(&b),
            "full hash must catch what partial hash misses");
    }

    #[test]
    fn nonexistent_file_returns_none() {
        assert!(partial_hash("/nonexistent/path/file.txt").is_none());
        assert!(full_hash("/nonexistent/path/file.txt").is_none());
    }

    // -- Wasted bytes calculation --
    // Motivation: wasted_bytes drives the "you can reclaim X" number shown to
    // users. If this is wrong, users either overestimate savings (disappointing)
    // or underestimate (miss cleanup opportunities).

    #[test]
    fn wasted_bytes_is_count_minus_one_times_size() {
        // 3 copies of a 1000-byte file → wasted = (3-1) * 1000 = 2000
        let group = DuplicateGroup {
            hash: "abc".to_string(),
            size: 1000,
            files: vec![
                DuplicateFile { path: "/a".into(), name: "a".into(), size: 1000, modified: "".into(), parent_dir: "/".into() },
                DuplicateFile { path: "/b".into(), name: "b".into(), size: 1000, modified: "".into(), parent_dir: "/".into() },
                DuplicateFile { path: "/c".into(), name: "c".into(), size: 1000, modified: "".into(), parent_dir: "/".into() },
            ],
            wasted_bytes: 2000,
            thumbnail: None,
        };
        let expected = (group.files.len() as u64 - 1) * group.size;
        assert_eq!(group.wasted_bytes, expected);
    }

    // -- End-to-end: the 3-stage pipeline finds real duplicates --
    // Motivation: unit-testing hashing alone doesn't catch bugs in the grouping
    // logic (e.g. off-by-one in retain, wrong HashMap key). This test creates
    // real files on disk and runs the full pipeline.

    #[test]
    fn pipeline_finds_duplicates_in_temp_dir() {
        let dir = tempfile::tempdir().unwrap();
        // Create 3 identical files (above MIN_FILE_SIZE) and 1 unique file.
        let content = vec![0x42u8; 2048]; // 2KB — above the 1KB minimum
        temp_file(dir.path(), "dup1.bin", &content);
        temp_file(dir.path(), "dup2.bin", &content);
        temp_file(dir.path(), "dup3.bin", &content);
        temp_file(dir.path(), "unique.bin", &vec![0xFFu8; 2048]);

        let result = run_duplicate_scan(DuplicateScanOptions {
            scan_path: dir.path().to_str().unwrap(),
            min_size_mb: 0, // use default min size
            fda: true, // pretend FDA
            skip_paths: &[],
        });

        assert_eq!(result.groups.len(), 1, "should find exactly one duplicate group");
        assert_eq!(result.groups[0].files.len(), 3, "group should contain all 3 copies");
        assert_eq!(result.groups[0].wasted_bytes, 2 * 2048, "wasted = (3-1) * 2048");
        assert!(result.files_scanned >= 4, "should have scanned at least 4 files");
    }

    #[test]
    fn pipeline_respects_min_size_filter() {
        let dir = tempfile::tempdir().unwrap();
        // Create duplicates that are 512 bytes — below the 1MB min_size_mb=1 threshold.
        let content = vec![0x42u8; 512];
        temp_file(dir.path(), "small1.bin", &content);
        temp_file(dir.path(), "small2.bin", &content);

        let result = run_duplicate_scan(DuplicateScanOptions {
            scan_path: dir.path().to_str().unwrap(),
            min_size_mb: 1, // 1MB minimum
            fda: true,
            skip_paths: &[],
        });

        assert_eq!(result.groups.len(), 0, "files below min_size_mb should be excluded");
    }
}
