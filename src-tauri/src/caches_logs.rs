// caches_logs.rs — Cache and log directory scanning commands.
//
// Scans ~/Library/Caches, ~/Library/Logs, and /var/log for cleanable entries.
// TCC-aware: without Full Disk Access, only scans safe known directories.

use crate::commands::{self, CacheEntry, LogEntry};
use std::fs;
use std::path::Path;

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
pub async fn scan_caches(has_fda: Option<bool>) -> Result<Vec<CacheEntry>, String> {
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

/// Scan ~/Library/Logs and /var/log for `.log` files.
///
/// IMPORTANT (TCC): ~/Library/Logs can contain sub-directories for other apps.
/// Walking into them without Full Disk Access can trigger macOS TCC permission
/// dialogs. Without FDA, we only scan /var/log (which is generally readable).
/// With FDA, we also scan ~/Library/Logs.
#[tauri::command]
pub async fn scan_logs(has_fda: Option<bool>) -> Result<Vec<LogEntry>, String> {
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
