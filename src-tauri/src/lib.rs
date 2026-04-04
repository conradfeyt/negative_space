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

// Shared image utilities (loading, HEIC conversion) for similar image detection
// and content classification.
mod image_utils;

// Perceptual hash-based similar image detection — finds visually similar images
// even when they differ in resolution, compression, or format.
mod similar_images;

// Bring standard-library and crate items into scope.
use commands::{
    AppInfo, CacheEntry, CleanResult, DiskUsage, DockerInfo, DockerItem, FileInfo,
    LargeFileScanDone, LargeFileScanProgress, LargeFileFound, LogEntry, TrashInfo,
};
use std::fs;
use std::path::Path;


use std::os::unix::fs::MetadataExt;

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

// ---------------------------------------------------------------------------
// Command 2: scan_large_files
// ---------------------------------------------------------------------------

/// Recursively scan for files larger than `min_size_mb` megabytes.
///
/// CRITICAL TCC DESIGN:
/// Without Full Disk Access, we CANNOT walk from ~ (home directory).
/// macOS has dozens of TCC-protected directories under ~/ and ~/Library/
/// that trigger BLOCKING modal permission dialogs when accessed. There is
/// no complete list — Apple adds new ones with each macOS version.
///
/// The ONLY safe approach without FDA:
///   - Walk a whitelist of known-safe directories (developer tools, etc.)
///   - Never touch ~/Library/ (too many protected subdirs)
///   - Never touch ~/Desktop, ~/Documents, ~/Downloads, ~/Pictures, etc.
///
/// With FDA, we walk from the user-specified path (default ~) freely.
#[tauri::command]
async fn scan_large_files(
    path: String,
    min_size_mb: u64,
    skip_paths: Option<Vec<String>>,
    has_fda: Option<bool>,
) -> Result<commands::LargeFileScanResult, String> {
    let home = commands::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
    let fda = has_fda.unwrap_or(false);
    let min_bytes = min_size_mb * 1024 * 1024;

    // Resolve user-configured skip paths via shared helper.
    let user_skips = skip_paths.unwrap_or_default();
    let skip_prefixes = commands::build_skip_prefixes(&home, &user_skips, &[]);

    // Safe dirs for large-file scanner (developer-focused, broad coverage).
    let safe_dirs = vec![
        format!("{}/Library/Developer", home),
        "/usr/local".to_string(),
        "/opt/homebrew".to_string(),
        format!("{}/Projects", home),
        format!("{}/projects", home),
        format!("{}/src", home),
        format!("{}/dev", home),
        format!("{}/code", home),
        format!("{}/workspace", home),
        format!("{}/go", home),
        format!("{}/node_modules", home),
        format!("{}/.cargo", home),
        format!("{}/.rustup", home),
        format!("{}/.npm", home),
        format!("{}/.gradle", home),
        format!("{}/.m2", home),
        format!("{}/.docker", home),
        format!("{}/.local", home),
        format!("{}/.cache", home),
        "/tmp".to_string(),
        "/var/tmp".to_string(),
        "/Applications".to_string(),
    ];
    let scan_roots = commands::build_scan_roots(&home, &path, fda, &safe_dirs);

    // Track skipped paths so the UI can show what was missed.
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

    let mut results: Vec<FileInfo> = Vec::new();

    // Walk each scan root.
    for root in &scan_roots {
        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| {
                let p = e.path().to_string_lossy();
                !skip_prefixes
                    .iter()
                    .any(|prefix| p.starts_with(prefix.as_str()))
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

            let apparent_size = metadata.len();
            if apparent_size < min_bytes {
                continue;
            }

            let actual_size = metadata.blocks() * 512;
            let is_sparse = (actual_size as f64) < (apparent_size as f64 * 0.8);

            let modified = metadata
                .modified()
                .map(commands::format_system_time)
                .unwrap_or_else(|_| "unknown".to_string());

            let file_path = entry.path().to_string_lossy().to_string();
            let file_name = entry.file_name().to_string_lossy().to_string();

            results.push(FileInfo {
                path: file_path,
                name: file_name,
                apparent_size,
                actual_size,
                modified,
                is_sparse,
            });
        }
    }

    results.sort_by(|a, b| b.apparent_size.cmp(&a.apparent_size));
    results.truncate(100);

    Ok(commands::LargeFileScanResult {
        files: results,
        skipped_paths,
    })
}

// ---------------------------------------------------------------------------
// Command 2b: scan_large_files_stream (streaming version)
// ---------------------------------------------------------------------------

/// Streaming variant of `scan_large_files`. Instead of collecting all results
/// and returning them in one shot, this command emits Tauri events as it walks:
///
/// - `"large-file-found"` — emitted each time a qualifying file is discovered.
///   Payload: `LargeFileFound { file: FileInfo }`.
///
/// - `"large-file-progress"` — emitted when entering a new top-level directory
///   or finding a file, so the UI can show where we're currently looking.
///   Payload: `LargeFileScanProgress { current_dir, files_found }`.
///
/// - `"large-file-done"` — emitted once when the entire scan finishes.
///   Payload: `LargeFileScanDone { total_files, skipped_paths }`.
///
#[tauri::command]
async fn scan_large_files_stream(
    app: tauri::AppHandle,
    path: String,
    min_size_mb: u64,
    skip_paths: Option<Vec<String>>,
    has_fda: Option<bool>,
) -> Result<(), String> {
    use tauri::Emitter;

    let home = commands::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
    let fda = has_fda.unwrap_or(false);
    let min_bytes = min_size_mb * 1024 * 1024;

    // --- Build skip prefixes and scan roots via shared helpers ---
    let user_skips = skip_paths.unwrap_or_default();
    let skip_prefixes = commands::build_skip_prefixes(&home, &user_skips, &[]);

    // Safe dirs for large-file scanner (same list as non-streaming variant).
    let safe_dirs = vec![
        format!("{}/Library/Developer", home),
        "/usr/local".to_string(),
        "/opt/homebrew".to_string(),
        format!("{}/Projects", home),
        format!("{}/projects", home),
        format!("{}/src", home),
        format!("{}/dev", home),
        format!("{}/code", home),
        format!("{}/workspace", home),
        format!("{}/go", home),
        format!("{}/node_modules", home),
        format!("{}/.cargo", home),
        format!("{}/.rustup", home),
        format!("{}/.npm", home),
        format!("{}/.gradle", home),
        format!("{}/.m2", home),
        format!("{}/.docker", home),
        format!("{}/.local", home),
        format!("{}/.cache", home),
        "/tmp".to_string(),
        "/var/tmp".to_string(),
        "/Applications".to_string(),
    ];
    let scan_roots = commands::build_scan_roots(&home, &path, fda, &safe_dirs);

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

    // Helper: replace the literal home dir with ~ for display purposes.
    let home_for_display = home.clone();
    let display_path = move |p: &str| -> String {
        if p.starts_with(&home_for_display) {
            format!("~{}", &p[home_for_display.len()..])
        } else {
            p.to_string()
        }
    };

    let mut files_found: usize = 0;
    // Track the last directory we emitted progress for, to avoid flooding
    // the event bus with thousands of identical progress events.
    // Starts empty — updated immediately in the first loop iteration.
    let mut last_progress_dir: String;

    // Walk each scan root, emitting events as we go.
    for root in &scan_roots {
        // Emit progress when starting a new root directory.
        let root_display = display_path(root);
        let _ = app.emit(
            "large-file-progress",
            LargeFileScanProgress {
                current_dir: root_display.clone(),
                files_found,
            },
        );
        last_progress_dir = root_display;

        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| {
                let p = e.path().to_string_lossy();
                !skip_prefixes
                    .iter()
                    .any(|prefix| p.starts_with(prefix.as_str()))
            })
            .filter_map(|e| e.ok())
        {
            // For directories, emit progress so the UI shows where we are.
            // We only emit when the directory changes to keep event volume low.
            if entry.file_type().is_dir() {
                let dir_path = entry.path().to_string_lossy().to_string();
                let dir_display = display_path(&dir_path);
                // Only emit if this is a different directory than the last one
                // we reported, and limit depth to keep things readable.
                // Emit for depth <= 3 (relative to root) to avoid flooding.
                if dir_display != last_progress_dir && entry.depth() <= 3 {
                    let _ = app.emit(
                        "large-file-progress",
                        LargeFileScanProgress {
                            current_dir: dir_display.clone(),
                            files_found,
                        },
                    );
                    last_progress_dir = dir_display;
                }
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if !metadata.is_file() {
                continue;
            }

            let apparent_size = metadata.len();
            if apparent_size < min_bytes {
                continue;
            }

            let actual_size = metadata.blocks() * 512;
            let is_sparse = (actual_size as f64) < (apparent_size as f64 * 0.8);

            let modified = metadata
                .modified()
                .map(commands::format_system_time)
                .unwrap_or_else(|_| "unknown".to_string());

            let file_path = entry.path().to_string_lossy().to_string();
            let file_name = entry.file_name().to_string_lossy().to_string();

            let file_info = FileInfo {
                path: file_path,
                name: file_name,
                apparent_size,
                actual_size,
                modified,
                is_sparse,
            };

            // Emit the found file immediately so the UI can display it.
            let _ = app.emit("large-file-found", LargeFileFound { file: file_info });
            files_found += 1;

            // Also update progress with the file's parent directory.
            if let Some(parent) = entry.path().parent() {
                let parent_display = display_path(&parent.to_string_lossy());
                if parent_display != last_progress_dir {
                    let _ = app.emit(
                        "large-file-progress",
                        LargeFileScanProgress {
                            current_dir: parent_display.clone(),
                            files_found,
                        },
                    );
                    last_progress_dir = parent_display;
                }
            }
        }
    }

    // Emit final "done" event.
    let _ = app.emit(
        "large-file-done",
        LargeFileScanDone {
            total_files: files_found,
            skipped_paths,
        },
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Command 3: scan_caches
// ---------------------------------------------------------------------------

/// List cache directories under ~/Library/Caches, plus Xcode DerivedData and
/// CoreSimulator caches if they exist.
///
/// IMPORTANT (TCC): Without Full Disk Access, walking into ~/Library/Caches
/// sub-directories belonging to other apps triggers macOS "access data from
/// other apps" permission dialogs that BLOCK the thread (spinning beach ball).
///
/// When `has_fda` is false, we ONLY scan our own app's cache and safe known
/// directories (Xcode DerivedData, CoreSimulator). We skip the generic
/// ~/Library/Caches enumeration entirely — it's not safe without FDA.
#[tauri::command]
async fn scan_caches(has_fda: Option<bool>) -> Result<Vec<CacheEntry>, String> {
    let home = commands::home_dir().ok_or("Cannot determine home directory")?;
    let fda = has_fda.unwrap_or(false);
    let mut entries: Vec<CacheEntry> = Vec::new();

    // Primary cache directory — only scan individual sub-dirs if we have FDA.
    // Without FDA, reading into other apps' cache dirs triggers TCC prompts.
    let caches_root = format!("{}/Library/Caches", home);
    if fda && Path::new(&caches_root).is_dir() {
        if let Ok(reader) = fs::read_dir(&caches_root) {
            for dir_entry in reader.filter_map(|e| e.ok()) {
                let path = dir_entry.path();
                if path.is_dir() {
                    let (size, item_count) = commands::dir_size(&path.to_string_lossy());
                    entries.push(CacheEntry {
                        name: dir_entry.file_name().to_string_lossy().to_string(),
                        path: path.to_string_lossy().to_string(),
                        size,
                        item_count,
                    });
                }
            }
        }
    }

    // Xcode DerivedData — safe without FDA (it's our own developer directory).
    let derived_data = format!("{}/Library/Developer/Xcode/DerivedData", home);
    if Path::new(&derived_data).is_dir() {
        let (size, item_count) = commands::dir_size(&derived_data);
        entries.push(CacheEntry {
            name: "Xcode DerivedData".to_string(),
            path: derived_data,
            size,
            item_count,
        });
    }

    // CoreSimulator caches — safe without FDA.
    let sim_caches = format!("{}/Library/Developer/CoreSimulator/Caches", home);
    if Path::new(&sim_caches).is_dir() {
        let (size, item_count) = commands::dir_size(&sim_caches);
        entries.push(CacheEntry {
            name: "CoreSimulator Caches".to_string(),
            path: sim_caches,
            size,
            item_count,
        });
    }

    // Sort by size descending.
    entries.sort_by(|a, b| b.size.cmp(&a.size));

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Command 4: scan_logs
// ---------------------------------------------------------------------------

/// Scan ~/Library/Logs and /var/log for `.log` files.
///
/// IMPORTANT (TCC): ~/Library/Logs can contain sub-directories for other apps.
/// Walking into them without Full Disk Access can trigger macOS TCC permission
/// dialogs. Without FDA, we only scan /var/log (which is generally readable).
/// With FDA, we also scan ~/Library/Logs.
#[tauri::command]
async fn scan_logs(has_fda: Option<bool>) -> Result<Vec<LogEntry>, String> {
    let home = commands::home_dir().ok_or("Cannot determine home directory")?;
    let fda = has_fda.unwrap_or(false);
    let mut entries: Vec<LogEntry> = Vec::new();

    // Build the list of directories to scan based on FDA status.
    // ~/Library/Logs is only safe to walk with FDA granted.
    // /var/log is generally readable without FDA.
    let mut log_dirs: Vec<String> = vec!["/var/log".to_string()];

    if fda {
        log_dirs.insert(0, format!("{}/Library/Logs", home));
    }

    for dir in &log_dirs {
        if !Path::new(dir).is_dir() {
            continue;
        }

        for entry in walkdir::WalkDir::new(dir)
            .follow_links(false) // Don't follow symlinks — they may point to TCC dirs
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Only include regular files ending in `.log`.
            let is_log = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "log")
                .unwrap_or(false);

            if !is_log {
                continue;
            }

            if let Ok(metadata) = entry.metadata() {
                if !metadata.is_file() {
                    continue;
                }
                let modified = metadata
                    .modified()
                    .map(commands::format_system_time)
                    .unwrap_or_else(|_| "unknown".to_string());

                entries.push(LogEntry {
                    path: path.to_string_lossy().to_string(),
                    name: entry.file_name().to_string_lossy().to_string(),
                    size: metadata.len(),
                    modified,
                });
            }
        }
    }

    entries.sort_by(|a, b| b.size.cmp(&a.size));

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Command 4b: is_docker_installed
// ---------------------------------------------------------------------------

/// Quick check: is the Docker CLI binary present on disk?
/// No daemon contact — just checks common install paths + PATH.
#[tauri::command]
async fn is_docker_installed() -> bool {
    ["/usr/local/bin/docker", "/opt/homebrew/bin/docker"]
        .iter()
        .any(|p| std::path::Path::new(p).exists())
        || std::process::Command::new("which")
            .arg("docker")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

// Command 5: get_docker_info
// ---------------------------------------------------------------------------

/// Check if Docker is installed and, if so, gather image/disk info.
#[tauri::command]
async fn get_docker_info() -> Result<DockerInfo, String> {
    // 1. Check if docker is installed by looking in common locations.
    // Tauri apps launched from the dock may have a minimal PATH that
    // doesn't include /usr/local/bin or /opt/homebrew/bin.
    let docker_bin = ["/usr/local/bin/docker", "/opt/homebrew/bin/docker"]
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|s| s.to_string())
        .or_else(|| {
            // Fall back to PATH lookup.
            std::process::Command::new("which")
                .arg("docker")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        });

    let docker_bin = match docker_bin {
        Some(bin) => bin,
        None => {
            return Ok(DockerInfo {
                installed: false,
                running: false,
                images: vec![],
                total_reclaimable: String::new(),
                disk_usage_raw: String::new(),
            });
        }
    };

    // 2. Get `docker system df` output to check if daemon is running.
    let df_output = std::process::Command::new(&docker_bin)
        .args(["system", "df"])
        .output();
    let disk_usage_raw = match df_output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => {
            // Docker is installed but daemon is not running.
            return Ok(DockerInfo {
                installed: true,
                running: false,
                images: vec![],
                total_reclaimable: String::new(),
                disk_usage_raw: String::new(),
            });
        }
    };

    // Try to extract reclaimable info from the last column of each row.
    let mut total_reclaimable = String::new();
    for line in disk_usage_raw.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // The "RECLAIMABLE" column is typically the last one and looks like
        // "1.23GB (50%)" — we just grab whatever is there.
        if let Some(_last) = parts.last() {
            if !total_reclaimable.is_empty() {
                total_reclaimable.push_str(", ");
            }
            // Grab the last two tokens to capture "1.23GB (50%)"
            let reclaim_parts: Vec<&str> = parts.iter().rev().take(2).copied().collect();
            total_reclaimable.push_str(
                &reclaim_parts
                    .into_iter()
                    .rev()
                    .collect::<Vec<&str>>()
                    .join(" "),
            );
        }
    }

    // 3. Get docker images.
    let images_output = std::process::Command::new(&docker_bin)
        .args([
            "images",
            "--format",
            "{{.Repository}}:{{.Tag}}\t{{.Size}}\t{{.ID}}",
        ])
        .output();

    let mut images: Vec<DockerItem> = Vec::new();
    if let Ok(o) = images_output {
        if o.status.success() {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines() {
                // Each line: "name:tag\tsize\tid"
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 3 {
                    images.push(DockerItem {
                        name: parts[0].to_string(),
                        size: parts[1].to_string(),
                        id: parts[2].to_string(),
                        item_type: "image".to_string(),
                    });
                }
            }
        }
    }

    Ok(DockerInfo {
        installed: true,
        running: true,
        images,
        total_reclaimable,
        disk_usage_raw,
    })
}

// ---------------------------------------------------------------------------
// Command 6: scan_apps
// ---------------------------------------------------------------------------

/// List installed applications with their sizes, leftover footprint, icons, and
/// install source.
///
/// Scans two directories:
///   - /Applications  (main install location)
///   - ~/Applications (user-level installs, Homebrew cask aliases, etc.)
///
/// For each .app bundle we compute:
///   `size`       — the .app bundle itself
///   `leftover_size` — sum of all ~/Library leftover dirs (caches, support, etc.)
///   `footprint`  — size + leftover_size = total disk impact
///
/// IMPORTANT (TCC): Leftover detection reads ~/Library/* which can trigger TCC
/// prompts. Without FDA we skip leftover detection and just list apps + sizes.
#[tauri::command]
async fn scan_apps(has_fda: Option<bool>) -> Result<Vec<AppInfo>, String> {
    let home = commands::home_dir().ok_or("Cannot determine home directory")?;
    let fda = has_fda.unwrap_or(false);

    // Build lookup tables for install-source detection (run once, reuse per app).
    let homebrew_casks = detect_homebrew_casks();
    let app_store_paths = detect_app_store_apps();

    // Collect .app bundles from both /Applications and ~/Applications.
    let mut app_paths: Vec<std::path::PathBuf> = Vec::new();
    for dir_path in &["/Applications", &format!("{}/Applications", home)] {
        let dir = Path::new(dir_path);
        if !dir.is_dir() {
            continue;
        }
        if let Ok(reader) = fs::read_dir(dir) {
            for entry in reader.filter_map(|e| e.ok()) {
                let path = entry.path();
                let is_app = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "app")
                    .unwrap_or(false);
                if is_app && path.is_dir() {
                    app_paths.push(path);
                }
            }
        }
    }

    let mut apps: Vec<AppInfo> = Vec::new();

    for path in &app_paths {
        let app_path_str = path.to_string_lossy().to_string();
        let app_name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // App bundle size via `du -sk` (fast, avoids TCC).
        let size = get_du_size(&app_path_str);

        // Bundle ID from Info.plist.
        let plist_path = format!("{}/Contents/Info.plist", app_path_str);
        let bundle_id = get_bundle_id(&plist_path);

        // Leftover detection — only with FDA to avoid TCC modal dialogs.
        let (leftover_paths, leftover_size) = if fda {
            let paths = find_leftover_paths(&home, &app_name, &bundle_id);
            let total: u64 = paths.iter().map(|p| get_path_size(p)).sum();
            (paths, total)
        } else {
            (vec![], 0)
        };

        let footprint = size + leftover_size;

        // Extract the app's launcher icon as a base64-encoded PNG.
        let icon_base64 = get_app_icon(&app_path_str);

        // Determine install source.
        let install_source = if homebrew_casks.contains(&app_name) {
            "homebrew".to_string()
        } else if app_store_paths.contains(&app_path_str) {
            "app-store".to_string()
        } else {
            "manual".to_string()
        };

        apps.push(AppInfo {
            name: app_name,
            path: app_path_str,
            size,
            bundle_id,
            leftover_paths,
            leftover_size,
            footprint,
            icon_base64,
            install_source,
        });
    }

    // Sort by total footprint descending — shows the biggest disk hogs first.
    apps.sort_by(|a, b| b.footprint.cmp(&a.footprint));

    Ok(apps)
}

/// Run `du -sk <path>` and return the result in bytes.
///
/// Uses `du` subprocess to avoid triggering TCC prompts for protected directories.
fn get_du_size(path: &str) -> u64 {
    crate::commands::get_du_size(path)
}

/// Extract CFBundleIdentifier from an Info.plist using PlistBuddy.
fn get_bundle_id(plist_path: &str) -> String {
    let output = std::process::Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print CFBundleIdentifier", plist_path])
        .output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => String::new(),
    }
}

/// Extract an app's launcher icon as a base64-encoded PNG data URI.
///
/// Strategy:
///   1. Read CFBundleIconFile from Info.plist (e.g. "AppIcon" or "electron.icns")
///   2. Normalize — append ".icns" if missing
///   3. Convert .icns → 64x64 PNG using macOS built-in `sips`
///   4. Base64-encode and return as `data:image/png;base64,...`
///
/// Returns empty string on any failure (missing plist key, missing file, etc.).
fn get_app_icon(app_path: &str) -> String {
    use base64::Engine;

    let plist_path = format!("{}/Contents/Info.plist", app_path);

    // Step 1: Read the icon file name from Info.plist.
    // Try CFBundleIconFile first, then CFBundleIconName as fallback.
    let icon_name = {
        let output = std::process::Command::new("/usr/libexec/PlistBuddy")
            .args(["-c", "Print :CFBundleIconFile", &plist_path])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let name = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !name.is_empty() {
                    name
                } else {
                    // Fallback to CFBundleIconName (used by some Catalyst/SwiftUI apps).
                    let output2 = std::process::Command::new("/usr/libexec/PlistBuddy")
                        .args(["-c", "Print :CFBundleIconName", &plist_path])
                        .output();
                    match output2 {
                        Ok(o2) if o2.status.success() => {
                            String::from_utf8_lossy(&o2.stdout).trim().to_string()
                        }
                        _ => return String::new(),
                    }
                }
            }
            _ => return String::new(),
        }
    };

    if icon_name.is_empty() {
        return String::new();
    }

    // Step 2: Normalize — some apps omit the .icns extension.
    let icon_file = if icon_name.ends_with(".icns") {
        icon_name
    } else {
        format!("{}.icns", icon_name)
    };
    let icon_path = format!("{}/Contents/Resources/{}", app_path, icon_file);

    if !Path::new(&icon_path).exists() {
        return String::new();
    }

    // Step 3: Convert .icns → 64x64 PNG using `sips` (built into macOS).
    // We use a temp file for the output. 64px is enough for sidebar-sized icons
    // and keeps the base64 payload small (~6-10KB per icon).
    let tmp_path = format!("/tmp/negative_space_icon_{}.png", std::process::id());
    let sips_result = std::process::Command::new("sips")
        .args([
            "-s", "format", "png",
            &icon_path,
            "--out", &tmp_path,
            "--resampleWidth", "64",
        ])
        // Suppress sips stdout (it prints the input/output paths).
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if !matches!(sips_result, Ok(s) if s.success()) {
        let _ = fs::remove_file(&tmp_path);
        return String::new();
    }

    // Step 4: Read PNG bytes, base64-encode, build data URI.
    let result = match fs::read(&tmp_path) {
        Ok(bytes) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            format!("data:image/png;base64,{}", b64)
        }
        Err(_) => String::new(),
    };

    // Clean up temp file.
    let _ = fs::remove_file(&tmp_path);

    result
}

/// Look for leftover files in ~/Library/ for a given app name and bundle id.
///
/// Checks a comprehensive set of Library subdirectories where macOS apps
/// commonly store their data. The search uses two identifiers:
///   - `app_name` — matches dirs like "Application Support/Slack/"
///   - `bundle_id` — matches dirs like "Caches/com.tinyspeck.slackmacgap/"
///
/// Also does fuzzy matching on Caches/ and HTTPStorages/ because some apps
/// use suffixed directory names (e.g. "com.foo.app.ShipIt").
fn find_leftover_paths(home: &str, app_name: &str, bundle_id: &str) -> Vec<String> {
    let mut leftovers: Vec<String> = Vec::new();
    let lib = format!("{}/Library", home);

    // --- Directories matched by EXACT app name ---
    // Many apps store data under their display name rather than their bundle ID.
    // e.g. ~/Library/Caches/Google/, ~/Library/Application Support/Slack/
    let name_match_dirs = [
        "Application Support",
        "Caches",
        "Logs",
        "HTTPStorages",
        "WebKit",
        "Saved Application State",
    ];

    for dir_name in &name_match_dirs {
        let candidate = format!("{}/{}/{}", lib, dir_name, app_name);
        if Path::new(&candidate).exists() && !leftovers.contains(&candidate) {
            leftovers.push(candidate);
        }
    }

    // --- Directories matched by EXACT bundle ID ---
    if !bundle_id.is_empty() {
        let bundle_exact_dirs = [
            "Caches",
            "HTTPStorages",
            "WebKit",
            "Containers",
            "Saved Application State",
        ];

        for dir_name in &bundle_exact_dirs {
            let candidate = format!("{}/{}/{}", lib, dir_name, bundle_id);
            if Path::new(&candidate).exists() && !leftovers.contains(&candidate) {
                leftovers.push(candidate);
            }
            // Saved Application State uses "{bundle_id}.savedState" naming.
            if *dir_name == "Saved Application State" {
                let saved = format!("{}/{}/{}.savedState", lib, dir_name, bundle_id);
                if Path::new(&saved).exists() && !leftovers.contains(&saved) {
                    leftovers.push(saved);
                }
            }
        }

        // Preferences: {bundle_id}.plist
        let prefs = format!("{}/Preferences/{}.plist", lib, bundle_id);
        if Path::new(&prefs).exists() && !leftovers.contains(&prefs) {
            leftovers.push(prefs);
        }

        // LaunchAgents: {bundle_id}.plist
        let launch = format!("{}/LaunchAgents/{}.plist", lib, bundle_id);
        if Path::new(&launch).exists() && !leftovers.contains(&launch) {
            leftovers.push(launch);
        }

        // Cookies: {bundle_id}.binarycookies
        let cookies = format!("{}/Cookies/{}.binarycookies", lib, bundle_id);
        if Path::new(&cookies).exists() && !leftovers.contains(&cookies) {
            leftovers.push(cookies);
        }
    }

    // --- Fuzzy matching: Caches and HTTPStorages often have suffixed dirs ---
    // e.g. "com.anthropic.claudefordesktop.ShipIt" should match bundle_id
    // "com.anthropic.claudefordesktop".
    if !bundle_id.is_empty() {
        for dir_name in &["Caches", "HTTPStorages"] {
            let dir_path = format!("{}/{}", lib, dir_name);
            if let Ok(reader) = fs::read_dir(&dir_path) {
                for entry in reader.filter_map(|e| e.ok()) {
                    let entry_name = entry.file_name().to_string_lossy().to_string();
                    // Skip exact matches (already added above).
                    if entry_name == bundle_id {
                        continue;
                    }
                    // Match if the entry starts with the bundle_id (suffix like .ShipIt).
                    if entry_name.starts_with(bundle_id) {
                        let full = format!("{}/{}", dir_path, entry_name);
                        if !leftovers.contains(&full) {
                            leftovers.push(full);
                        }
                    }
                }
            }
        }
    }

    // --- Group Containers: matched by vendor domain portion of bundle_id ---
    // Bundle ID "com.docker.docker" → look for dirs containing "com.docker".
    if !bundle_id.is_empty() {
        let parts: Vec<&str> = bundle_id.split('.').collect();
        if parts.len() >= 2 {
            let vendor_domain = format!("{}.{}", parts[0], parts[1]);
            let group_dir = format!("{}/Group Containers", lib);
            if let Ok(reader) = fs::read_dir(&group_dir) {
                for entry in reader.filter_map(|e| e.ok()) {
                    let entry_name = entry.file_name().to_string_lossy().to_string();
                    if entry_name.contains(&vendor_domain) {
                        let full = format!("{}/{}", group_dir, entry_name);
                        if !leftovers.contains(&full) {
                            leftovers.push(full);
                        }
                    }
                }
            }
        }
    }

    // --- Application Support: also check by bundle_id (some apps use it) ---
    if !bundle_id.is_empty() {
        let app_support_bid = format!("{}/Application Support/{}", lib, bundle_id);
        if Path::new(&app_support_bid).exists() && !leftovers.contains(&app_support_bid) {
            leftovers.push(app_support_bid);
        }
    }

    leftovers
}

/// Get the size of a path (file or directory) in bytes.
/// Uses `du -sk` subprocess to avoid TCC dialogs on protected directories.
fn get_path_size(path: &str) -> u64 {
    get_du_size(path)
}

/// Detect Homebrew cask apps by running `brew list --cask`.
///
/// Returns a HashSet of app *display names* (e.g. "AltTab", "Docker") that
/// were installed via Homebrew. We map cask tokens to their actual .app names
/// by checking /opt/homebrew/Caskroom (Apple Silicon) or /usr/local/Caskroom.
///
/// If Homebrew is not installed, returns an empty set (no error).
fn detect_homebrew_casks() -> std::collections::HashSet<String> {
    let mut cask_apps = std::collections::HashSet::new();

    // Step 1: Get list of installed cask tokens.
    let output = std::process::Command::new("brew")
        .args(["list", "--cask"])
        .output();

    let tokens: Vec<String> = match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
        _ => return cask_apps, // brew not installed or failed
    };

    // Step 2: For each cask, check the Caskroom to find the actual .app name.
    // Caskroom layout: /opt/homebrew/Caskroom/{token}/{version}/{AppName}.app
    let caskroom_candidates = ["/opt/homebrew/Caskroom", "/usr/local/Caskroom"];
    let caskroom = caskroom_candidates.iter().find(|p| Path::new(p).is_dir());

    if let Some(caskroom_path) = caskroom {
        for token in &tokens {
            let token_dir = format!("{}/{}", caskroom_path, token);
            // Walk into the version subdirectory to find .app files.
            if let Ok(versions) = fs::read_dir(&token_dir) {
                for version_entry in versions.filter_map(|e| e.ok()) {
                    if let Ok(files) = fs::read_dir(version_entry.path()) {
                        for file in files.filter_map(|e| e.ok()) {
                            let name = file.file_name().to_string_lossy().to_string();
                            if name.ends_with(".app") {
                                // "AltTab.app" → "AltTab"
                                let app_name = name.trim_end_matches(".app").to_string();
                                cask_apps.insert(app_name);
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: if Caskroom lookup didn't match everything, insert the token
    // names with common transformations (capitalize first letter, etc.).
    // This is imperfect but catches simple cases like "docker" → "Docker".
    if cask_apps.is_empty() {
        for token in &tokens {
            // Simple title-case: "alt-tab" → "Alt-Tab"
            let titled: String = token
                .split('-')
                .map(|part| {
                    let mut c = part.chars();
                    match c.next() {
                        None => String::new(),
                        Some(first) => {
                            first.to_uppercase().to_string() + c.as_str()
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join("-");
            cask_apps.insert(titled);
        }
    }

    cask_apps
}

/// Detect Mac App Store apps using Spotlight metadata.
///
/// Returns a HashSet of absolute .app paths that were installed from the
/// Mac App Store. Uses `mdfind` which queries the Spotlight index.
///
/// If Spotlight is unavailable, returns an empty set.
fn detect_app_store_apps() -> std::collections::HashSet<String> {
    let output = std::process::Command::new("mdfind")
        .args(["kMDItemAppStoreHasReceipt == 1"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
        _ => std::collections::HashSet::new(),
    }
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
async fn delete_files(paths: Vec<String>) -> CleanResult {
    let mut freed_bytes: u64 = 0;
    let mut deleted_count: u64 = 0;
    let mut errors: Vec<String> = Vec::new();

    for path_str in &paths {
        let path = Path::new(path_str);

        // Get the size before we delete, so we know how much space was freed.
        // We use `du -sk` (subprocess) instead of walkdir to avoid TCC dialogs
        // if the path contains items from TCC-protected directories.
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

    CleanResult {
        success: errors.is_empty(),
        freed_bytes,
        deleted_count,
        errors,
    }
}

// ---------------------------------------------------------------------------
// Command 9: clean_docker
// ---------------------------------------------------------------------------

/// Run `docker system prune` to free unused Docker resources.
/// If `prune_all` is true, also removes all unused images (not just dangling ones).
#[tauri::command]
async fn clean_docker(prune_all: bool) -> CleanResult {
    let mut args = vec!["system", "prune", "-f"];
    if prune_all {
        args.push("-a");
    }

    let output = std::process::Command::new("docker").args(&args).output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();

            // Docker prune output typically ends with a line like:
            // "Total reclaimed space: 1.23GB"
            let freed_display = stdout
                .lines()
                .find(|line| line.contains("reclaimed space"))
                .unwrap_or("Total reclaimed space: 0B")
                .to_string();

            CleanResult {
                success: true,
                freed_bytes: 0, // Docker doesn't give us exact bytes easily
                deleted_count: 0,
                errors: vec![freed_display], // We repurpose errors to carry the message
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            CleanResult {
                success: false,
                freed_bytes: 0,
                deleted_count: 0,
                errors: vec![format!("docker prune failed: {}", stderr)],
            }
        }
        Err(e) => CleanResult {
            success: false,
            freed_bytes: 0,
            deleted_count: 0,
            errors: vec![format!("Failed to run docker: {}", e)],
        },
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
// Command 11: uninstall_app
// ---------------------------------------------------------------------------

/// Move an .app to the Trash via Finder (so it goes to the Trash instead of
/// being permanently deleted), and optionally remove leftover Library files.
#[tauri::command]
async fn uninstall_app(app_path: String, remove_leftovers: bool) -> CleanResult {
    let mut freed_bytes: u64 = 0;
    let mut deleted_count: u64 = 0;
    let mut errors: Vec<String> = Vec::new();

    let path = Path::new(&app_path);
    if !path.exists() {
        return CleanResult {
            success: false,
            freed_bytes: 0,
            deleted_count: 0,
            errors: vec![format!("App not found: {}", app_path)],
        };
    }

    // Measure app size before trashing.
    let app_size = get_du_size(&app_path);

    // Use AppleScript to move the .app to Trash. This is the macOS-native way
    // and shows the expected "moved to Trash" behaviour.
    let script = format!(
        "tell application \"Finder\" to delete POSIX file \"{}\"",
        app_path
    );

    let result = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output();

    match result {
        Ok(o) if o.status.success() => {
            freed_bytes += app_size;
            deleted_count += 1;
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            errors.push(format!("Failed to trash {}: {}", app_path, stderr));
        }
        Err(e) => {
            errors.push(format!("Failed to run osascript: {}", e));
        }
    }

    // Optionally remove leftover Library files.
    if remove_leftovers {
        let home = commands::home_dir().unwrap_or_default();
        let app_name = Path::new(&app_path)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let plist_path = format!("{}/Contents/Info.plist", app_path);
        let bundle_id = get_bundle_id(&plist_path);

        let leftover_paths = find_leftover_paths(&home, &app_name, &bundle_id);

        for leftover in &leftover_paths {
            let leftover_p = Path::new(leftover);
            if !leftover_p.exists() {
                continue;
            }

            let size = get_path_size(leftover);

            let del_result = if leftover_p.is_dir() {
                fs::remove_dir_all(leftover_p)
            } else {
                fs::remove_file(leftover_p)
            };

            match del_result {
                Ok(()) => {
                    freed_bytes += size;
                    deleted_count += 1;
                }
                Err(e) => {
                    errors.push(format!("Failed to delete leftover {}: {}", leftover, e));
                }
            }
        }
    }

    CleanResult {
        success: errors.is_empty(),
        freed_bytes,
        deleted_count,
        errors,
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
) -> Result<duplicates::DuplicateScanResult, String> {
    let fda = has_fda.unwrap_or(false);
    let skips = skip_paths.unwrap_or_default();
    Ok(duplicates::run_duplicate_scan(&path, min_size_mb, fda, &skips))
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
) -> Result<similar_images::SimilarScanResult, String> {
    let fda = has_fda.unwrap_or(false);
    let skips = skip_paths.unwrap_or_default();
    let min_bytes = if min_size_mb > 0 { min_size_mb * 1024 * 1024 } else { 10 * 1024 }; // default 10KB min
    Ok(similar_images::run_similar_scan(&app, threshold, min_bytes, fda, &skips))
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
// Command 36-41: Vault (compress-instead-of-delete)
// ---------------------------------------------------------------------------

#[tauri::command]
async fn scan_vault_candidates(
    path: String,
    min_size_mb: u64,
    min_age_days: u64,
    fda: bool,
) -> Vec<vault::CompressionCandidate> {
    vault::scan_candidates(&path, min_size_mb, min_age_days, fda)
}

#[tauri::command]
async fn compress_to_vault(paths: Vec<String>) -> vault::CompressResult {
    vault::compress_files(&paths)
}

#[tauri::command]
async fn restore_from_vault(entry_id: String) -> vault::RestoreResult {
    vault::restore_file(&entry_id)
}

#[tauri::command]
async fn get_vault_summary() -> vault::VaultSummary {
    vault::get_summary()
}

#[tauri::command]
async fn get_vault_entries() -> Vec<vault::VaultEntry> {
    vault::get_entries()
}

#[tauri::command]
async fn delete_vault_entry(entry_id: String) -> Result<(), String> {
    vault::delete_entry(&entry_id)
}

#[tauri::command]
async fn collect_vault_directory(path: String) -> Vec<vault::CompressionCandidate> {
    vault::collect_directory_files(&path)
}

#[tauri::command]
async fn get_directory_size(path: String) -> u64 {
    commands::dir_size(&path).0
}

#[tauri::command]
async fn compress_directory_to_vault(path: String) -> vault::CompressResult {
    vault::compress_directory(&path)
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
            scan_large_files,
            scan_large_files_stream,
            scan_caches,
            scan_logs,
            is_docker_installed,
            get_docker_info,
            scan_apps,
            get_trash_info,
            delete_files,
            clean_docker,
            empty_trash,
            uninstall_app,
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
            scan_vitals,
            quit_process,
            force_quit_process,
            quit_process_group,
            scan_thermal,
            gradient::set_native_background,
            gradient::update_native_background_position,
            save_scan_cache,
            load_scan_cache,
            scan_vault_candidates,
            compress_to_vault,
            restore_from_vault,
            get_vault_summary,
            get_vault_entries,
            delete_vault_entry,
            collect_vault_directory,
            compress_directory_to_vault,
            get_directory_size,
            check_intelligence_available,
            check_ai_available,
            classify_files_ai,
            generate_scan_summary_ai,
            render_sf_symbol,
            list_system_images,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
