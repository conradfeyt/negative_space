// vault.rs — Compress-instead-of-delete file archival for Negative _.
//
// The Vault lets users reclaim disk space by compressing large, stale files
// into a managed archive. Files can be restored to their original path at
// any time. This addresses the core anxiety of disk cleanup: users need
// space but are afraid to permanently delete files they might need.
//
// COMPRESSION: zstd level 3 (good balance of speed and ratio).
// STORAGE: ~/Library/Application Support/NegativeSpace/vault/
// INTEGRITY: BLAKE3 hash verified on both compress and restore.
// DEDUP: Files with identical content share a single compressed copy.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use crate::commands;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VaultEntry {
    pub id: String,
    pub original_path: String,
    pub original_size: u64,
    pub compressed_size: u64,
    pub compression_ratio: f64,
    pub blake3_hash: String,
    pub vault_filename: String,
    pub archived_at: String,
    pub original_modified: String,
    pub original_accessed: String,
    pub permissions: u32,
    pub file_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VaultManifest {
    pub version: u32,
    pub entries: Vec<VaultEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VaultSummary {
    pub file_count: u64,
    pub total_original_size: u64,
    pub total_compressed_size: u64,
    pub total_savings: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompressionCandidate {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub estimated_compressed_size: u64,
    pub estimated_savings: u64,
    pub estimated_ratio: f64,
    pub last_accessed: String,
    pub last_modified: String,
    pub file_type: String,
    pub recently_accessed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompressResult {
    pub success: bool,
    pub files_compressed: u64,
    pub total_original_size: u64,
    pub total_compressed_size: u64,
    pub total_savings: u64,
    pub errors: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RestoreResult {
    pub success: bool,
    pub restored_path: String,
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ZSTD_LEVEL: i32 = 3;
const SAMPLE_BYTES: usize = 65536; // 64KB for compression estimation
const MIN_SAVINGS_RATIO: f64 = 0.20; // Skip files with <20% estimated savings

/// File extensions that are already compressed — skip these.
const SKIP_EXTENSIONS: &[&str] = &[
    // Images
    "jpg", "jpeg", "png", "gif", "webp", "heic", "heif", "raw", "tiff", "tif",
    "bmp", "ico", "svg",
    // Video
    "mp4", "mov", "avi", "mkv", "wmv", "webm", "m4v", "flv",
    // Audio
    "mp3", "aac", "flac", "ogg", "m4a", "wma", "opus",
    // Archives
    "zip", "gz", "bz2", "xz", "zst", "7z", "rar", "tar", "tgz", "dmg", "iso",
    // App bundles (would break code signatures)
    "app",
];

// ---------------------------------------------------------------------------
// Vault paths
// ---------------------------------------------------------------------------

fn vault_dir() -> Option<String> {
    let home = commands::home_dir()?;
    let new_path = format!("{}/Documents/MyNegativeSpaceVault", home);
    let old_path = format!("{}/Library/Application Support/NegativeSpace/vault", home);
    if std::path::Path::new(&new_path).exists() {
        Some(new_path)
    } else if std::path::Path::new(&old_path).exists() {
        Some(old_path)
    } else {
        Some(new_path)
    }
}

fn vault_data_dir() -> Option<String> {
    Some(format!("{}/data", vault_dir()?))
}

fn manifest_path() -> Option<String> {
    Some(format!("{}/manifest.json", vault_dir()?))
}

fn ensure_vault_dirs() -> Result<(), String> {
    let data_dir = vault_data_dir().ok_or("Cannot determine vault directory")?;
    fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create vault directory: {}", e))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Manifest I/O
// ---------------------------------------------------------------------------

fn load_manifest() -> VaultManifest {
    let empty = || VaultManifest { version: 1, entries: vec![] };

    let path = match manifest_path() {
        Some(p) => p,
        None => return empty(),
    };
    let data = match fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return empty(),
    };
    match serde_json::from_str(&data) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[vault] manifest parse failed: {}. Backing up corrupt file.", e);
            let backup = format!("{}.corrupt", path);
            if let Err(be) = fs::copy(&path, &backup) {
                eprintln!("[vault] failed to back up corrupt manifest: {}", be);
            }
            empty()
        }
    }
}

fn save_manifest(manifest: &VaultManifest) -> Result<(), String> {
    let path = manifest_path().ok_or("Cannot determine manifest path")?;
    let data = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    fs::write(&path, &data)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

fn blake3_hash_file(path: &str) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|e| format!("Cannot open {}: {}", path, e))?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 65536];
    loop {
        let bytes_read = file.read(&mut buf)
            .map_err(|e| format!("Read error on {}: {}", path, e))?;
        if bytes_read == 0 { break; }
        hasher.update(&buf[..bytes_read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

// ---------------------------------------------------------------------------
// Compression estimation
// ---------------------------------------------------------------------------

fn estimate_compression(path: &str) -> Option<f64> {
    let mut file = fs::File::open(path).ok()?;
    let mut sample = vec![0u8; SAMPLE_BYTES];
    let bytes_read = file.read(&mut sample).ok()?;
    if bytes_read == 0 { return None; }
    sample.truncate(bytes_read);

    let compressed = zstd::encode_all(sample.as_slice(), ZSTD_LEVEL).ok()?;
    let ratio = compressed.len() as f64 / bytes_read as f64;
    Some(ratio)
}

fn is_skip_extension(path: &str) -> bool {
    let lower = path.to_lowercase();
    SKIP_EXTENSIONS.iter().any(|ext| lower.ends_with(&format!(".{}", ext)))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Options for scanning vault compression candidates.
pub struct VaultScanOptions<'a> {
    pub scan_path: &'a str,
    pub min_size_mb: u64,
    pub min_age_days: u64,
    pub fda: bool,
}

/// Scan for files that are good compression candidates.
pub fn scan_candidates(opts: VaultScanOptions<'_>) -> Vec<CompressionCandidate> {
    let scan_path = opts.scan_path;
    let min_size_mb = opts.min_size_mb;
    let min_age_days = opts.min_age_days;
    let fda = opts.fda;
    let home = match commands::home_dir() {
        Some(h) => h,
        None => return vec![],
    };

    let min_bytes = min_size_mb.max(10) * 1024 * 1024;
    let min_age_secs = min_age_days * 86400;
    let now = std::time::SystemTime::now();
    let recent_threshold = 7 * 86400; // 7 days = "recently accessed"

    // Resolve scan root via shared helper (FDA always true here — vault
    // scans from a single resolved root, not a safe_dirs whitelist).
    let scan_roots = commands::build_scan_roots(&home, scan_path, true, &[]);
    let root = scan_roots.into_iter().next().unwrap_or_else(|| home.clone());

    // Load existing vault to skip already-archived files
    let manifest = load_manifest();
    let archived_paths: std::collections::HashSet<String> =
        manifest.entries.iter().map(|e| e.original_path.clone()).collect();

    // Vault-specific extra skip prefixes: vault data dir + ~/Library when no FDA.
    let vault = vault_dir().unwrap_or_default();
    let mut extra = vec![vault];
    if !fda {
        extra.push(format!("{}/Library", home));
    }
    // No user skip_paths for vault; extra prefixes are vault-specific.
    let skip_prefixes = commands::build_skip_prefixes(&home, &[], &extra);

    let mut candidates = Vec::new();

    for entry in walkdir::WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let p = e.path().to_string_lossy();
            !skip_prefixes.iter().any(|prefix| p.starts_with(prefix.as_str()))
                && !(e.file_type().is_dir() && e.path().file_name().map_or(false, |n| n == ".git"))
        })
        .filter_map(|e| e.ok())
    {
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if !meta.is_file() || meta.len() < min_bytes {
            continue;
        }

        let path_str = entry.path().to_string_lossy().to_string();

        // Skip already-compressed formats
        if is_skip_extension(&path_str) {
            continue;
        }

        // Skip already archived
        if archived_paths.contains(&path_str) {
            continue;
        }

        // Check age (last accessed)
        let accessed_age = now.duration_since(
            meta.accessed().unwrap_or(std::time::UNIX_EPOCH)
        ).unwrap_or_default().as_secs();

        if accessed_age < min_age_secs {
            continue;
        }

        let recently_accessed = accessed_age < recent_threshold;

        // Estimate compression
        let ratio = match estimate_compression(&path_str) {
            Some(r) => r,
            None => continue,
        };

        // Skip if savings would be <20%
        if ratio > (1.0 - MIN_SAVINGS_RATIO) {
            continue;
        }

        let size = meta.len();
        let estimated_compressed = (size as f64 * ratio) as u64;
        let estimated_savings = size.saturating_sub(estimated_compressed);

        let name = entry.path().file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let file_type = entry.path().extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        let last_accessed = meta.accessed()
            .map(commands::format_system_time)
            .unwrap_or_else(|_| "unknown".to_string());

        let last_modified = meta.modified()
            .map(commands::format_system_time)
            .unwrap_or_else(|_| "unknown".to_string());

        candidates.push(CompressionCandidate {
            path: path_str,
            name,
            size,
            estimated_compressed_size: estimated_compressed,
            estimated_savings,
            estimated_ratio: ratio,
            last_accessed,
            last_modified,
            file_type,
            recently_accessed,
        });
    }

    // Sort by estimated savings descending
    candidates.sort_by(|a, b| b.estimated_savings.cmp(&a.estimated_savings));

    // Cap at 200 results
    candidates.truncate(200);
    candidates
}

/// Compress files into the vault.
pub fn compress_files(paths: &[String]) -> CompressResult {
    let mut result = CompressResult {
        success: true,
        files_compressed: 0,
        total_original_size: 0,
        total_compressed_size: 0,
        total_savings: 0,
        errors: vec![],
    };

    if let Err(e) = ensure_vault_dirs() {
        result.success = false;
        result.errors.push(e);
        return result;
    }

    let data_dir = match vault_data_dir() {
        Some(d) => d,
        None => {
            result.success = false;
            result.errors.push("Cannot determine vault directory".to_string());
            return result;
        }
    };

    let mut manifest = load_manifest();

    // Build a map of existing hashes for dedup
    let mut hash_refcount: HashMap<String, u32> = HashMap::new();
    for entry in &manifest.entries {
        *hash_refcount.entry(entry.blake3_hash.clone()).or_insert(0) += 1;
    }

    for path_str in paths {
        match compress_single_file(path_str, &data_dir) {
            Ok((entry, original_size, compressed_size)) => {
                crate::spotlight::index_vault_entry(&entry);
                let hash = entry.blake3_hash.clone();
                manifest.entries.push(entry);
                *hash_refcount.entry(hash).or_insert(0) += 1;

                // Delete original — the critical step. Only after verified compression.
                match fs::remove_file(Path::new(path_str)) {
                    Ok(_) => {
                        result.files_compressed += 1;
                        result.total_original_size += original_size;
                        result.total_compressed_size += compressed_size;
                        result.total_savings += original_size.saturating_sub(compressed_size);
                    }
                    Err(e) => {
                        result.errors.push(format!("Compressed but failed to delete original {}: {}", path_str, e));
                    }
                }
            }
            Err(e) => {
                result.errors.push(e);
            }
        }
    }

    if let Err(e) = save_manifest(&manifest) {
        result.errors.push(format!("Failed to save manifest: {}", e));
    }

    result
}

/// Compress a single file into the vault data directory.
/// Returns (VaultEntry, original_size, compressed_size) on success,
/// or an error message string on failure.
fn compress_single_file(
    path_str: &str,
    data_dir: &str,
) -> Result<(VaultEntry, u64, u64), String> {
    let path = Path::new(path_str);

    // Read original size before compression (metadata for dates/permissions read later by build_vault_entry)
    let original_size = fs::metadata(path)
        .map_err(|e| format!("{}: {}", path_str, e))?
        .len();

    // Hash the original
    let hash = blake3_hash_file(path_str)?;

    let vault_filename = format!("{}.zst", hash);
    let vault_path = format!("{}/{}", data_dir, vault_filename);

    // Compress (skip if already exists from dedup)
    if !Path::new(&vault_path).exists() {
        if let Err(e) = compress_file_to(path_str, &vault_path) {
            let _ = fs::remove_file(&vault_path);
            return Err(format!("{}: {}", path_str, e));
        }

        // Verify: decompress and check hash
        match verify_compressed(&vault_path, &hash) {
            Ok(true) => {}
            Ok(false) => {
                let _ = fs::remove_file(&vault_path);
                return Err(format!("{}: integrity check failed after compression", path_str));
            }
            Err(e) => {
                let _ = fs::remove_file(&vault_path);
                return Err(format!("{}: verification error: {}", path_str, e));
            }
        }
    }

    let compressed_size = fs::metadata(&vault_path)
        .map(|m| m.len())
        .unwrap_or(0);

    let ratio = if original_size > 0 {
        compressed_size as f64 / original_size as f64
    } else {
        1.0
    };

    let file_type = Path::new(path_str)
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    let entry = build_vault_entry(path_str, original_size, compressed_size, ratio, hash, vault_filename, &file_type);

    Ok((entry, original_size, compressed_size))
}

/// Restore a file from the vault to its original location.
pub fn restore_file(entry_id: &str) -> RestoreResult {
    let mut manifest = load_manifest();

    let entry_idx = match manifest.entries.iter().position(|e| e.id == entry_id) {
        Some(i) => i,
        None => return RestoreResult {
            success: false,
            restored_path: String::new(),
            errors: vec![format!("Entry {} not found in vault", entry_id)],
        },
    };

    let entry = &manifest.entries[entry_idx];
    let data_dir = match vault_data_dir() {
        Some(d) => d,
        None => return RestoreResult {
            success: false,
            restored_path: String::new(),
            errors: vec!["Cannot determine vault directory".to_string()],
        },
    };

    let vault_path = format!("{}/{}", data_dir, entry.vault_filename);
    let original_path = entry.original_path.clone();
    let expected_hash = entry.blake3_hash.clone();
    let permissions = entry.permissions;

    // Ensure parent directory exists
    if let Some(parent) = Path::new(&original_path).parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return RestoreResult {
                success: false,
                restored_path: original_path,
                errors: vec![format!("Cannot create directory: {}", e)],
            };
        }
    }

    let is_directory = entry.file_type == "directory";

    // Check if path already exists
    if Path::new(&original_path).exists() {
        return RestoreResult {
            success: false,
            restored_path: original_path,
            errors: vec!["A file or directory already exists at the original path. Remove it first or restore to a different location.".to_string()],
        };
    }

    if is_directory {
        // Restore directory from tar.zst
        if let Err(e) = restore_directory(&vault_path, &original_path) {
            let mut errors = vec![format!("Directory restore failed: {}", e)];
            if let Err(ce) = fs::remove_dir_all(&original_path) {
                errors.push(format!("Warning: failed to clean up partial restore at {}: {}", original_path, ce));
            }
            return RestoreResult {
                success: false,
                restored_path: original_path,
                errors,
            };
        }
    } else {
        // Restore single file from .zst
        if let Err(e) = decompress_file_to(&vault_path, &original_path) {
            let mut errors = vec![format!("Decompression failed: {}", e)];
            if let Err(ce) = fs::remove_file(&original_path) {
                errors.push(format!("Warning: failed to clean up partial restore at {}: {}", original_path, ce));
            }
            return RestoreResult {
                success: false,
                restored_path: original_path,
                errors,
            };
        }

        // Verify hash for single files
        match blake3_hash_file(&original_path) {
            Ok(hash) if hash == expected_hash => {}
            Ok(_) => {
                let mut errors = vec!["Integrity check failed: restored file hash does not match original".to_string()];
                if let Err(ce) = fs::remove_file(&original_path) {
                    errors.push(format!("Warning: failed to clean up partial restore at {}: {}", original_path, ce));
                }
                return RestoreResult {
                    success: false,
                    restored_path: original_path,
                    errors,
                };
            }
            Err(e) => {
                let mut errors = vec![format!("Hash verification error: {}", e)];
                if let Err(ce) = fs::remove_file(&original_path) {
                    errors.push(format!("Warning: failed to clean up partial restore at {}: {}", original_path, ce));
                }
                return RestoreResult {
                    success: false,
                    restored_path: original_path,
                    errors,
                };
            }
        }

        // Restore permissions for single files
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&original_path, fs::Permissions::from_mode(permissions));
    }

    // Check if other entries reference the same compressed file (dedup)
    let hash_ref_count = manifest.entries.iter()
        .filter(|e| e.blake3_hash == expected_hash)
        .count();

    // Remove entry from manifest and Spotlight
    crate::spotlight::deindex_vault_entry(entry_id);
    manifest.entries.remove(entry_idx);

    // Only delete compressed file if no other entries reference it
    if hash_ref_count <= 1 {
        if let Err(e) = fs::remove_file(&vault_path) {
            eprintln!("[vault] failed to remove vault file {}: {}", vault_path, e);
        }
    }

    if let Err(e) = save_manifest(&manifest) {
        return RestoreResult {
            success: true, // File was restored, manifest save failed
            restored_path: original_path,
            errors: vec![format!("File restored but manifest save failed: {}", e)],
        };
    }

    RestoreResult {
        success: true,
        restored_path: original_path,
        errors: vec![],
    }
}

/// Get a summary of the vault contents.
pub fn get_summary() -> VaultSummary {
    let manifest = load_manifest();
    let total_original: u64 = manifest.entries.iter().map(|e| e.original_size).sum();
    let total_compressed: u64 = manifest.entries.iter().map(|e| e.compressed_size).sum();

    VaultSummary {
        file_count: manifest.entries.len() as u64,
        total_original_size: total_original,
        total_compressed_size: total_compressed,
        total_savings: total_original.saturating_sub(total_compressed),
    }
}

/// Get all vault entries.
pub fn get_entries() -> Vec<VaultEntry> {
    load_manifest().entries
}

/// Permanently delete a vault entry (no restore possible after this).
pub fn delete_entry(entry_id: &str) -> Result<(), String> {
    let mut manifest = load_manifest();

    let entry_idx = manifest.entries.iter().position(|e| e.id == entry_id)
        .ok_or_else(|| format!("Entry {} not found", entry_id))?;

    let entry = &manifest.entries[entry_idx];
    let hash = entry.blake3_hash.clone();
    let vault_filename = entry.vault_filename.clone();

    // Check if other entries reference the same file
    let hash_ref_count = manifest.entries.iter()
        .filter(|e| e.blake3_hash == hash)
        .count();

    crate::spotlight::deindex_vault_entry(entry_id);
    manifest.entries.remove(entry_idx);

    // Only delete compressed file if no other entries reference it
    if hash_ref_count <= 1 {
        if let Some(data_dir) = vault_data_dir() {
            let vault_path = format!("{}/{}", data_dir, vault_filename);
            if let Err(e) = fs::remove_file(&vault_path) {
                eprintln!("[vault] failed to remove vault file {}: {}", vault_path, e);
            }
        }
    }

    save_manifest(&manifest)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn compress_file_to(src: &str, dst: &str) -> Result<(), String> {
    let input = fs::File::open(src)
        .map_err(|e| format!("Cannot open source: {}", e))?;
    let output = fs::File::create(dst)
        .map_err(|e| format!("Cannot create compressed file: {}", e))?;

    let mut reader = std::io::BufReader::new(input);
    let mut encoder = zstd::Encoder::new(output, ZSTD_LEVEL)
        .map_err(|e| format!("Cannot create zstd encoder: {}", e))?;

    std::io::copy(&mut reader, &mut encoder)
        .map_err(|e| format!("Compression failed: {}", e))?;

    encoder.finish()
        .map_err(|e| format!("Cannot finalize compression: {}", e))?;

    Ok(())
}

fn decompress_file_to(src: &str, dst: &str) -> Result<(), String> {
    let input = fs::File::open(src)
        .map_err(|e| format!("Cannot open compressed file: {}", e))?;
    let mut output = fs::File::create(dst)
        .map_err(|e| format!("Cannot create output file: {}", e))?;

    let mut decoder = zstd::Decoder::new(input)
        .map_err(|e| format!("Cannot create zstd decoder: {}", e))?;

    std::io::copy(&mut decoder, &mut output)
        .map_err(|e| format!("Decompression failed: {}", e))?;

    output.flush()
        .map_err(|e| format!("Cannot flush output: {}", e))?;

    Ok(())
}

fn verify_compressed(vault_path: &str, expected_hash: &str) -> Result<bool, String> {
    // Decompress to memory and hash
    let input = fs::File::open(vault_path)
        .map_err(|e| format!("Cannot open for verification: {}", e))?;
    let mut decoder = zstd::Decoder::new(input)
        .map_err(|e| format!("Cannot create decoder for verification: {}", e))?;

    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 65536];
    loop {
        let bytes_read = decoder.read(&mut buf)
            .map_err(|e| format!("Read error during verification: {}", e))?;
        if bytes_read == 0 { break; }
        hasher.update(&buf[..bytes_read]);
    }

    let hash = hasher.finalize().to_hex().to_string();
    Ok(hash == expected_hash)
}

/// Build a VaultEntry from compression results and filesystem metadata.
fn build_vault_entry(
    original_path: &str,
    original_size: u64,
    compressed_size: u64,
    compression_ratio: f64,
    blake3_hash: String,
    vault_filename: String,
    file_type: &str,
) -> VaultEntry {
    let path = Path::new(original_path);
    let meta = fs::metadata(path).ok();
    let permissions = meta.as_ref().map(|m| m.mode()).unwrap_or(0o755);
    let modified = meta.as_ref()
        .and_then(|m| m.modified().ok())
        .map(commands::format_system_time)
        .unwrap_or_else(|| "unknown".to_string());
    let accessed = meta.as_ref()
        .and_then(|m| m.accessed().ok())
        .map(commands::format_system_time)
        .unwrap_or_else(|| "unknown".to_string());

    VaultEntry {
        id: uuid::Uuid::new_v4().to_string(),
        original_path: original_path.to_string(),
        original_size,
        compressed_size,
        compression_ratio,
        blake3_hash,
        vault_filename,
        archived_at: chrono_now(),
        original_modified: modified,
        original_accessed: accessed,
        permissions,
        file_type: file_type.to_string(),
    }
}

/// Compress an entire directory into a single vault entry (tar.zst).
/// The whole directory tree is archived as one blob and the original is removed.
pub fn compress_directory(dir_path: &str) -> CompressResult {
    let mut result = CompressResult {
        success: false,
        files_compressed: 0,
        total_original_size: 0,
        total_compressed_size: 0,
        total_savings: 0,
        errors: vec![],
    };

    let path = Path::new(dir_path);
    if !path.is_dir() {
        result.errors.push(format!("Not a directory: {}", dir_path));
        return result;
    }

    if let Err(e) = ensure_vault_dirs() {
        result.errors.push(e);
        return result;
    }

    let data_dir = match vault_data_dir() {
        Some(d) => d,
        None => {
            result.errors.push("Cannot determine vault directory".to_string());
            return result;
        }
    };

    // Calculate total directory size
    let original_size = dir_size_bytes(dir_path);

    // Create tar.zst: tar the directory into a zstd-compressed stream
    let hash_hex = blake3_hash_directory(dir_path);
    let vault_filename = format!("{}.tar.zst", hash_hex);
    let vault_path = format!("{}/{}", data_dir, vault_filename);

    if Path::new(&vault_path).exists() {
        result.errors.push("This directory is already archived in the vault".to_string());
        return result;
    }

    // Create tar.zst
    match tar_zstd_directory(dir_path, &vault_path) {
        Ok(_) => {}
        Err(e) => {
            result.errors.push(format!("Compression failed: {}", e));
            if let Err(ce) = fs::remove_file(&vault_path) {
                eprintln!("[vault] cleanup failed for {}: {}", vault_path, ce);
            }
            return result;
        }
    }

    let compressed_size = fs::metadata(&vault_path).map(|m| m.len()).unwrap_or(0);
    let ratio = if original_size > 0 { compressed_size as f64 / original_size as f64 } else { 1.0 };

    let entry = build_vault_entry(dir_path, original_size, compressed_size, ratio, hash_hex, vault_filename, "directory");

    let mut manifest = load_manifest();
    crate::spotlight::index_vault_entry(&entry);
    manifest.entries.push(entry);

    // Compression succeeded — set result regardless of deletion outcome
    result.success = true;
    result.files_compressed = 1;
    result.total_original_size = original_size;
    result.total_compressed_size = compressed_size;
    result.total_savings = original_size.saturating_sub(compressed_size);

    // Remove the original directory
    if let Err(e) = fs::remove_dir_all(path) {
        result.errors.push(format!("Compressed but failed to delete original: {}", e));
    }

    if let Err(e) = save_manifest(&manifest) {
        result.errors.push(format!("Manifest save error: {}", e));
    }

    result
}

/// Restore a directory from a tar.zst vault entry.
fn restore_directory(vault_path: &str, original_path: &str) -> Result<(), String> {
    // Ensure parent exists
    if let Some(parent) = Path::new(original_path).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create parent directory: {}", e))?;
    }

    // Extract tar.zst to the parent directory
    let parent = Path::new(original_path).parent()
        .ok_or("Cannot determine parent directory")?;

    let file = fs::File::open(vault_path)
        .map_err(|e| format!("Cannot open archive: {}", e))?;
    let decoder = zstd::Decoder::new(file)
        .map_err(|e| format!("Cannot create decoder: {}", e))?;
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(parent)
        .map_err(|e| format!("Cannot extract archive: {}", e))?;

    Ok(())
}

fn tar_zstd_directory(dir_path: &str, output_path: &str) -> Result<(), String> {
    let output = fs::File::create(output_path)
        .map_err(|e| format!("Cannot create output: {}", e))?;
    let encoder = zstd::Encoder::new(output, ZSTD_LEVEL)
        .map_err(|e| format!("Cannot create encoder: {}", e))?;
    let mut builder = tar::Builder::new(encoder);

    let dir = Path::new(dir_path);
    let dir_name = dir.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    builder.append_dir_all(&dir_name, dir)
        .map_err(|e| format!("Cannot add directory to archive: {}", e))?;

    let encoder = builder.into_inner()
        .map_err(|e| format!("Cannot finalize tar: {}", e))?;
    encoder.finish()
        .map_err(|e| format!("Cannot finalize zstd: {}", e))?;

    Ok(())
}

fn dir_size_bytes(path: &str) -> u64 {
    let mut total: u64 = 0;
    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if let Ok(m) = entry.metadata() {
            if m.is_file() {
                total += m.len();
            }
        }
    }
    total
}

/// Hash a directory by hashing all file paths + sizes (fast identity check).
fn blake3_hash_directory(path: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(path.as_bytes());
    for entry in walkdir::WalkDir::new(path)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if let Ok(m) = entry.metadata() {
            if m.is_file() {
                hasher.update(entry.path().to_string_lossy().as_bytes());
                hasher.update(&m.len().to_le_bytes());
            }
        }
    }
    hasher.finalize().to_hex().to_string()
}

/// Collect all compressible files in a user-chosen directory (no age filter).
/// Used when the user explicitly picks a folder to archive via the folder picker.
pub fn collect_directory_files(dir_path: &str) -> Vec<CompressionCandidate> {
    let manifest = load_manifest();
    let archived_paths: std::collections::HashSet<String> =
        manifest.entries.iter().map(|e| e.original_path.clone()).collect();

    let vault = vault_dir().unwrap_or_default();
    let now = std::time::SystemTime::now();
    let recent_threshold = 7 * 86400;

    let mut candidates = Vec::new();

    for entry in walkdir::WalkDir::new(dir_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let p = e.path().to_string_lossy();
            !p.starts_with(&vault)
                && !(e.file_type().is_dir() && e.path().file_name().map_or(false, |n| n == ".git"))
        })
        .filter_map(|e| e.ok())
    {
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if !meta.is_file() {
            continue;
        }

        let path_str = entry.path().to_string_lossy().to_string();

        if is_skip_extension(&path_str) || archived_paths.contains(&path_str) {
            continue;
        }

        let ratio = match estimate_compression(&path_str) {
            Some(r) => r,
            None => continue,
        };

        if ratio > (1.0 - MIN_SAVINGS_RATIO) {
            continue;
        }

        let size = meta.len();
        let estimated_compressed = (size as f64 * ratio) as u64;
        let estimated_savings = size.saturating_sub(estimated_compressed);

        let accessed_age = now.duration_since(
            meta.accessed().unwrap_or(std::time::UNIX_EPOCH)
        ).unwrap_or_default().as_secs();

        let name = entry.path().file_name()
            .unwrap_or_default().to_string_lossy().to_string();

        let file_type = entry.path().extension()
            .unwrap_or_default().to_string_lossy().to_lowercase();

        let last_accessed = meta.accessed()
            .map(commands::format_system_time)
            .unwrap_or_else(|_| "unknown".to_string());

        let last_modified = meta.modified()
            .map(commands::format_system_time)
            .unwrap_or_else(|_| "unknown".to_string());

        candidates.push(CompressionCandidate {
            path: path_str,
            name,
            size,
            estimated_compressed_size: estimated_compressed,
            estimated_savings,
            estimated_ratio: ratio,
            last_accessed,
            last_modified,
            file_type,
            recently_accessed: accessed_age < recent_threshold,
        });
    }

    candidates.sort_by(|a, b| b.estimated_savings.cmp(&a.estimated_savings));
    candidates.truncate(500);
    candidates
}

/// Simple ISO-ish timestamp without pulling in chrono.
fn chrono_now() -> String {
    let output = std::process::Command::new("date")
        .args(["+%Y-%m-%dT%H:%M:%S"])
        .output();
    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        }
        _ => "unknown".to_string(),
    }
}
