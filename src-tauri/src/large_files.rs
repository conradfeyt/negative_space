// large_files.rs — Streaming large file scanner.
//
// Walks the filesystem emitting Tauri events as large files are discovered,
// so the UI can display results incrementally.

use crate::commands::{
    self, FileInfo, LargeFileFound, LargeFileScanDone, LargeFileScanProgress,
};
use std::os::unix::fs::MetadataExt;

/// Streaming variant of large file scanning. Emits Tauri events as it walks:
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
pub async fn scan_large_files_stream(
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

    // Emit final "done" event — critical for the frontend to exit scanning state.
    if let Err(e) = app.emit(
        "large-file-done",
        LargeFileScanDone {
            total_files: files_found,
            skipped_paths,
        },
    ) {
        eprintln!("[scan] failed to emit done: {}", e);
    }

    Ok(())
}
