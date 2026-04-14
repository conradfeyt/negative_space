// lib.rs — Main Tauri application setup and command handlers for Negative _.

mod commands;

// The security module handles scanning for potential security issues:
// launch agents/daemons, app trust (code signing), and shell init file auditing.
mod security;

// The browser module handles detecting installed browsers and scanning their
// cache, cookies, history, and session data for cleanup.
mod browser;

// The maintenance module handles system maintenance tasks like flushing DNS,
// rebuilding Spotlight index, freeing purgeable space, etc.
mod maintenance;

// The duplicates module handles finding duplicate files using a 3-stage pipeline:
// size grouping -> partial hash (first 4KB) -> full BLAKE3 hash.
mod duplicates;

// The diskmap module builds a directory-size tree for the treemap visualization.
// Uses `du -sk` subprocess for TCC-safe sizing.
mod diskmap;

// Shared process name dictionaries used by both memory.rs and vitals.rs.
mod process_info;

// The memory module analyzes running processes, groups them by application/category,
// and provides human-readable descriptions for macOS system daemons.
mod memory;

// The preview module generates file previews (thumbnails, text excerpts) for the
// duplicate finder and other views that show file lists.
mod preview;

// The packages module detects installed package managers (Homebrew, pip, npm,
// cargo, etc.), their packages, and runtimes (Java, Node/nvm, Rust, Go, Flutter).
mod packages;

// The vitals module provides real-time system health monitoring: thermal state,
// CPU hogs, system load, and actionable remediation suggestions. Answers the
// question "why is my Mac hot and what can I do about it?"
mod vitals;

// The thermal module reads hardware temperature sensors and fan speeds directly
// from the Apple SMC (System Management Controller) via IOKit. Provides per-
// sensor readings (CPU cores, GPU clusters, SSD, battery, etc.) and fan RPM.
// No sudo required — read-only SMC access works for regular user processes.
mod thermal;

// The gradient module handles rendering the screen-anchored gradient background
// as a native NSImageView layer behind the WKWebView. This is positioned by
// the window compositor (not JS), so it moves in perfect sync during drag.
#[cfg(target_os = "macos")]
mod gradient;
mod vault;
mod intelligence;
mod spotlight;
mod large_files;
mod caches_logs;
mod docker;
mod apps;

// Shared image utilities (loading, HEIC conversion) for similar image detection
// and content classification.
mod image_utils;

// Perceptual hash-based similar image detection — finds visually similar images
// even when they differ in resolution, compression, or format.
mod similar_images;

// NSFW / sensitive content detection using CoreML + Vision framework.
// Scans images and classifies them with a bundled OpenNSFW2 model.
mod nsfw;

// Bring standard-library and crate items into scope.
use commands::{CleanResult, DiskUsage, TrashInfo};
use std::fs;
use std::path::Path;
use tauri::Emitter;

// ---------------------------------------------------------------------------
// Command 1: get_disk_usage
// ---------------------------------------------------------------------------

/// Query disk usage for the root volume (`/`) via `df -k`.
// NOTE ON ASYNC: All commands are `async` so Tauri runs them on a background
// thread pool instead of the main thread. This prevents the spinning beach
// ball — the main thread stays free to handle UI events while scans run.

#[tauri::command]
async fn get_disk_usage() -> Result<DiskUsage, String> {
    // Run `df -k /System/Volumes/Data` to get accurate APFS disk usage.
    // On macOS with APFS, `df /` shows the *system snapshot*, which reports
    // a misleadingly low "Used" value. The Data volume shows the real picture.
    let output = std::process::Command::new("df")
        .args(["-k", "/System/Volumes/Data"])
        .output()
        .map_err(|e| format!("Failed to run df: {}", e))?;
    if !output.status.success() {
        return Err("df command failed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // `df -k /` output looks like:
    // Filesystem 1024-blocks  Used Available Capacity ...
    // /dev/disk3s1s1 ...       ...  ...       ...
    let lines: Vec<&str> = stdout.lines().collect();
    if lines.len() < 2 {
        return Err("Unexpected df output".to_string());
    }

    let parts: Vec<&str> = lines[1].split_whitespace().collect();
    if parts.len() < 4 {
        return Err("Could not parse df output".to_string());
    }

    // Parse the 1K-block numbers and convert to bytes (* 1024).
    let total_kb: u64 = parts[1].parse().map_err(|_| "Parse error for total")?;
    let used_kb: u64 = parts[2].parse().map_err(|_| "Parse error for used")?;
    let free_kb: u64 = parts[3].parse().map_err(|_| "Parse error for free")?;

    let total = total_kb * 1024;
    let used = used_kb * 1024;
    let free = free_kb * 1024;

    // Avoid division by zero (shouldn't happen, but let's be safe).
    let percentage = if total > 0 {
        (used as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Ok(DiskUsage {
        total,
        used,
        free,
        percentage,
    })
}

/// Run `du -sk <path>` and return the result in bytes.
///
/// Uses `du` subprocess to avoid triggering TCC prompts for protected directories.
fn get_du_size(path: &str) -> u64 {
    crate::commands::get_du_size(path)
}

// ---------------------------------------------------------------------------
// Command 7: get_trash_info
// ---------------------------------------------------------------------------

/// Get the size and item count of ~/.Trash/.
///
/// IMPORTANT (TCC): The Trash can contain items that were moved from
/// TCC-protected directories (e.g. Desktop, Documents). Walking those
/// items may trigger permission dialogs. We use `du -sk` (a subprocess)
/// to safely get the size — `du` runs in its own process and permission
/// errors just result in lines to stderr, not blocking modal dialogs in
/// our app thread.
#[tauri::command]
async fn get_trash_info() -> Result<TrashInfo, String> {
    let home = commands::home_dir().ok_or("Cannot determine home directory")?;
    let trash_path = format!("{}/.Trash", home);

    // Check existence via subprocess (avoid any in-process filesystem calls).
    let exists = std::process::Command::new("test")
        .args(["-d", &trash_path])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !exists {
        return Ok(TrashInfo {
            size: 0,
            item_count: 0,
        });
    }

    // Count top-level items via ls (subprocess avoids TCC dialogs).
    // We use `ls -A` to include hidden files but exclude . and ..
    let item_count = match std::process::Command::new("ls")
        .args(["-A", &trash_path])
        .output()
    {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            text.lines().count() as u64
        }
        _ => 0,
    };

    // Get total size via `du -sk` (subprocess — won't trigger TCC dialogs).
    let size = get_du_size(&trash_path);

    Ok(TrashInfo { size, item_count })
}

// ---------------------------------------------------------------------------
// Command 8: delete_files
// ---------------------------------------------------------------------------

/// Delete a list of files and/or directories. Returns a summary of what happened.
#[tauri::command]
async fn delete_files(app: tauri::AppHandle, paths: Vec<String>) -> CleanResult {
    let mut freed_bytes: u64 = 0;
    let mut deleted_count: u64 = 0;
    let mut errors: Vec<String> = Vec::new();
    let total = paths.len() as u64;

    for (idx, path_str) in paths.iter().enumerate() {
        let path = Path::new(path_str);
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let _ = app.emit("operation-progress", vault::OperationProgress {
            operation: "delete".to_string(),
            processed: idx as u64,
            total,
            current_file: name,
        });

        let size = get_du_size(path_str);
        if size == 0 && !path.exists() {
            errors.push(format!("Path not found: {}", path_str));
            continue;
        }

        let result = if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };

        match result {
            Ok(()) => {
                freed_bytes += size;
                deleted_count += 1;
            }
            Err(e) => {
                errors.push(format!("Failed to delete {}: {}", path_str, e));
            }
        }
    }

    let _ = app.emit("operation-progress", vault::OperationProgress {
        operation: "delete".to_string(),
        processed: total,
        total,
        current_file: String::new(),
    });

    CleanResult {
        success: errors.is_empty(),
        freed_bytes,
        deleted_count,
        errors,
    }
}

// ---------------------------------------------------------------------------
// Command 10: empty_trash
// ---------------------------------------------------------------------------

/// Empty the user's Trash by deleting every item inside ~/.Trash/.
/// The .Trash directory itself is left in place.
///
/// IMPORTANT: We use `rm -rf` in a subprocess to avoid TCC issues. Items in
/// Trash may have come from TCC-protected directories, and in-process walkdir
/// or read_dir on those items can trigger permission dialogs.
#[tauri::command]
async fn empty_trash() -> CleanResult {
    let home = match commands::home_dir() {
        Some(h) => h,
        None => {
            return CleanResult {
                success: false,
                freed_bytes: 0,
                deleted_count: 0,
                errors: vec!["Cannot determine home directory".to_string()],
            };
        }
    };

    let trash_path = format!("{}/.Trash", home);

    // Check existence via subprocess.
    let exists = std::process::Command::new("test")
        .args(["-d", &trash_path])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !exists {
        return CleanResult {
            success: true,
            freed_bytes: 0,
            deleted_count: 0,
            errors: vec![],
        };
    }

    // Measure total size before deleting (subprocess — safe from TCC).
    let total_size = get_du_size(&trash_path);

    // Count items before delete (subprocess — safe from TCC).
    let item_count = match std::process::Command::new("ls")
        .args(["-A", &trash_path])
        .output()
    {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            text.lines().count() as u64
        }
        _ => 0,
    };

    // Delete all contents of ~/.Trash using a subprocess.
    // `find ~/.Trash -mindepth 1 -delete` removes everything inside without
    // removing .Trash itself, and runs as a subprocess so TCC dialogs don't
    // block our app thread.
    let result = std::process::Command::new("find")
        .args([&trash_path, "-mindepth", "1", "-delete"])
        .output();

    match result {
        Ok(o) if o.status.success() => CleanResult {
            success: true,
            freed_bytes: total_size,
            deleted_count: item_count,
            errors: vec![],
        },
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            // Partial success — some files may have been deleted.
            CleanResult {
                success: false,
                freed_bytes: total_size / 2, // estimate partial
                deleted_count: item_count / 2,
                errors: vec![format!("Some items could not be deleted: {}", stderr)],
            }
        }
        Err(e) => CleanResult {
            success: false,
            freed_bytes: 0,
            deleted_count: 0,
            errors: vec![format!("Failed to empty trash: {}", e)],
        },
    }
}

// ---------------------------------------------------------------------------
// Command 12: check_full_disk_access
// ---------------------------------------------------------------------------

/// Check whether the app has Full Disk Access by querying the TCC database.
///
/// IMPORTANT: We CANNOT use `ls`, `readdir`, or ANY directory listing on
/// TCC-protected paths — even from a subprocess, macOS attributes the access
/// to our app's bundle ID and shows a permission dialog.
///
/// Instead, we query the user's TCC database directly using `sqlite3`.
/// The TCC database tracks which apps have been granted access.
/// If our app (or "SystemPolicyAllFiles" service) has an entry, we have FDA.
///
/// Fallback: try `stat` on a file known to exist inside ~/Library/Safari/.
/// `stat` (metadata lookup) does NOT trigger TCC dialogs — only `readdir` does.
#[tauri::command]
async fn check_full_disk_access() -> bool {
    let home = match commands::home_dir() {
        Some(h) => h,
        None => return false,
    };

    // Method 1: Query the TCC database for our bundle ID.
    // The user-level TCC database is at ~/Library/Application Support/com.apple.TCC/TCC.db
    let tcc_db = format!(
        "{}/Library/Application Support/com.apple.TCC/TCC.db",
        home
    );
    let query = "SELECT allowed FROM access WHERE service='kTCCServiceSystemPolicyAllFiles' AND client='com.conradfe.negativespace' LIMIT 1";

    if let Ok(output) = std::process::Command::new("sqlite3")
        .args(["-separator", "", &tcc_db, query])
        .output()
    {
        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // "1" means granted, "0" means denied
            if result == "1" {
                return true;
            }
            if result == "0" {
                return false;
            }
        }
    }

    // Method 2: Fallback — try to read a tiny known file inside a TCC-protected
    // directory using `cat`. Unlike `ls` (which does readdir), `cat` on a specific
    // file path uses open() which may behave differently.
    // Actually, use `test -r` on a specific file — this checks read permission
    // without actually reading content.
    let safari_bookmarks = format!("{}/Library/Safari/Bookmarks.plist", home);
    match std::process::Command::new("test")
        .args(["-r", &safari_bookmarks])
        .status()
    {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Command 13: check_path_access
// ---------------------------------------------------------------------------

/// Check whether the app can read a specific directory path.
/// Returns a struct with the path, whether it exists, and whether it's readable.
///
/// IMPORTANT: We CANNOT use `ls` or `readdir` on TCC-protected directories —
/// even from a subprocess, macOS shows a permission dialog attributed to our app.
///
/// For TCC-protected user directories (Desktop, Documents, Downloads, Pictures,
/// Movies, Music), we check if the current FDA status would allow access.
/// For non-protected paths, `test -r` is safe to use.
#[tauri::command]
async fn check_path_access(path: String) -> commands::PathAccess {
    // Expand tilde
    let resolved = if path == "~" {
        commands::home_dir().unwrap_or_default()
    } else if path.starts_with("~/") {
        let home = commands::home_dir().unwrap_or_default();
        format!("{}{}", home, &path[1..])
    } else {
        path.clone()
    };

    let home = commands::home_dir().unwrap_or_default();

    // Known TCC-protected directories — we MUST NOT probe these with ls/readdir.
    // Instead, we infer readability from whether we have FDA.
    let tcc_protected_prefixes = vec![
        format!("{}/Desktop", home),
        format!("{}/Documents", home),
        format!("{}/Downloads", home),
        format!("{}/Movies", home),
        format!("{}/Music", home),
        format!("{}/Pictures", home),
        format!("{}/Library/Mail", home),
        format!("{}/Library/Messages", home),
        format!("{}/Library/Safari", home),
        format!("{}/Library/Caches", home),
        format!("{}/Library/Containers", home),
        format!("{}/Library/Mobile Documents", home),
    ];

    let is_tcc_protected = tcc_protected_prefixes
        .iter()
        .any(|prefix| resolved.starts_with(prefix.as_str()));

    if is_tcc_protected {
        // For TCC-protected paths: assume they exist (standard macOS dirs).
        // Readability depends on FDA status — check without triggering TCC.
        let has_fda = check_fda_via_tcc_db(&home);

        return commands::PathAccess {
            path,
            resolved_path: resolved,
            exists: true,
            readable: has_fda,
        };
    }

    // For non-TCC paths, `test -e` and `test -r` are safe.
    let exists = match std::process::Command::new("test")
        .args(["-e", &resolved])
        .status()
    {
        Ok(status) => status.success(),
        Err(_) => false,
    };

    if !exists {
        return commands::PathAccess {
            path,
            resolved_path: resolved,
            exists: false,
            readable: false,
        };
    }

    let readable = match std::process::Command::new("test")
        .args(["-r", &resolved])
        .status()
    {
        Ok(status) => status.success(),
        Err(_) => false,
    };

    commands::PathAccess {
        path,
        resolved_path: resolved,
        exists: true,
        readable,
    }
}

/// Helper: check FDA status by querying the TCC database (no TCC dialogs).
fn check_fda_via_tcc_db(home: &str) -> bool {
    let tcc_db = format!(
        "{}/Library/Application Support/com.apple.TCC/TCC.db",
        home
    );
    let query = "SELECT allowed FROM access WHERE service='kTCCServiceSystemPolicyAllFiles' AND client='com.conradfe.negativespace' LIMIT 1";

    if let Ok(output) = std::process::Command::new("sqlite3")
        .args(["-separator", "", &tcc_db, query])
        .output()
    {
        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return result == "1";
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Command 15: request_path_access
// ---------------------------------------------------------------------------

/// Deliberately trigger a macOS TCC permission prompt for a specific directory.
///
/// This is the ONE place where we INTENTIONALLY use an in-process filesystem
/// call on a TCC-protected path. When the user explicitly toggles on a
/// directory in Settings, we want macOS to show the "would like to access"
/// dialog so the user can grant per-directory access.
///
/// Returns true if access was granted, false if denied or error.
#[tauri::command]
async fn request_path_access(path: String) -> bool {
    let resolved = if path == "~" {
        commands::home_dir().unwrap_or_default()
    } else if path.starts_with("~/") {
        let home = commands::home_dir().unwrap_or_default();
        format!("{}{}", home, &path[1..])
    } else {
        path
    };

    // Intentionally call fs::read_dir — this triggers the TCC prompt.
    // If the user clicks "Allow", subsequent calls will succeed.
    // If they click "Don't Allow", it fails and we return false.
    match fs::read_dir(&resolved) {
        Ok(_) => true,
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Command 16: open_full_disk_access_settings
// ---------------------------------------------------------------------------

/// Open System Settings directly to the Full Disk Access pane.
#[tauri::command]
async fn open_full_disk_access_settings() -> Result<(), String> {
    // On macOS Ventura+ (13+), System Settings uses this URL scheme.
    // On older macOS, it falls back to System Preferences.
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
        .spawn()
        .map_err(|e| format!("Failed to open System Settings: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Command: reveal_in_finder
// ---------------------------------------------------------------------------

/// Reveal a file or directory in Finder, selecting it in the enclosing folder.
///
/// Uses `open -R <path>` which is the macOS equivalent of
/// `NSWorkspace.activateFileViewerSelectingURLs` — it opens Finder and
/// highlights the target file/folder.
#[tauri::command]
async fn reveal_in_finder(path: String) -> Result<(), String> {
    std::process::Command::new("open")
        .args(["-R", &path])
        .spawn()
        .map_err(|e| format!("Failed to reveal in Finder: {}", e))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Command 17: scan_security
// ---------------------------------------------------------------------------

/// Run a full security scan: launch items, app trust, and shell init files.
///
/// This combines all three security sub-scans into a single result. The frontend
/// can call this once and display all findings in a unified security dashboard.
///
/// NOTE ON ASYNC: This is `async` so Tauri runs it on a background thread.
/// Security scans invoke many subprocesses (codesign, PlistBuddy, etc.) and
/// can take several seconds — we MUST NOT block the main thread.
#[tauri::command]
async fn scan_security() -> Result<security::SecurityScanResult, String> {
    // The heavy lifting is in security::run_full_scan(). We wrap the result
    // in Ok() because Tauri commands return Result<T, String>.
    Ok(security::run_full_scan())
}

// ---------------------------------------------------------------------------
// Command 18: disable_launch_item
// ---------------------------------------------------------------------------

/// Disable a launch agent/daemon by unloading it (without deleting the file).
///
/// The frontend sends the plist file path, and we use `launchctl unload -w`
/// to stop the agent and prevent it from auto-starting on next login.
#[tauri::command]
async fn disable_launch_item(path: String) -> CleanResult {
    match security::disable_launch_item(&path) {
        Ok(()) => CleanResult {
            success: true,
            freed_bytes: 0,
            deleted_count: 0,
            errors: vec![],
        },
        Err(e) => CleanResult {
            success: false,
            freed_bytes: 0,
            deleted_count: 0,
            errors: vec![e],
        },
    }
}

// ---------------------------------------------------------------------------
// Command 19: remove_launch_item
// ---------------------------------------------------------------------------

/// Permanently remove a launch agent/daemon (unload + delete the plist file).
///
/// WARNING: This is destructive — the plist file is deleted from disk. The
/// frontend should confirm with the user before calling this command.
#[tauri::command]
async fn remove_launch_item(path: String) -> CleanResult {
    // Get the file size before removal so we can report freed bytes.
    let freed = get_du_size(&path);

    match security::remove_launch_item(&path) {
        Ok(()) => CleanResult {
            success: true,
            freed_bytes: freed,
            deleted_count: 1,
            errors: vec![],
        },
        Err(e) => CleanResult {
            success: false,
            freed_bytes: 0,
            deleted_count: 0,
            errors: vec![e],
        },
    }
}

// ---------------------------------------------------------------------------
// Command 20: scan_browsers
// ---------------------------------------------------------------------------

/// Scan all installed browsers for cleanable data (cache, cookies, history, etc.)
///
/// Detects Safari, Chrome, Firefox, Brave, Edge, Arc, Opera, and Vivaldi.
/// TCC-aware: Safari data is TCC-protected and can only be sized/cleaned
/// with Full Disk Access. Other browsers' data is not TCC-protected.
///
/// NOTE ON ASYNC: This spawns many `du -sk` subprocesses (one per data path
/// per browser) so it can take a few seconds. Running async keeps the UI responsive.
#[tauri::command]
async fn scan_browsers(has_fda: Option<bool>) -> Result<browser::BrowserScanResult, String> {
    let fda = has_fda.unwrap_or(false);
    Ok(browser::run_browser_scan(fda))
}

// ---------------------------------------------------------------------------
// Command 21: clean_browser_data
// ---------------------------------------------------------------------------

/// Clean specific browser data paths.
///
/// Takes a list of absolute filesystem paths to delete. The frontend is
/// responsible for confirming with the user before calling this, especially
/// for destructive categories like cookies and history.
///
/// Uses `rm -rf` subprocess internally for TCC safety.
#[tauri::command]
async fn clean_browser_data(paths: Vec<String>) -> browser::BrowserCleanResult {
    browser::clean_browser_data(paths)
}

// ---------------------------------------------------------------------------
// Command 22: scan_duplicates
// ---------------------------------------------------------------------------

/// Find duplicate files using a 3-stage pipeline (size -> partial hash -> full hash).
///
/// TCC-aware: without FDA, only scans known-safe directories. With FDA, scans
/// from the user's home directory (or specified path).
///
/// NOTE ON ASYNC: Duplicate scanning is the most I/O-intensive operation in
/// Negative _ — it reads the first 4KB of every same-size file, then fully hashes
/// confirmed candidates. On a large home directory this can take 30-60+ seconds.
/// Running async is critical to avoid blocking the UI.
#[tauri::command]
async fn scan_duplicates(
    path: String,
    min_size_mb: u64,
    has_fda: Option<bool>,
    skip_paths: Option<Vec<String>>,
    scan_paths: Option<Vec<String>>,
) -> Result<duplicates::DuplicateScanResult, String> {
    let fda = has_fda.unwrap_or(false);
    let skips = skip_paths.unwrap_or_default();
    let explicit_roots = scan_paths.unwrap_or_default();
    Ok(duplicates::run_duplicate_scan(duplicates::DuplicateScanOptions {
        scan_path: &path,
        min_size_mb,
        fda,
        skip_paths: &skips,
        scan_paths: &explicit_roots,
    }))
}

// ---------------------------------------------------------------------------
// Command: generate_thumbnails_batch
// ---------------------------------------------------------------------------

/// Generate small JPEG thumbnails for multiple files in one call.
/// Takes a list of (key, path) pairs. Returns a map of key -> base64 JPEG.
/// Keys are typically group hashes. Uses threads for concurrent generation.
#[tauri::command]
async fn generate_thumbnails_batch(
    items: Vec<(String, String)>,
    max_dim: Option<u32>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let dim = max_dim.unwrap_or(140);
    // Spawn each thumbnail on a thread to avoid sequential blocking
    let handles: Vec<_> = items.into_iter().map(|(key, path)| {
        std::thread::spawn(move || {
            match image_utils::generate_thumbnail(std::path::Path::new(&path), dim) {
                Ok(b64) => Some((key, b64)),
                Err(_) => None,
            }
        })
    }).collect();

    let mut results = std::collections::HashMap::new();
    for handle in handles {
        if let Ok(Some((key, b64))) = handle.join() {
            results.insert(key, b64);
        }
    }
    Ok(results)
}

// ---------------------------------------------------------------------------
// Command: scan_similar_images
// ---------------------------------------------------------------------------

/// Scan for visually similar images using perceptual hashing.
///
/// Unlike `scan_duplicates` which finds byte-identical files, this finds images
/// that look similar even if they differ in resolution, compression, or format.
/// Uses dHash (gradient) perceptual hashing with configurable Hamming distance
/// threshold.
#[tauri::command]
async fn scan_similar_images(
    app: tauri::AppHandle,
    threshold: Option<u32>,
    min_size_mb: u64,
    has_fda: Option<bool>,
    skip_paths: Option<Vec<String>>,
    scan_paths: Option<Vec<String>>,
) -> Result<similar_images::SimilarScanResult, String> {
    let fda = has_fda.unwrap_or(false);
    let skips = skip_paths.unwrap_or_default();
    let explicit_roots = scan_paths.unwrap_or_default();
    let min_bytes = if min_size_mb > 0 { min_size_mb * 1024 * 1024 } else { 10 * 1024 }; // default 10KB min
    Ok(similar_images::run_similar_scan(&app, similar_images::SimilarScanOptions {
        threshold,
        min_size_bytes: min_bytes,
        fda,
        skip_paths: &skips,
        scan_paths: &explicit_roots,
    }))
}

// ---------------------------------------------------------------------------
// Command 23: get_disk_map
// ---------------------------------------------------------------------------

/// Build a disk usage tree for the space visualization treemap.
///
/// Returns a tree of directory nodes with sizes, suitable for rendering as a
/// treemap. Each node has a category for color coding (developer, media,
/// documents, applications, system, caches, docker, other).
///
/// NOTE ON ASYNC: This spawns many `du -sk` subprocesses. Depending on the
/// number of directories and FDA status, this can take 10-30 seconds.
#[tauri::command]
async fn get_disk_map(has_fda: Option<bool>) -> Result<diskmap::DiskMapResult, String> {
    let fda = has_fda.unwrap_or(false);
    // Depth 5 = top-level dirs + 4 levels of children for large dirs.
    // Gives the sunburst 5-6 visible rings of real data.
    Ok(diskmap::build_disk_map(fda, 5))
}

// ---------------------------------------------------------------------------
// Command 24: expand_disk_node
// ---------------------------------------------------------------------------

/// Expand a specific directory in the disk map, returning its sized children.
///
/// Called when the user clicks on a directory in the treemap to drill down.
#[tauri::command]
async fn expand_disk_node(
    path: String,
    has_fda: Option<bool>,
) -> Result<diskmap::DiskNode, String> {
    let fda = has_fda.unwrap_or(false);
    Ok(diskmap::expand_directory(&path, fda))
}

// ---------------------------------------------------------------------------
// Command 25: enrich_disk_nodes
// ---------------------------------------------------------------------------

/// Get the last-modified timestamp for a batch of file/directory paths.
///
/// Used by the frontend's "Recency" overlay on the sunburst visualization.
/// The sunburst renders immediately from scan data (size mode), then the
/// frontend calls this in batches to asynchronously populate modification
/// times. Colors fade in as data arrives.
///
/// Uses `stat -f %m` (subprocess) for each path — TCC-safe, returns the
/// modification time as a Unix timestamp.
///
/// Returns a map of path → Unix timestamp (seconds since epoch).
/// Paths that fail (permission denied, not found) are silently omitted.
#[tauri::command]
async fn enrich_disk_nodes(paths: Vec<String>) -> Result<std::collections::HashMap<String, u64>, String> {
    let mut result = std::collections::HashMap::new();

    for path in &paths {
        // `stat -f %m <path>` returns the modification time as a Unix timestamp.
        // Use subprocess stat (not std::fs::metadata) to stay TCC-safe.
        let output = std::process::Command::new("stat")
            .args(["-f", "%m", path.as_str()])
            .output();

        if let Ok(o) = output {
            if o.status.success() {
                let text = String::from_utf8_lossy(&o.stdout);
                if let Ok(ts) = text.trim().parse::<u64>() {
                    result.insert(path.clone(), ts);
                }
            }
        }
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Command 26: save_disk_map_cache
// ---------------------------------------------------------------------------

/// Save a disk map scan result to the app's cache directory.
///
/// The frontend passes the serialized JSON string of the DiskMapResult.
/// We write it to `~/Library/Application Support/NegativeSpace/cache/spacemap-{ISO8601}.json`.
/// Auto-purges oldest files if more than 5 caches exist.
///
/// Returns the cache ID (filename stem) so the frontend can reference it.
#[tauri::command]
async fn save_disk_map_cache(data: String) -> Result<String, String> {
    let cache_dir = get_cache_dir()?;

    // Generate ISO 8601-ish timestamp for the filename.
    // We use `date` subprocess to avoid pulling in chrono.
    let timestamp = std::process::Command::new("date")
        .args(["+%Y-%m-%dT%H:%M:%S"])
        .output()
        .map_err(|e| format!("Failed to get timestamp: {}", e))?;

    let ts_str = String::from_utf8_lossy(&timestamp.stdout).trim().to_string();
    let id = format!("spacemap-{}", ts_str);
    let file_path = format!("{}/{}.json", cache_dir, id);

    // Write the JSON data to disk.
    std::fs::write(&file_path, &data)
        .map_err(|e| format!("Failed to write cache file: {}", e))?;

    // Auto-purge: keep only the 5 most recent cache files.
    purge_old_caches(&cache_dir, 5);

    Ok(id)
}

// ---------------------------------------------------------------------------
// Command 27: list_disk_map_caches
// ---------------------------------------------------------------------------

/// List all saved disk map cache files with metadata.
///
/// Returns a Vec of CacheMetadata sorted by timestamp descending (newest first).
/// Each entry has the ID, timestamp, and age in seconds.
#[tauri::command]
async fn list_disk_map_caches() -> Result<Vec<commands::CacheMetadata>, String> {
    let cache_dir = get_cache_dir()?;
    let mut entries: Vec<commands::CacheMetadata> = Vec::new();

    let dir = std::fs::read_dir(&cache_dir)
        .map_err(|e| format!("Failed to read cache directory: {}", e))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    for entry in dir.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("spacemap-") || !name.ends_with(".json") {
            continue;
        }

        // Extract the timestamp portion from the filename.
        // Format: "spacemap-2025-03-10T14:30:00.json"
        let id = name.trim_end_matches(".json").to_string();
        let timestamp = id.trim_start_matches("spacemap-").to_string();

        // Calculate age from the file's modification time.
        let age_seconds = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| now.saturating_sub(d.as_secs()))
            .unwrap_or(0);

        entries.push(commands::CacheMetadata {
            id,
            timestamp,
            age_seconds,
        });
    }

    // Sort newest first.
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Command 28: load_disk_map_cache
// ---------------------------------------------------------------------------

/// Load a specific cached disk map scan by its ID.
///
/// Returns the raw JSON string. The frontend deserializes it into DiskMapResult.
#[tauri::command]
async fn load_disk_map_cache(id: String) -> Result<String, String> {
    let cache_dir = get_cache_dir()?;
    let file_path = format!("{}/{}.json", cache_dir, id);

    std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read cache file '{}': {}", id, e))
}

// ---------------------------------------------------------------------------
// Command 29: delete_disk_map_cache
// ---------------------------------------------------------------------------

/// Delete a specific cached disk map scan by its ID.
#[tauri::command]
async fn delete_disk_map_cache(id: String) -> Result<(), String> {
    let cache_dir = get_cache_dir()?;
    let file_path = format!("{}/{}.json", cache_dir, id);

    std::fs::remove_file(&file_path)
        .map_err(|e| format!("Failed to delete cache file '{}': {}", id, e))
}

// ---------------------------------------------------------------------------
// Command 30: export_disk_map
// ---------------------------------------------------------------------------

/// Export a disk map scan result to a user-chosen JSON file path.
///
/// The frontend uses a save dialog to get the path, then calls this command.
#[tauri::command]
async fn export_disk_map(data: String, path: String) -> Result<(), String> {
    std::fs::write(&path, &data)
        .map_err(|e| format!("Failed to export disk map: {}", e))
}

// ---------------------------------------------------------------------------
// Command 31: import_disk_map
// ---------------------------------------------------------------------------

/// Import a disk map scan result from a user-chosen JSON file path.
///
/// Returns the raw JSON string. The frontend deserializes it into DiskMapResult.
#[tauri::command]
async fn import_disk_map(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to import disk map: {}", e))
}

// ---------------------------------------------------------------------------
// Cache helpers (not commands — internal utility functions)
// ---------------------------------------------------------------------------

/// Get (and ensure existence of) the Negative _ cache directory.
///
/// Path: `~/Library/Application Support/NegativeSpace/cache/`
///
fn get_cache_dir() -> Result<String, String> {
    let home = commands::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?;
    let cache_dir = format!("{}/Library/Application Support/NegativeSpace/cache", home);

    // Ensure the directory tree exists.
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    Ok(cache_dir)
}

/// Purge old cache files, keeping only the `max_keep` most recent.
///
/// Sorts files by modification time and removes the oldest ones.
fn purge_old_caches(cache_dir: &str, max_keep: usize) {
    // Collect all spacemap-*.json files with their modification times.
    let dir = match std::fs::read_dir(cache_dir) {
        Ok(d) => d,
        Err(_) => return,
    };

    let mut files: Vec<(std::path::PathBuf, std::time::SystemTime)> = dir
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("spacemap-") && name.ends_with(".json")
        })
        .filter_map(|e| {
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((e.path(), mtime))
        })
        .collect();

    if files.len() <= max_keep {
        return;
    }

    // Sort newest first (we want to keep the newest).
    files.sort_by(|a, b| b.1.cmp(&a.1));

    // Remove everything past max_keep.
    for (path, _) in files.iter().skip(max_keep) {
        let _ = std::fs::remove_file(path);
    }
}

// ---------------------------------------------------------------------------
// Command 32: get_maintenance_tasks
// (renumbered after adding disk map cache commands)
// ---------------------------------------------------------------------------

/// Get the list of available system maintenance tasks.
///
/// Returns a list of tasks with their descriptions, admin requirements, and
/// current status (always "idle" initially). The frontend displays these and
/// lets the user choose which to run.
#[tauri::command]
async fn get_maintenance_tasks() -> maintenance::MaintenanceTaskList {
    maintenance::get_tasks()
}

// ---------------------------------------------------------------------------
// Command 23: run_maintenance_task
// ---------------------------------------------------------------------------

/// Execute a single system maintenance task by its ID.
///
/// Some tasks require administrator privileges and will trigger macOS's native
/// password dialog via osascript. If the user cancels the auth dialog, the
/// task returns a failure with "Authentication cancelled by user".
///
/// NOTE ON ASYNC: Some tasks (especially free_purgeable) can take 30-60 seconds.
/// Running async keeps the UI responsive.
#[tauri::command]
async fn run_maintenance_task(task_id: String) -> maintenance::MaintenanceResult {
    maintenance::run_task(&task_id)
}

// ---------------------------------------------------------------------------
// Command 26: scan_memory
// ---------------------------------------------------------------------------

/// Scan running processes and return memory usage analysis.
///
/// Groups processes by application (e.g. all Chrome helpers -> "Google Chrome")
/// and by system category (Spotlight, Networking, Security, etc.). Provides
/// human-readable descriptions for ~150 macOS system daemons.
///
/// Also returns system-wide memory stats from `vm_stat` and `sysctl hw.memsize`.
///
/// NOTE ON ASYNC: Process scanning is fast (<1s typically) since `ps` is a
/// snapshot, but we still run async to avoid any main-thread blocking.
#[tauri::command]
async fn scan_memory() -> Result<memory::MemoryScanResult, String> {
    Ok(memory::scan_memory())
}

// ---------------------------------------------------------------------------
// Command 27: preview_file
// ---------------------------------------------------------------------------

/// Generate a preview for a file — image thumbnail, text excerpt, or metadata.
///
/// Uses macOS Quick Look (`qlmanage -t`) for image/PDF thumbnails, which handles
/// every format Finder can preview. Text files get the first ~100 lines.
/// Everything else returns metadata (file type, size, extension).
///
/// The frontend calls this when the user clicks a file in the duplicate finder
/// to see what they're looking at before deciding which copy to keep.
///
/// NOTE ON ASYNC: Quick Look thumbnail generation takes ~50-200ms per file.
/// Running async prevents the UI from freezing during preview generation.
#[tauri::command]
async fn preview_file(path: String, max_size: Option<u32>) -> Result<preview::FilePreview, String> {
    let size = max_size.unwrap_or(256);
    Ok(preview::generate_preview(&path, size))
}

// ---------------------------------------------------------------------------
// Command 27: scan_packages
// ---------------------------------------------------------------------------

/// Detect all installed package managers, their packages, and runtimes.
///
/// This scans for Homebrew, pip, npm, cargo, and runtimes like Java, Node (nvm),
/// Rust (rustup), Go, and Flutter. Returns a comprehensive inventory with sizes,
/// dependency info, and removal instructions.
///
/// NOTE: This can take several seconds because it runs multiple subprocess calls
/// (brew list, pip3 list, etc.). Must be async to avoid blocking the UI.
#[tauri::command]
async fn scan_packages() -> Result<packages::PackageScanResult, String> {
    Ok(packages::scan_all())
}

#[tauri::command]
async fn get_custom_probes() -> Result<Vec<packages::CustomProbe>, String> {
    packages::load_custom_probes()
}

#[tauri::command]
async fn save_custom_probes(probes: Vec<packages::CustomProbe>) -> Result<(), String> {
    packages::save_custom_probes(&probes)
}

#[tauri::command]
async fn delete_custom_probe(id: String) -> Result<(), String> {
    packages::delete_custom_probe(&id)
}

#[tauri::command]
async fn test_probe_command(program: String, args: Vec<String>) -> packages::CommandRecord {
    packages::test_probe_command(&program, &args)
}

// ---------------------------------------------------------------------------
// Command 29: scan_vitals
// ---------------------------------------------------------------------------

/// Scan system vitals: thermal state, CPU hogs, system load, and generate
/// actionable remediation suggestions. This is the backend for the System
/// Vitals view — it answers "why is my Mac hot and what can I do about it?"
///
/// Data sources: `ps -eo pid,ppid,%cpu,rss,comm`, `sysctl` for load/cores,
/// `pmset -g therm` for thermal state. All userspace, no sudo needed.
#[tauri::command]
async fn scan_vitals() -> Result<vitals::VitalsResult, String> {
    Ok(vitals::scan_vitals())
}

// ---------------------------------------------------------------------------
// Command 30: quit_process
// ---------------------------------------------------------------------------

/// Send SIGTERM to a process. Used by the System Vitals view to let users
/// quit CPU hogs directly from Negative _.
#[tauri::command]
async fn quit_process(pid: u32) -> Result<String, String> {
    vitals::quit_process(pid)
}

// ---------------------------------------------------------------------------
// Command 31: force_quit_process
// ---------------------------------------------------------------------------

/// Send SIGKILL to a process. Last resort when SIGTERM doesn't work.
#[tauri::command]
async fn force_quit_process(pid: u32) -> Result<String, String> {
    vitals::force_quit_process(pid)
}

// ---------------------------------------------------------------------------
// Command 32: quit_process_group
// ---------------------------------------------------------------------------

/// Quit all processes in a group by sending SIGTERM to each PID.
/// Returns a message with success/failure counts.
#[tauri::command]
async fn quit_process_group(pids: Vec<u32>) -> Result<String, String> {
    let (ok, fail) = vitals::quit_group(pids);
    if fail == 0 {
        Ok(format!("Quit {} process{}", ok, if ok > 1 { "es" } else { "" }))
    } else {
        Ok(format!("Quit {} process{}, {} failed", ok, if ok > 1 { "es" } else { "" }, fail))
    }
}

// ---------------------------------------------------------------------------
// Command 33: scan_thermal
// ---------------------------------------------------------------------------

/// Read all hardware temperature sensors and fan speeds from the Apple SMC.
/// Returns per-sensor readings (CPU cores, GPU clusters, SSD, battery, etc.),
/// category summaries, fan RPM, and the hottest sensor. No sudo required —
/// uses IOKit read-only SMC access via the `smc` crate.
#[tauri::command]
async fn scan_thermal() -> Result<thermal::ThermalScanResult, String> {
    thermal::scan_thermal()
}

// ---------------------------------------------------------------------------
// Command 34-35: Generic scan result cache (persist between sessions)
// ---------------------------------------------------------------------------

/// Save a domain's scan results to disk.
/// `domain` is a key like "caches", "logs", "large-files", etc.
/// `data` is the JSON-serialized scan result.
#[tauri::command]
async fn save_scan_cache(domain: String, data: String) -> Result<(), String> {
    let cache_dir = get_cache_dir()?;
    let file_path = format!("{}/scan-{}.json", cache_dir, domain);
    std::fs::write(&file_path, &data)
        .map_err(|e| format!("Failed to write cache for {}: {}", domain, e))?;
    Ok(())
}

/// Load a domain's cached scan results from disk.
/// Returns the JSON string, or null if no cache exists.
#[tauri::command]
async fn load_scan_cache(domain: String) -> Option<String> {
    let cache_dir = get_cache_dir().ok()?;
    let file_path = format!("{}/scan-{}.json", cache_dir, domain);
    std::fs::read_to_string(&file_path).ok()
}

// ---------------------------------------------------------------------------
// Command 36-41: Archive (compress-to-save-space)
// ---------------------------------------------------------------------------

#[tauri::command]
async fn scan_archive_candidates(
    path: String,
    min_size_mb: u64,
    min_age_days: u64,
    fda: bool,
) -> Vec<vault::CompressionCandidate> {
    vault::scan_candidates(vault::VaultScanOptions {
        scan_path: &path,
        min_size_mb,
        min_age_days,
        fda,
    })
}

#[tauri::command]
async fn compress_to_archive(app: tauri::AppHandle, paths: Vec<String>) -> vault::CompressResult {
    vault::compress_files(&paths, vault::StorageKind::Archive, |p| { let _ = app.emit("operation-progress", p); })
}

#[tauri::command]
async fn restore_from_archive(entry_id: String) -> vault::RestoreResult {
    vault::restore_file(&entry_id, vault::StorageKind::Archive)
}

#[tauri::command]
async fn get_archive_summary() -> vault::VaultSummary {
    vault::get_summary(vault::StorageKind::Archive)
}

#[tauri::command]
async fn get_archive_entries() -> Vec<vault::VaultEntry> {
    vault::get_entries(vault::StorageKind::Archive)
}

#[tauri::command]
async fn delete_archive_entry(entry_id: String) -> Result<(), String> {
    vault::delete_entry(&entry_id, vault::StorageKind::Archive)
}

#[tauri::command]
async fn collect_archive_directory(path: String) -> Vec<vault::CompressionCandidate> {
    vault::collect_directory_files(&path)
}

#[tauri::command]
async fn get_directory_size(path: String) -> u64 {
    commands::dir_size(&path).0
}

#[tauri::command]
async fn compress_directory_to_archive(path: String) -> vault::CompressResult {
    vault::compress_directory(&path, vault::StorageKind::Archive)
}

// ---------------------------------------------------------------------------
// Command 42-47: Vault (sensitive content secure storage)
// ---------------------------------------------------------------------------

#[tauri::command]
async fn compress_to_vault(app: tauri::AppHandle, paths: Vec<String>) -> vault::CompressResult {
    vault::compress_files(&paths, vault::StorageKind::Vault, |p| { let _ = app.emit("operation-progress", p); })
}

#[tauri::command]
async fn restore_from_vault(entry_id: String) -> vault::RestoreResult {
    vault::restore_file(&entry_id, vault::StorageKind::Vault)
}

#[tauri::command]
async fn get_vault_summary() -> vault::VaultSummary {
    vault::get_summary(vault::StorageKind::Vault)
}

#[tauri::command]
async fn get_vault_entries() -> Vec<vault::VaultEntry> {
    vault::get_entries(vault::StorageKind::Vault)
}

#[tauri::command]
async fn delete_vault_entry(entry_id: String) -> Result<(), String> {
    vault::delete_entry(&entry_id, vault::StorageKind::Vault)
}

#[tauri::command]
async fn move_files_to_directory(app: tauri::AppHandle, paths: Vec<String>, target_dir: String) -> vault::MoveResult {
    vault::move_files_to_directory(&paths, &target_dir, |p| { let _ = app.emit("operation-progress", p); })
}

#[tauri::command]
async fn get_storage_config() -> vault::StorageConfig {
    vault::load_storage_config()
}

#[tauri::command]
async fn set_storage_config(config: vault::StorageConfig) -> Result<(), String> {
    vault::save_storage_config(&config)
}

// ---------------------------------------------------------------------------
// Command 42-44: Apple Intelligence
// ---------------------------------------------------------------------------

#[tauri::command]
async fn check_intelligence_available() -> bool {
    intelligence::is_available()
}

#[tauri::command]
async fn check_ai_available() -> bool {
    intelligence::is_ai_available()
}

#[tauri::command]
async fn classify_files_ai(files: Vec<intelligence::FileClassificationInput>) -> Vec<intelligence::FileClassification> {
    intelligence::classify_files(&files)
}

#[tauri::command]
async fn generate_scan_summary_ai(input: intelligence::ScanSummaryInput) -> intelligence::ScanSummaryOutput {
    intelligence::generate_scan_summary(&input)
}

#[tauri::command]
async fn render_sf_symbol(name: String, size: u32, mode: Option<String>, style: Option<String>, glyph_scale: Option<f64>) -> String {
    intelligence::render_sf_symbol(&name, size, mode.as_deref(), style.as_deref(), glyph_scale)
}

#[tauri::command]
async fn list_system_images() -> Vec<String> {
    intelligence::list_system_images()
}

// ---------------------------------------------------------------------------
// Tauri app entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // The opener plugin lets the frontend open URLs / files with the OS.
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            use tauri::Manager;

            vault::migrate_legacy_vault();

            let window = app
                .get_webview_window("main")
                .expect("main window must exist per tauri.conf.json");


            // The sidebar gradient is a standalone background NSWindow that
            // never moves (see gradient.rs). We manage its visibility based
            // on the main window's focus state — show when Negative _ is
            // frontmost, hide when it's not.
            window.on_window_event(move |event| {
                match event {
                    tauri::WindowEvent::Focused(focused) => {
                        #[cfg(target_os = "macos")]
                        if *focused {
                            gradient::show_background();
                        } else {
                            gradient::hide_background();
                        }
                    }
                    tauri::WindowEvent::CloseRequested { .. } => {
                        #[cfg(target_os = "macos")]
                        gradient::teardown_background();
                    }
                    tauri::WindowEvent::Destroyed => {
                        #[cfg(target_os = "macos")]
                        gradient::teardown_background();
                    }
                    _ => {}
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_disk_usage,
            large_files::scan_large_files_stream,
            caches_logs::scan_caches,
            caches_logs::scan_logs,
            docker::is_docker_installed,
            docker::get_docker_info,
            apps::scan_apps,
            get_trash_info,
            delete_files,
            docker::clean_docker,
            empty_trash,
            apps::uninstall_app,
            check_full_disk_access,
            check_path_access,
            request_path_access,
            open_full_disk_access_settings,
            reveal_in_finder,
            scan_security,
            disable_launch_item,
            remove_launch_item,
            scan_browsers,
            clean_browser_data,
            scan_duplicates,
            scan_similar_images,
            generate_thumbnails_batch,
            get_disk_map,
            expand_disk_node,
            enrich_disk_nodes,
            save_disk_map_cache,
            list_disk_map_caches,
            load_disk_map_cache,
            delete_disk_map_cache,
            export_disk_map,
            import_disk_map,
            get_maintenance_tasks,
            run_maintenance_task,
            scan_memory,
            preview_file,
            scan_packages,
            get_custom_probes,
            save_custom_probes,
            delete_custom_probe,
            test_probe_command,
            scan_vitals,
            quit_process,
            force_quit_process,
            quit_process_group,
            scan_thermal,
            gradient::set_native_background,
            gradient::update_native_background_position,
            save_scan_cache,
            load_scan_cache,
            scan_archive_candidates,
            compress_to_archive,
            restore_from_archive,
            get_archive_summary,
            get_archive_entries,
            delete_archive_entry,
            collect_archive_directory,
            compress_directory_to_archive,
            get_directory_size,
            compress_to_vault,
            restore_from_vault,
            get_vault_summary,
            get_vault_entries,
            delete_vault_entry,
            move_files_to_directory,
            get_storage_config,
            set_storage_config,
            check_intelligence_available,
            check_ai_available,
            classify_files_ai,
            generate_scan_summary_ai,
            render_sf_symbol,
            list_system_images,
            nsfw::scan_nsfw,
            nsfw::cancel_nsfw_scan,
            nsfw::dismiss_nsfw_paths,
            nsfw::clear_nsfw_dismissed,
            nsfw::delete_photo_assets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
