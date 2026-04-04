// commands.rs — Data structures and utility functions for Negative _.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------
/// Disk usage statistics for the root volume.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DiskUsage {
    /// Total disk space in bytes
    pub total: u64,
    /// Used disk space in bytes
    pub used: u64,
    /// Free disk space in bytes
    pub free: u64,
    /// Usage as a percentage (0.0 – 100.0)
    pub percentage: f64,
}

/// Information about a single file found during a large-file scan.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileInfo {
    /// Absolute path to the file
    pub path: String,
    /// File name (last component of the path)
    pub name: String,
    /// Apparent (logical) size in bytes — what `ls -l` shows
    pub apparent_size: u64,
    /// Actual (physical) size on disk — blocks * 512
    pub actual_size: u64,
    /// Last-modified time as a human-readable string
    pub modified: String,
    /// True if the file is sparse (actual_size < 80% of apparent_size)
    pub is_sparse: bool,
}

/// One cache directory found under ~/Library/Caches or similar locations.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CacheEntry {
    /// Absolute path to the cache directory
    pub path: String,
    /// Directory name
    pub name: String,
    /// Total size in bytes (sum of all files inside)
    pub size: u64,
    /// Number of files inside the directory
    pub item_count: u64,
}

/// A single .log file found during a log scan.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEntry {
    /// Absolute path
    pub path: String,
    /// File name
    pub name: String,
    /// Size in bytes
    pub size: u64,
    /// Last-modified time as a human-readable string
    pub modified: String,
}

/// Docker installation info — images, disk usage, reclaimable space.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DockerInfo {
    /// Whether `docker` is installed (found on disk)
    pub installed: bool,
    /// Whether the Docker daemon is running (can execute commands)
    pub running: bool,
    /// List of Docker images / build-cache items
    pub images: Vec<DockerItem>,
    /// Human-readable total reclaimable space (from `docker system df`)
    pub total_reclaimable: String,
    /// Raw output of `docker system df`
    pub disk_usage_raw: String,
}

/// One Docker image or cache item.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DockerItem {
    pub name: String,
    pub size: String,
    pub id: String,
    /// e.g. "image", "container", "build-cache"
    pub item_type: String,
}

/// Information about an installed application.
///
/// `size` = the .app bundle itself.
/// `leftover_size` = sum of all detected Library leftovers.
/// `footprint` = size + leftover_size = total disk impact of this app.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppInfo {
    /// Display name (e.g. "Safari")
    pub name: String,
    /// Path to the .app bundle
    pub path: String,
    /// Size of the .app bundle in bytes
    pub size: u64,
    /// CFBundleIdentifier from Info.plist (empty string if unavailable)
    pub bundle_id: String,
    /// Leftover paths found in ~/Library/...
    pub leftover_paths: Vec<String>,
    /// Sum of leftover sizes in bytes
    pub leftover_size: u64,
    /// Total disk footprint: size + leftover_size
    pub footprint: u64,
    /// Base64-encoded PNG of the app's launcher icon (data:image/png;base64,...).
    /// Empty string if extraction failed.
    pub icon_base64: String,
    /// How the app was installed: "homebrew", "app-store", or "manual"
    pub install_source: String,
}

/// Summary of the Trash folder.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrashInfo {
    /// Total size of all items in Trash, in bytes
    pub size: u64,
    /// Number of items (files + directories at the top level)
    pub item_count: u64,
}

/// Result returned by any "clean" / "delete" operation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CleanResult {
    /// Whether the overall operation succeeded (may still have partial errors)
    pub success: bool,
    /// Total bytes freed
    pub freed_bytes: u64,
    /// Number of items deleted
    pub deleted_count: u64,
    /// Human-readable error messages for any items that failed
    pub errors: Vec<String>,
}

/// Result of a large file scan — includes both found files and skipped directories.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LargeFileScanResult {
    /// Files found matching the size criteria
    pub files: Vec<FileInfo>,
    /// Directories that were skipped due to permission errors
    pub skipped_paths: Vec<String>,
}

// ---------------------------------------------------------------------------
// Streaming large-file scan event payloads
// ---------------------------------------------------------------------------
/// Emitted each time a large file is discovered during a streaming scan.
/// The frontend appends it to the reactive list immediately so the user
/// sees files appear in real time.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LargeFileFound {
    pub file: FileInfo,
}

/// Emitted periodically to show which directory the scanner is currently
/// walking. Throttled to avoid flooding the event bus — we emit at most
/// once per directory entry at the top two levels, plus any time a file
/// is found.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LargeFileScanProgress {
    /// Human-readable path being scanned (with ~ substituted for home)
    pub current_dir: String,
    /// Number of files found so far
    pub files_found: usize,
}

/// Emitted once when the scan finishes (successfully or not).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LargeFileScanDone {
    /// Total number of files found
    pub total_files: usize,
    /// Directories that were skipped (e.g. TCC-protected without FDA)
    pub skipped_paths: Vec<String>,
}

/// Result of checking access to a specific path.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PathAccess {
    /// The original path as provided (e.g. "~/Desktop")
    pub path: String,
    /// The resolved absolute path (e.g. "/Users/conradfe/Desktop")
    pub resolved_path: String,
    /// Whether the path exists on disk
    pub exists: bool,
    /// Whether the app can read the directory contents
    pub readable: bool,
}

// ---------------------------------------------------------------------------
// Disk map cache metadata
// ---------------------------------------------------------------------------

/// Metadata about a cached disk map scan result.
/// Returned by `list_disk_map_caches` so the frontend can show a dropdown of
/// past scans without loading the full JSON payload.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CacheMetadata {
    /// Unique ID — the filename stem, e.g. "spacemap-2025-03-10T14:30:00"
    pub id: String,
    /// ISO 8601 timestamp string of when the scan was saved
    pub timestamp: String,
    /// How many seconds ago this cache was saved (computed at list time)
    pub age_seconds: u64,
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Run `du -sk <path>` and return the result in bytes.
///
/// Uses `du` subprocess to avoid triggering TCC prompts for protected directories.
/// Returns 0 if the path doesn't exist or can't be read.
pub fn get_du_size(path: &str) -> u64 {
    let output = std::process::Command::new("du")
        .args(["-sk", path])
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            // Output format: "12345\t/path/to/thing"
            text.split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0)
                * 1024 // du -sk gives kilobytes
        }
        _ => 0,
    }
}

/// Run a subprocess and return its stdout as a trimmed String.
/// Returns an empty string if the command fails.
pub fn run_cmd(program: &str, args: &[&str]) -> String {
    match std::process::Command::new(program).args(args).output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => String::new(),
    }
}

/// Run a subprocess and return whether it exited successfully.
/// This is useful for commands like `test -e` or `codesign -v`.
pub fn run_cmd_ok(program: &str, args: &[&str]) -> bool {
    std::process::Command::new(program)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Convert a byte count into a human-readable string (B, KB, MB, GB, TB).
///
/// # Examples
/// ```ignore
/// assert_eq!(format_size(0), "0 B");
/// assert_eq!(format_size(1024), "1.00 KB");
/// ```
#[allow(dead_code)] // Utility exposed for future use by other modules.
pub fn format_size(bytes: u64) -> String {
    let bytes_f = bytes as f64;

    // We define the unit thresholds as constants.
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    if bytes_f >= TB {
        format!("{:.2} TB", bytes_f / TB)
    } else if bytes_f >= GB {
        format!("{:.2} GB", bytes_f / GB)
    } else if bytes_f >= MB {
        format!("{:.2} MB", bytes_f / MB)
    } else if bytes_f >= KB {
        format!("{:.2} KB", bytes_f / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Get the current user's home directory as a `String`.
pub fn home_dir() -> Option<String> {
    std::env::var("HOME").ok()
}

// ---------------------------------------------------------------------------
// Shared scan configuration helpers
// ---------------------------------------------------------------------------

/// Resolved skip prefixes and scan roots for filesystem scanners.
#[allow(dead_code)]
pub struct ScanConfig {
    pub skip_prefixes: Vec<String>,
    pub scan_roots: Vec<String>,
}

/// System paths that every scanner should skip.
const SYSTEM_SKIP_PREFIXES: &[&str] = &[
    "/System",
    "/Library/Apple",
    "/private/var/db",
    "/private/var/folders",
];

/// Build skip prefixes from user-provided paths, resolving `~/` to the home
/// directory. The returned vec starts with [`SYSTEM_SKIP_PREFIXES`] and
/// appends any additional prefixes supplied via `extra_prefixes`.
pub fn build_skip_prefixes(
    home: &str,
    user_paths: &[String],
    extra_prefixes: &[String],
) -> Vec<String> {
    let mut prefixes: Vec<String> = SYSTEM_SKIP_PREFIXES
        .iter()
        .map(|s| s.to_string())
        .collect();

    for sp in user_paths {
        let resolved = if *sp == "~" {
            home.to_string()
        } else if sp.starts_with("~/") {
            format!("{}{}", home, &sp[1..])
        } else {
            sp.clone()
        };
        prefixes.push(resolved);
    }

    prefixes.extend(extra_prefixes.iter().cloned());
    prefixes
}

/// Build scan roots for a given path, FDA status, and domain-specific safe
/// directories.
///
/// If `has_fda` is true, resolves `path` (expanding `~` / `~/`) and returns it
/// as the sole root.  Otherwise, returns only those entries from `safe_dirs`
/// that correspond to existing directories on disk.
pub fn build_scan_roots(
    home: &str,
    path: &str,
    has_fda: bool,
    safe_dirs: &[String],
) -> Vec<String> {
    if has_fda {
        let start = if path.is_empty() || path == "~" {
            home.to_string()
        } else if path.starts_with("~/") {
            format!("{}{}", home, &path[1..])
        } else {
            path.to_string()
        };
        vec![start]
    } else {
        safe_dirs
            .iter()
            .filter(|d| std::path::Path::new(d.as_str()).is_dir())
            .cloned()
            .collect()
    }
}

/// Calculate the total size (in bytes) of a directory by recursively walking it.
/// Permission errors are silently skipped.
///
/// Returns `(total_bytes, file_count)`.
pub fn dir_size(path: &str) -> (u64, u64) {
    let mut total: u64 = 0;
    let mut count: u64 = 0;

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        // `filter_map(|e| e.ok())` silently skips entries that produced errors
        // (e.g. permission denied).
        .filter_map(|e| e.ok())
    {
        if let Ok(m) = entry.metadata() {
            if m.is_file() {
                total += m.len();
                count += 1;
            }
        }
    }

    (total, count)
}

/// Try to format a `SystemTime` as a human-readable "YYYY-MM-DD HH:MM:SS" string.
/// Falls back to "unknown" on error.
pub fn format_system_time(time: std::time::SystemTime) -> String {
    match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(dur) => {
            let secs = dur.as_secs();
            // We shell out to `date` for reliable local-time formatting.
            let output = std::process::Command::new("date")
                .args(["-r", &secs.to_string(), "+%Y-%m-%d %H:%M:%S"])
                .output();
            match output {
                Ok(o) if o.status.success() => {
                    String::from_utf8_lossy(&o.stdout).trim().to_string()
                }
                _ => {
                    // Fallback: just show the Unix timestamp
                    format!("{}", secs)
                }
            }
        }
        Err(_) => "unknown".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests (optional but good practice)
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1_048_576), "1.00 MB");
        assert_eq!(format_size(1_073_741_824), "1.00 GB");
    }

    #[test]
    fn test_build_skip_prefixes_resolves_tilde() {
        let prefixes = build_skip_prefixes(
            "/Users/test",
            &["~/Desktop".to_string(), "/tmp".to_string()],
            &[],
        );
        // Should contain the 4 system prefixes + 2 user paths
        assert_eq!(prefixes.len(), 6);
        assert!(prefixes.contains(&"/Users/test/Desktop".to_string()));
        assert!(prefixes.contains(&"/tmp".to_string()));
        assert!(prefixes.contains(&"/System".to_string()));
    }

    #[test]
    fn test_build_skip_prefixes_bare_tilde() {
        let prefixes = build_skip_prefixes(
            "/Users/test",
            &["~".to_string()],
            &[],
        );
        assert!(prefixes.contains(&"/Users/test".to_string()));
    }

    #[test]
    fn test_build_skip_prefixes_extra() {
        let prefixes = build_skip_prefixes(
            "/Users/test",
            &[],
            &["/extra/path".to_string()],
        );
        assert!(prefixes.contains(&"/extra/path".to_string()));
    }

    #[test]
    fn test_build_scan_roots_with_fda() {
        let roots = build_scan_roots("/Users/test", "~/Projects", true, &[]);
        assert_eq!(roots, vec!["/Users/test/Projects".to_string()]);
    }

    #[test]
    fn test_build_scan_roots_fda_empty_path() {
        let roots = build_scan_roots("/Users/test", "", true, &[]);
        assert_eq!(roots, vec!["/Users/test".to_string()]);
    }

    #[test]
    fn test_build_scan_roots_fda_tilde_path() {
        let roots = build_scan_roots("/Users/test", "~", true, &[]);
        assert_eq!(roots, vec!["/Users/test".to_string()]);
    }

    #[test]
    fn test_build_scan_roots_without_fda_filters_existing() {
        // /tmp should exist; /nonexistent_dir_xyz should not
        let safe = vec!["/tmp".to_string(), "/nonexistent_dir_xyz".to_string()];
        let roots = build_scan_roots("/Users/test", "", false, &safe);
        assert_eq!(roots, vec!["/tmp".to_string()]);
    }
}
