// memory.rs — Memory usage analysis for Negative _.
//
// Groups running processes by application and provides human-readable
// descriptions for system daemons and services. Solves the problem of
// Activity Monitor showing 20+ separate Chrome helper processes or
// cryptic names like "mds_stores" without explanation.
//
// DATA SOURCE:
//   We use `ps -eo pid,ppid,rss,%mem,comm` to get all processes with their
//   parent-child relationships and memory usage. RSS (Resident Set Size) is
//   the most meaningful single memory metric — it's the actual physical RAM
//   the process is using right now.
//
//   We also run `sysctl hw.memsize` to get total physical RAM, and parse
//   `vm_stat` for a breakdown of memory pages (free, active, inactive, wired).
//
// GROUPING STRATEGY:
//   1. Parse all processes from ps output
//   2. Match each process to a "group" using these rules (in order):
//      a. Known app bundles: if the binary path contains a .app bundle name,
//         group under that app (e.g. all Chrome helpers -> "Google Chrome")
//      b. Parent-child: if a process's parent is already in a group, join it
//      c. Known system categories: match against a dictionary of macOS daemon
//         names and group into categories (Spotlight, Networking, etc.)
//      d. Fallback: group as "Other system processes"
//   3. For each group, sum RSS to get total memory usage
//   4. Return groups sorted by total memory descending

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A single process with its memory usage.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessInfo {
    /// Process ID
    pub pid: u32,
    /// Parent process ID
    pub ppid: u32,
    /// Resident Set Size in bytes (actual physical RAM used)
    pub rss_bytes: u64,
    /// Percentage of total memory used by this process
    pub mem_percent: f64,
    /// Short process name (last component of the command path)
    pub name: String,
    /// Full command path
    pub command: String,
    /// Human-readable description of what this process does
    pub description: String,
}

/// A group of related processes (e.g. all Chrome helpers, all Spotlight daemons).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessGroup {
    /// Group display name (e.g. "Google Chrome", "Spotlight", "Networking")
    pub name: String,
    /// Group category for UI coloring: "app", "system", "developer", "background"
    pub category: String,
    /// Human-readable description of the group
    pub description: String,
    /// Total RSS across all processes in this group
    pub total_rss_bytes: u64,
    /// Percentage of total system memory used by this group
    pub total_mem_percent: f64,
    /// Number of processes in this group
    pub process_count: u32,
    /// Individual processes in this group, sorted by RSS descending
    pub processes: Vec<ProcessInfo>,
}

/// System-wide memory statistics.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemoryStats {
    /// Total physical RAM in bytes
    pub total_bytes: u64,
    /// Currently used memory (active + wired) in bytes
    pub used_bytes: u64,
    /// Active memory (recently used) in bytes
    pub active_bytes: u64,
    /// Inactive memory (not recently used, available for reuse) in bytes
    pub inactive_bytes: u64,
    /// Wired memory (kernel, can't be paged out) in bytes
    pub wired_bytes: u64,
    /// Free memory in bytes
    pub free_bytes: u64,
    /// Compressed memory in bytes
    pub compressed_bytes: u64,
    /// App memory (active - wired ≈ what apps are using)
    pub app_bytes: u64,
}

/// Full result of a memory scan.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemoryScanResult {
    /// System-wide memory statistics
    pub stats: MemoryStats,
    /// Process groups sorted by total_rss_bytes descending
    pub groups: Vec<ProcessGroup>,
    /// Total number of processes running
    pub total_processes: u32,
}

// Process description dictionary and app bundle mappings are shared with
// vitals.rs and live in crate::process_info.
use crate::process_info::{get_app_bundle_mappings, get_process_dictionary};

// ---------------------------------------------------------------------------
// System category groupings
// ---------------------------------------------------------------------------

fn get_system_category_name(category: &str) -> &str {
    match category {
        "system" => "macOS System",
        "display" => "Display & UI",
        "networking" => "Networking",
        "security" => "Security & Privacy",
        "storage" => "Storage & Indexing",
        "icloud" => "iCloud Services",
        "audio" => "Audio",
        "input" => "Input Devices",
        "developer" => "Developer Tools",
        "app" => "Applications",
        "background" => "Background Services",
        _ => "Other",
    }
}

fn get_system_category_description(category: &str) -> &str {
    match category {
        "system" => "Core macOS services, daemons, and process management",
        "display" => "Window compositing, display management, and UI rendering",
        "networking" => "DNS resolution, Wi-Fi, Bluetooth, AirDrop, and network services",
        "security" => "Keychain, certificate validation, malware scanning, and authentication",
        "storage" => "Spotlight indexing, filesystem events, disk management, and Time Machine",
        "icloud" => "iCloud Drive, Photos, Keychain, and cross-device sync",
        "audio" => "Audio input/output, media controls, and audio routing",
        "input" => "Keyboard, trackpad, Touch Bar, and input processing",
        "developer" => "Compilers, language servers, build tools, and runtime environments",
        "app" => "User-installed applications",
        "background" => "Background daemons and agents",
        _ => "Other processes",
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Scan running processes, group them, and return memory usage analysis.
pub fn scan_memory() -> MemoryScanResult {
    let stats = get_memory_stats();
    let processes = get_all_processes();
    let total_processes = processes.len() as u32;
    let total_mem = stats.total_bytes;

    let process_dict = get_process_dictionary();
    let app_mappings = get_app_bundle_mappings();

    // Step 1: Assign each process to a group.
    // group_key -> Vec<ProcessInfo>
    let mut group_map: HashMap<String, Vec<ProcessInfo>> = HashMap::new();
    // group_key -> (name, category, description)
    let mut group_meta: HashMap<String, (String, String, String)> = HashMap::new();

    for proc in &processes {
        let (group_key, group_name, group_category, group_desc, proc_desc) =
            classify_process(proc, &process_dict, &app_mappings);

        let proc_info = ProcessInfo {
            pid: proc.pid,
            ppid: proc.ppid,
            rss_bytes: proc.rss_bytes,
            mem_percent: if total_mem > 0 {
                (proc.rss_bytes as f64 / total_mem as f64) * 100.0
            } else {
                0.0
            },
            name: proc.name.clone(),
            command: proc.command.clone(),
            description: proc_desc,
        };

        group_map
            .entry(group_key.clone())
            .or_default()
            .push(proc_info);
        group_meta
            .entry(group_key)
            .or_insert_with(|| (group_name, group_category, group_desc));
    }

    // Step 2: Build ProcessGroup structs.
    let mut groups: Vec<ProcessGroup> = Vec::new();

    for (key, mut procs) in group_map {
        let (name, category, description) = group_meta.remove(&key).unwrap_or_default();

        // Sort processes within group by RSS descending.
        procs.sort_by(|a, b| b.rss_bytes.cmp(&a.rss_bytes));

        let total_rss: u64 = procs.iter().map(|p| p.rss_bytes).sum();
        let total_pct = if total_mem > 0 {
            (total_rss as f64 / total_mem as f64) * 100.0
        } else {
            0.0
        };

        groups.push(ProcessGroup {
            name,
            category,
            description,
            total_rss_bytes: total_rss,
            total_mem_percent: total_pct,
            process_count: procs.len() as u32,
            processes: procs,
        });
    }

    // Sort groups by total RSS descending.
    groups.sort_by(|a, b| b.total_rss_bytes.cmp(&a.total_rss_bytes));

    MemoryScanResult {
        stats,
        groups,
        total_processes,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Raw process data from ps.
struct RawProcess {
    pid: u32,
    ppid: u32,
    rss_bytes: u64,
    name: String,
    command: String,
}

/// Parse all processes from `ps` output.
fn get_all_processes() -> Vec<RawProcess> {
    // ps -eo pid,ppid,rss,comm
    // PID: process ID
    // PPID: parent process ID
    // RSS: resident set size in KB
    // COMM: full command path
    // Absolute path for bundled .app compatibility — /bin/ps is always
    // present on macOS regardless of PATH environment.
    let output = std::process::Command::new("/bin/ps")
        .args(["-eo", "pid,ppid,rss,comm"])
        .output();

    let text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return vec![],
    };

    let mut processes = Vec::new();

    for line in text.lines().skip(1) {
        // Skip the header line
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Format: "  PID  PPID   RSS COMM"
        // Split by whitespace, but COMM may contain spaces.
        let parts: Vec<&str> = trimmed.splitn(4, char::is_whitespace).collect();
        if parts.len() < 4 {
            continue;
        }

        // Filter out empty parts from multiple spaces
        let non_empty: Vec<&str> = trimmed.split_whitespace().collect();
        if non_empty.len() < 4 {
            continue;
        }

        let pid: u32 = match non_empty[0].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ppid: u32 = match non_empty[1].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let rss_kb: u64 = match non_empty[2].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        // COMM is everything after the third column. Rejoin from index 3 onward.
        let comm = non_empty[3..].join(" ");
        let name = comm.rsplit('/').next().unwrap_or(&comm).to_string();

        processes.push(RawProcess {
            pid,
            ppid,
            rss_bytes: rss_kb * 1024,
            name,
            command: comm,
        });
    }

    processes
}

/// Classify a process into a group.
/// Returns (group_key, group_name, group_category, group_description, process_description).
fn classify_process(
    proc: &RawProcess,
    dict: &HashMap<&str, (&str, &str)>,
    app_mappings: &[(&str, &str, &str)],
) -> (String, String, String, String, String) {
    // 1. Check app bundle mappings first (command path contains app name).
    for (path_contains, group_name, app_desc) in app_mappings {
        if proc.command.contains(path_contains) {
            let proc_desc = dict
                .get(proc.name.as_str())
                .map(|(d, _)| d.to_string())
                .unwrap_or_else(|| format!("Part of {}", group_name));

            return (
                format!("app:{}", group_name),
                group_name.to_string(),
                "app".to_string(),
                app_desc.to_string(),
                proc_desc,
            );
        }
    }

    // 2. Check the process dictionary for known daemons/services.
    if let Some((desc, category)) = dict.get(proc.name.as_str()) {
        let group_name = get_system_category_name(category).to_string();
        let group_desc = get_system_category_description(category).to_string();

        return (
            format!("sys:{}", category),
            group_name,
            category.to_string(),
            group_desc,
            desc.to_string(),
        );
    }

    // 3. If the process name looks like an app (starts with uppercase, no dots),
    //    treat it as a standalone app.
    if proc
        .name
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
        && !proc.name.contains('.')
        && proc.rss_bytes > 1024 * 1024
    // > 1MB to avoid tiny helpers
    {
        return (
            format!("app:{}", proc.name),
            proc.name.clone(),
            "app".to_string(),
            String::new(),
            String::new(),
        );
    }

    // 4. If command starts with com.apple, group as system.
    if proc.command.starts_with("com.apple.") || proc.name.starts_with("com.apple.") {
        let short_name = proc.name.trim_start_matches("com.apple.").to_string();
        return (
            "sys:system".to_string(),
            "macOS System".to_string(),
            "system".to_string(),
            "Core macOS services, daemons, and process management".to_string(),
            format!("Apple system service ({})", short_name),
        );
    }

    // 5. Fallback: group as "Other".
    (
        "other".to_string(),
        "Other Processes".to_string(),
        "background".to_string(),
        "Background processes and daemons not matching any known category".to_string(),
        String::new(),
    )
}

/// Get system-wide memory statistics.
fn get_memory_stats() -> MemoryStats {
    // Total physical RAM from sysctl.
    let total_bytes = get_sysctl_value("hw.memsize");

    // vm_stat gives page-level memory breakdown.
    // Absolute path for bundled .app compatibility
    let output = std::process::Command::new("/usr/bin/vm_stat").output();

    let text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => {
            return MemoryStats {
                total_bytes,
                used_bytes: 0,
                active_bytes: 0,
                inactive_bytes: 0,
                wired_bytes: 0,
                free_bytes: 0,
                compressed_bytes: 0,
                app_bytes: 0,
            };
        }
    };

    // vm_stat output format:
    // Mach Virtual Memory Statistics: (page size of 16384 bytes)
    // Pages free:                              12345.
    // Pages active:                            67890.
    // Pages inactive:                          11111.
    // Pages speculative:                       22222.
    // Pages wired down:                        33333.
    // ...
    // Pages occupied by compressor:            44444.

    let page_size = parse_vm_stat_page_size(&text);
    let free_pages = parse_vm_stat_field(&text, "Pages free");
    let active_pages = parse_vm_stat_field(&text, "Pages active");
    let inactive_pages = parse_vm_stat_field(&text, "Pages inactive");
    let wired_pages = parse_vm_stat_field(&text, "Pages wired down");
    let compressed_pages = parse_vm_stat_field(&text, "Pages occupied by compressor");
    let speculative_pages = parse_vm_stat_field(&text, "Pages speculative");

    let free_bytes = (free_pages + speculative_pages) * page_size;
    let active_bytes = active_pages * page_size;
    let inactive_bytes = inactive_pages * page_size;
    let wired_bytes = wired_pages * page_size;
    let compressed_bytes = compressed_pages * page_size;
    let used_bytes = active_bytes + wired_bytes;
    let app_bytes = if active_bytes > wired_bytes {
        active_bytes - wired_bytes
    } else {
        active_bytes
    };

    MemoryStats {
        total_bytes,
        used_bytes,
        active_bytes,
        inactive_bytes,
        wired_bytes,
        free_bytes,
        compressed_bytes,
        app_bytes,
    }
}

/// Parse `sysctl hw.memsize` to get total physical RAM in bytes.
fn get_sysctl_value(key: &str) -> u64 {
    // Absolute path for bundled .app compatibility
    let output = std::process::Command::new("/usr/sbin/sysctl")
        .args(["-n", key])
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            text.trim().parse::<u64>().unwrap_or(0)
        }
        _ => 0,
    }
}

/// Parse the page size from vm_stat header.
fn parse_vm_stat_page_size(text: &str) -> u64 {
    // "Mach Virtual Memory Statistics: (page size of 16384 bytes)"
    if let Some(line) = text.lines().next() {
        if let Some(start) = line.find("page size of ") {
            let after = &line[start + 13..];
            if let Some(end) = after.find(' ') {
                return after[..end].parse::<u64>().unwrap_or(16384);
            }
        }
    }
    16384 // Default page size on Apple Silicon
}

/// Parse a field from vm_stat output. Returns the page count.
fn parse_vm_stat_field(text: &str, field_name: &str) -> u64 {
    for line in text.lines() {
        if line.starts_with(field_name) {
            // Line format: "Pages free:                     12345."
            if let Some(colon_pos) = line.find(':') {
                let value_str = line[colon_pos + 1..].trim().trim_end_matches('.');
                return value_str.parse::<u64>().unwrap_or(0);
            }
        }
    }
    0
}
