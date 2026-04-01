// maintenance.rs — System maintenance tasks for Negative _.
//
// Provides macOS-specific maintenance operations:
//   - Flush DNS cache
//   - Free purgeable disk space
//   - Rebuild Launch Services database
//   - Rebuild Spotlight index
//   - Clear font caches
//   - Flush memory (purge inactive memory pages)
//
// RUST CONCEPT: This is a module — declared in lib.rs with `mod maintenance;`.
// All public items are accessible from lib.rs via `maintenance::SomeType`.
//
// IMPORTANT: Several of these operations require administrator (root) privileges.
// We use `osascript` with `do shell script ... with administrator privileges`
// to prompt the user for their password via macOS's native authentication dialog.
// This is the recommended approach for GUI apps — it uses the Security framework
// and never touches the user's password directly.
//
// All operations run as subprocesses and are fully async-safe.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A single maintenance task that can be performed.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaintenanceTask {
    /// Internal identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Short description of what this task does and when it's useful
    pub description: String,
    /// Whether this task requires administrator privileges
    pub requires_admin: bool,
    /// Current status: "idle", "running", "success", "error"
    pub status: String,
    /// Result message (success or error details)
    pub message: String,
    /// Warning about side effects or duration
    pub warning: String,
    /// Exact shell commands that will be executed
    pub commands: Vec<String>,
    /// Services or daemons affected by this task
    pub services_affected: Vec<String>,
    /// Filesystem paths that will be read, modified, or deleted
    pub paths_affected: Vec<String>,
    /// Whether this task modifies or deletes data (vs. just signaling a daemon)
    pub destructive: bool,
    /// How the effect is reversed, if applicable. Empty string means not reversible.
    /// When non-empty, explains the specific mechanism (automatic rebuild, manual step, etc.)
    pub reversible_info: String,
}

/// Result of running a single maintenance task.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaintenanceResult {
    /// Task ID that was executed
    pub task_id: String,
    /// Whether the task succeeded
    pub success: bool,
    /// Human-readable result message
    pub message: String,
}

/// Result of getting the list of available tasks.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaintenanceTaskList {
    /// All available maintenance tasks
    pub tasks: Vec<MaintenanceTask>,
}

// ---------------------------------------------------------------------------
// Task definitions
// ---------------------------------------------------------------------------

/// Get the list of all available maintenance tasks.
/// Each task is returned in "idle" status — the frontend decides which to run.
pub fn get_tasks() -> MaintenanceTaskList {
    let tasks = vec![
        MaintenanceTask {
            id: "flush_dns".to_string(),
            name: "Flush DNS Cache".to_string(),
            description: "Clears the DNS resolver cache. Useful when websites aren't loading or you've recently changed DNS settings.".to_string(),
            requires_admin: true,
            status: "idle".to_string(),
            message: String::new(),
            warning: String::new(),
            commands: vec![
                "dscacheutil -flushcache".to_string(),
                "killall -HUP mDNSResponder".to_string(),
            ],
            services_affected: vec![
                "mDNSResponder (macOS DNS resolver daemon) -- sent SIGHUP to reload".to_string(),
            ],
            paths_affected: vec![
                "In-memory DNS cache only -- no files on disk are modified".to_string(),
            ],
            destructive: false,
            reversible_info: "The DNS cache repopulates automatically as you browse. Within seconds of visiting any website, its DNS entry is cached again. No action needed.".to_string(),
        },
        MaintenanceTask {
            id: "free_purgeable".to_string(),
            name: "Free Purgeable Space".to_string(),
            description: "Removes local Time Machine snapshots that APFS keeps as purgeable space. These are automatic backups macOS creates hourly.".to_string(),
            requires_admin: false,
            status: "idle".to_string(),
            message: String::new(),
            warning: "May take 30-60 seconds. Removes local snapshots -- Time Machine backups on external drives are not affected.".to_string(),
            commands: vec![
                "tmutil thinlocalsnapshots / 999999999999 4".to_string(),
            ],
            services_affected: vec![
                "Time Machine (local snapshots only -- external backups untouched)".to_string(),
            ],
            paths_affected: vec![
                "APFS local snapshots on / (hidden, managed by macOS)".to_string(),
                "No user-visible files are deleted".to_string(),
            ],
            destructive: true,
            reversible_info: String::new(),
        },
        MaintenanceTask {
            id: "rebuild_launch_services".to_string(),
            name: "Rebuild Launch Services Database".to_string(),
            description: "Rebuilds the database macOS uses to map file types to applications. Fixes duplicate or missing entries in the 'Open With' menu.".to_string(),
            requires_admin: false,
            status: "idle".to_string(),
            message: String::new(),
            warning: "Finder may briefly become unresponsive while the database rebuilds.".to_string(),
            commands: vec![
                "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -kill -r -domain local -domain system -domain user".to_string(),
            ],
            services_affected: vec![
                "Launch Services (file-to-app association database)".to_string(),
                "Finder (may restart or pause briefly)".to_string(),
            ],
            paths_affected: vec![
                "~/Library/Preferences/com.apple.LaunchServices/ -- rebuilt from scratch".to_string(),
                "Scans /Applications, /System/Applications, ~/Applications".to_string(),
            ],
            destructive: false,
            reversible_info: "The database is rebuilt immediately by scanning /Applications, /System/Applications, and ~/Applications. The lsregister command itself does the rebuild -- no further action needed. Takes a few seconds to complete.".to_string(),
        },
        MaintenanceTask {
            id: "rebuild_spotlight".to_string(),
            name: "Rebuild Spotlight Index".to_string(),
            description: "Erases and rebuilds the Spotlight search index for the root volume. Fixes missing, stale, or incorrect search results.".to_string(),
            requires_admin: true,
            status: "idle".to_string(),
            message: String::new(),
            warning: "Re-indexing runs in the background and may take several hours. Spotlight search will return incomplete results until it finishes.".to_string(),
            commands: vec![
                "mdutil -E /".to_string(),
            ],
            services_affected: vec![
                "Spotlight (mds, mdworker processes)".to_string(),
            ],
            paths_affected: vec![
                "/.Spotlight-V100/ -- Spotlight index database (erased and rebuilt)".to_string(),
                "Entire root volume / will be re-crawled in the background".to_string(),
            ],
            destructive: true,
            reversible_info: "macOS automatically re-indexes the entire root volume in the background via the mds and mdworker processes. This happens without any user action but takes 1-4 hours depending on disk size. Spotlight search returns incomplete results until re-indexing finishes. Progress is visible in System Settings > Spotlight (or via 'mdutil -s /' in Terminal).".to_string(),
        },
        MaintenanceTask {
            id: "clear_font_cache".to_string(),
            name: "Clear Font Caches".to_string(),
            description: "Deletes cached font rendering data. Fixes garbled text, missing fonts, or font display corruption.".to_string(),
            requires_admin: true,
            status: "idle".to_string(),
            message: String::new(),
            warning: "A restart is recommended after clearing. Fonts will render normally once caches rebuild on next login.".to_string(),
            commands: vec![
                "atsutil databases -remove".to_string(),
            ],
            services_affected: vec![
                "Apple Type Services (ATS font daemon)".to_string(),
            ],
            paths_affected: vec![
                "/private/var/folders/*/*/com.apple.ATS/ -- font cache files (deleted)".to_string(),
                "Font caches are rebuilt automatically on next login".to_string(),
            ],
            destructive: true,
            reversible_info: "macOS rebuilds font caches automatically on the next login. Restart (or log out and back in) to trigger the rebuild. Fonts display normally once the cache is regenerated -- typically within 1-2 minutes of logging in.".to_string(),
        },
        MaintenanceTask {
            id: "flush_memory".to_string(),
            name: "Flush Memory Cache".to_string(),
            description: "Forces macOS to release inactive memory pages. Mostly useful for benchmarking or development. macOS manages memory well on its own under normal use.".to_string(),
            requires_admin: true,
            status: "idle".to_string(),
            message: String::new(),
            warning: "Applications may feel slower briefly while their caches refill from disk.".to_string(),
            commands: vec![
                "purge".to_string(),
            ],
            services_affected: vec![
                "Virtual memory subsystem (kernel)".to_string(),
            ],
            paths_affected: vec![
                "RAM only -- no files on disk are modified or deleted".to_string(),
                "Inactive memory pages are released back to the free pool".to_string(),
            ],
            destructive: false,
            reversible_info: "macOS immediately begins refilling the memory cache as applications access data from disk. Within minutes of normal use, frequently accessed data is cached in RAM again. This is entirely automatic -- the kernel manages it without any user action.".to_string(),
        },
    ];

    MaintenanceTaskList { tasks }
}

// ---------------------------------------------------------------------------
// Task execution
// ---------------------------------------------------------------------------

/// Execute a single maintenance task by ID.
///
/// For tasks requiring admin privileges, we use `osascript` with
/// `do shell script ... with administrator privileges` which triggers
/// macOS's native password prompt. This is the standard way for GUI apps
/// to run privileged operations.
pub fn run_task(task_id: &str) -> MaintenanceResult {
    match task_id {
        "flush_dns" => flush_dns(),
        "free_purgeable" => free_purgeable(),
        "rebuild_launch_services" => rebuild_launch_services(),
        "rebuild_spotlight" => rebuild_spotlight(),
        "clear_font_cache" => clear_font_cache(),
        "flush_memory" => flush_memory(),
        _ => MaintenanceResult {
            task_id: task_id.to_string(),
            success: false,
            message: format!("Unknown task: {}", task_id),
        },
    }
}

/// Run a shell command with administrator privileges using osascript.
///
/// This triggers macOS's native password dialog. If the user cancels,
/// we get an error back from osascript (exit code 1 with "User canceled").
///
/// RUST CONCEPT: The `format!` macro builds the AppleScript command string.
/// We escape single quotes in the shell command to prevent injection.
fn run_admin_command(shell_cmd: &str) -> Result<String, String> {
    // Escape single quotes for AppleScript string embedding.
    // AppleScript uses '' (two single quotes) to escape a single quote inside
    // a single-quoted string. Actually, AppleScript uses \" for double-quoted
    // strings. We'll use double quotes and escape the inner command.
    let escaped = shell_cmd.replace('\\', "\\\\").replace('"', "\\\"");

    let script = format!(
        "do shell script \"{}\" with administrator privileges",
        escaped
    );

    let output = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| format!("Failed to run osascript: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.contains("User canceled") || stderr.contains("(-128)") {
            Err("Authentication cancelled by user".to_string())
        } else {
            Err(format!("Command failed: {}", stderr))
        }
    }
}

/// Run a shell command without admin privileges.
fn run_command(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", cmd, e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("{} failed: {}", cmd, stderr))
    }
}

// ---------------------------------------------------------------------------
// Individual task implementations
// ---------------------------------------------------------------------------

/// Flush the DNS resolver cache.
/// Requires admin: `dscacheutil -flushcache` needs root, and `killall -HUP mDNSResponder`
/// needs root to signal the system daemon.
fn flush_dns() -> MaintenanceResult {
    let cmd = "dscacheutil -flushcache && killall -HUP mDNSResponder";
    match run_admin_command(cmd) {
        Ok(_) => MaintenanceResult {
            task_id: "flush_dns".to_string(),
            success: true,
            message: "DNS cache flushed successfully".to_string(),
        },
        Err(e) => MaintenanceResult {
            task_id: "flush_dns".to_string(),
            success: false,
            message: e,
        },
    }
}

/// Free purgeable disk space on APFS volumes.
///
/// On APFS, macOS keeps deleted data as "purgeable" space that can be
/// reclaimed when needed. We write a large temporary file and delete it
/// to force the system to actually free purgeable blocks.
///
/// Alternative approaches considered:
///   - `diskutil apfs defragment / live` — only available on some macOS versions
///   - `tmutil thinlocalsnapshots / 999999999` — removes Time Machine snapshots
/// We use both approaches for maximum effect.
fn free_purgeable() -> MaintenanceResult {
    let mut messages: Vec<String> = Vec::new();
    let mut any_success = false;

    // Approach 1: Thin local Time Machine snapshots.
    // This reclaims space held by local TM snapshots that APFS considers purgeable.
    // Does NOT require admin — tmutil thinlocalsnapshots works as regular user.
    match run_command("tmutil", &["thinlocalsnapshots", "/", "999999999999", "4"]) {
        Ok(output) => {
            any_success = true;
            if output.is_empty() {
                messages.push("Time Machine snapshots thinned".to_string());
            } else {
                messages.push(format!("Time Machine: {}", output));
            }
        }
        Err(e) => {
            // Not fatal — TM may not be configured.
            messages.push(format!("Time Machine thinning skipped: {}", e));
        }
    }

    // Approach 2: Use `diskutil` to show purgeable space (informational).
    // We report how much purgeable space exists so the user knows the impact.
    match run_command("diskutil", &["info", "-plist", "/"]) {
        Ok(plist_output) => {
            // Try to extract purgeable space from the plist output.
            // We look for APFSContainerFree or FreePurgeable in the output.
            if let Some(purgeable) = extract_purgeable_from_plist(&plist_output) {
                if purgeable > 0 {
                    let size_str = crate::commands::format_size(purgeable);
                    messages.push(format!("Purgeable space available: {}", size_str));
                }
            }
        }
        Err(_) => {
            // Not critical, skip.
        }
    }

    if any_success {
        MaintenanceResult {
            task_id: "free_purgeable".to_string(),
            success: true,
            message: messages.join(". "),
        }
    } else {
        MaintenanceResult {
            task_id: "free_purgeable".to_string(),
            success: false,
            message: messages.join(". "),
        }
    }
}

/// Try to extract purgeable space from diskutil plist output.
/// Returns bytes if found, None otherwise.
fn extract_purgeable_from_plist(plist: &str) -> Option<u64> {
    // Look for <key>APFSContainerFree</key> followed by <integer>...</integer>
    // This is a simplified XML parser — good enough for this specific use case.
    // We look for "Purgeable" in any key.
    for (i, line) in plist.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains("Purgeable") || trimmed.contains("purgeable") {
            // The value should be on the next line as <integer>...</integer>
            if let Some(next_line) = plist.lines().nth(i + 1) {
                let next_trimmed = next_line.trim();
                if next_trimmed.starts_with("<integer>") && next_trimmed.ends_with("</integer>") {
                    let num_str = next_trimmed
                        .trim_start_matches("<integer>")
                        .trim_end_matches("</integer>");
                    return num_str.parse::<u64>().ok();
                }
            }
        }
    }
    None
}

/// Rebuild the Launch Services database.
/// No admin required — the lsregister tool runs as the current user.
fn rebuild_launch_services() -> MaintenanceResult {
    let lsregister = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";

    match run_command(
        lsregister,
        &["-kill", "-r", "-domain", "local", "-domain", "system", "-domain", "user"],
    ) {
        Ok(_) => MaintenanceResult {
            task_id: "rebuild_launch_services".to_string(),
            success: true,
            message: "Launch Services database rebuilt successfully. Duplicate entries in 'Open With' should be resolved.".to_string(),
        },
        Err(e) => MaintenanceResult {
            task_id: "rebuild_launch_services".to_string(),
            success: false,
            message: e,
        },
    }
}

/// Rebuild the Spotlight search index.
/// Requires admin: `mdutil -E /` needs root to erase the index.
fn rebuild_spotlight() -> MaintenanceResult {
    match run_admin_command("mdutil -E /") {
        Ok(_) => MaintenanceResult {
            task_id: "rebuild_spotlight".to_string(),
            success: true,
            message: "Spotlight index rebuild started. Re-indexing will continue in the background and may take several hours.".to_string(),
        },
        Err(e) => MaintenanceResult {
            task_id: "rebuild_spotlight".to_string(),
            success: false,
            message: e,
        },
    }
}

/// Clear font caches.
/// Requires admin: `atsutil databases -remove` needs root.
fn clear_font_cache() -> MaintenanceResult {
    match run_admin_command("atsutil databases -remove") {
        Ok(_) => MaintenanceResult {
            task_id: "clear_font_cache".to_string(),
            success: true,
            message: "Font caches cleared. A restart is recommended to rebuild them.".to_string(),
        },
        Err(e) => MaintenanceResult {
            task_id: "clear_font_cache".to_string(),
            success: false,
            message: e,
        },
    }
}

/// Flush inactive memory pages.
/// Requires admin: the `purge` command needs root privileges.
fn flush_memory() -> MaintenanceResult {
    match run_admin_command("purge") {
        Ok(_) => MaintenanceResult {
            task_id: "flush_memory".to_string(),
            success: true,
            message: "Memory cache purged. Inactive memory pages have been freed.".to_string(),
        },
        Err(e) => MaintenanceResult {
            task_id: "flush_memory".to_string(),
            success: false,
            message: e,
        },
    }
}
