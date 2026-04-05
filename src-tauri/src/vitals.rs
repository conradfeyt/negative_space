// vitals.rs — System Vitals for Negative _.
//
// Answers the question: "Why is my Mac hot and what can I do about it?"
//
// Unlike Activity Monitor which shows raw numbers, this module groups processes
// intelligently, identifies the actual CPU hogs, estimates energy impact, and
// provides actionable remediation suggestions.
//
// DATA SOURCES:
//   - `ps -eo pid,ppid,%cpu,rss,comm` — CPU% and memory for all processes
//   - `sysctl hw.ncpu` — number of CPU cores (100% per core)
//   - `sysctl vm.loadavg` — system load averages (1, 5, 15 min)
//   - `uptime` — system uptime
//   - `pmset -g therm` — thermal state (no sudo required)
//   - `NSProcessInfo.thermalState` is ideal but requires ObjC bridge;
//     we parse pmset output instead which is reliable and sudo-free.
//   - LaunchAgents/LaunchDaemons dirs — background startup items
//
// GROUPING STRATEGY:
//   Reuses the memory.rs app-bundle and daemon dictionary patterns but with
//   CPU% as the primary sort. Adds "idle time" heuristic: if a process has
//   high memory but near-zero CPU, it's likely idle and a quit candidate.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Thermal state of the system. Maps to Apple's NSProcessInfoThermalState.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ThermalState {
    /// System is cool, running at full performance.
    Nominal,
    /// System is warm. Performance may be slightly reduced.
    Fair,
    /// System is hot. Performance is significantly reduced.
    Serious,
    /// System is critically hot. Immediate action recommended.
    Critical,
    /// Unable to determine thermal state.
    Unknown,
}

/// A single process with CPU and memory usage.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VitalsProcess {
    /// Process ID
    pub pid: u32,
    /// Parent process ID
    pub ppid: u32,
    /// CPU usage percentage (can exceed 100% on multi-core — each core = 100%)
    pub cpu_percent: f64,
    /// Resident Set Size in bytes
    pub rss_bytes: u64,
    /// Short process name
    pub name: String,
    /// Full command path
    pub command: String,
    /// Human-readable description
    pub description: String,
}

/// A group of related processes, sorted by CPU impact.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VitalsGroup {
    /// Display name (e.g. "Google Chrome", "Spotlight")
    pub name: String,
    /// Category: "app", "system", "developer", "background"
    pub category: String,
    /// Human-readable description
    pub description: String,
    /// Total CPU % across all processes in this group
    pub total_cpu_percent: f64,
    /// Total RSS across all processes
    pub total_rss_bytes: u64,
    /// Number of processes in this group
    pub process_count: u32,
    /// Is this group likely idle? (high memory, <1% CPU)
    pub likely_idle: bool,
    /// Smart remediation suggestion (if any)
    pub suggestion: Option<String>,
    /// Severity of the suggestion: "info", "warning", "critical"
    pub suggestion_severity: Option<String>,
    /// Can the user quit this? (false for system processes)
    pub can_quit: bool,
    /// Individual processes, sorted by CPU descending
    pub processes: Vec<VitalsProcess>,
}

/// System load information.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemLoad {
    /// 1-minute load average
    pub load_1m: f64,
    /// 5-minute load average
    pub load_5m: f64,
    /// 15-minute load average
    pub load_15m: f64,
    /// Number of logical CPU cores
    pub cpu_cores: u32,
    /// Overall CPU usage percentage (0-100, normalized across all cores)
    pub cpu_usage_percent: f64,
    /// System uptime in seconds
    pub uptime_seconds: u64,
    /// Human-readable uptime string
    pub uptime_display: String,
}

/// Battery information. None on desktops without a battery.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BatteryInfo {
    /// Whether a battery is installed
    pub installed: bool,
    /// Current charge percentage (0-100)
    pub charge_percent: u32,
    /// Whether the battery is currently charging
    pub is_charging: bool,
    /// Whether AC power is connected
    pub ac_connected: bool,
    /// Battery health as a percentage of original design capacity (0-100)
    pub health_percent: u32,
    /// Battery cycle count
    pub cycle_count: u32,
    /// Current max capacity in mAh
    pub max_capacity_mah: u32,
    /// Original design capacity in mAh
    pub design_capacity_mah: u32,
    /// Battery temperature in degrees Celsius
    pub temperature_celsius: f64,
    /// Battery condition string: "Normal", "Service Recommended", etc.
    pub condition: String,
    /// Power source: "AC Power", "Battery Power"
    pub power_source: String,
}

/// Full vitals scan result.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VitalsResult {
    /// Current thermal state
    pub thermal_state: ThermalState,
    /// Human-readable thermal description
    pub thermal_description: String,
    /// System load information
    pub load: SystemLoad,
    /// Process groups sorted by total_cpu_percent descending
    pub groups: Vec<VitalsGroup>,
    /// Total number of running processes
    pub total_processes: u32,
    /// Total CPU usage across all processes
    pub total_cpu_percent: f64,
    /// Number of background launch agents/daemons found
    pub background_agent_count: u32,
    /// Headline insight — the one-liner answer to "why is my Mac hot?"
    pub headline: String,
    /// Battery info — None on desktops without a battery
    pub battery: Option<BatteryInfo>,
}

// Process description dictionary and app bundle mappings are shared with
// memory.rs and live in crate::process_info.
use crate::process_info::{get_app_bundle_mappings, get_process_dictionary};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Scan system vitals: thermal state, CPU hogs, memory, load, and generate
/// actionable insights.
pub fn scan_vitals() -> VitalsResult {
    let thermal_state = get_thermal_state();
    let thermal_description = describe_thermal(&thermal_state);
    let load = get_system_load();
    let processes = get_all_processes_with_cpu();
    let total_processes = processes.len() as u32;

    let process_dict = get_process_dictionary();
    let app_mappings = get_app_bundle_mappings();

    // Group processes (same pattern as memory.rs but with CPU data)
    let mut group_map: HashMap<String, Vec<VitalsProcess>> = HashMap::new();
    let mut group_meta: HashMap<String, (String, String, String)> = HashMap::new();

    for proc in &processes {
        let (group_key, group_name, group_category, group_desc, proc_desc) =
            classify_process(proc, &process_dict, &app_mappings);

        let vp = VitalsProcess {
            pid: proc.pid,
            ppid: proc.ppid,
            cpu_percent: proc.cpu_percent,
            rss_bytes: proc.rss_bytes,
            name: proc.name.clone(),
            command: proc.command.clone(),
            description: proc_desc,
        };

        group_map.entry(group_key.clone()).or_default().push(vp);
        group_meta
            .entry(group_key)
            .or_insert_with(|| (group_name, group_category, group_desc));
    }

    // Build VitalsGroup structs with smart suggestions
    let mut groups: Vec<VitalsGroup> = Vec::new();
    let total_cpu: f64 = processes.iter().map(|p| p.cpu_percent).sum();

    for (key, mut procs) in group_map {
        let (name, category, description) = group_meta.remove(&key).unwrap_or_default();

        procs.sort_by(|a, b| {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_group_cpu: f64 = procs.iter().map(|p| p.cpu_percent).sum();
        let total_group_rss: u64 = procs.iter().map(|p| p.rss_bytes).sum();
        let process_count = procs.len() as u32;

        // Idle detection: high memory (>200MB) but barely using CPU (<1%)
        let likely_idle = total_group_rss > 200 * 1024 * 1024 && total_group_cpu < 1.0;

        // Can the user quit this? Not for core system processes.
        let can_quit = category != "system" && name != "Negative _";

        // Generate smart remediation suggestion
        let (suggestion, severity) = generate_suggestion(
            &name,
            &category,
            total_group_cpu,
            total_group_rss,
            process_count,
            likely_idle,
            &thermal_state,
            &load,
        );

        groups.push(VitalsGroup {
            name,
            category,
            description,
            total_cpu_percent: total_group_cpu,
            total_rss_bytes: total_group_rss,
            process_count,
            likely_idle,
            suggestion,
            suggestion_severity: severity,
            can_quit,
            processes: procs,
        });
    }

    // Sort by CPU descending
    groups.sort_by(|a, b| {
        b.total_cpu_percent
            .partial_cmp(&a.total_cpu_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Count background launch agents
    let background_agent_count = count_launch_agents();

    // Generate headline insight
    let headline = generate_headline(&thermal_state, &load, &groups);

    // Battery info (None on desktops)
    let battery = get_battery_info();

    VitalsResult {
        thermal_state,
        thermal_description,
        load,
        groups,
        total_processes,
        total_cpu_percent: total_cpu,
        background_agent_count,
        headline,
        battery,
    }
}

/// Gracefully quit a process (SIGTERM).
pub fn quit_process(pid: u32) -> Result<String, String> {
    let output = std::process::Command::new("/bin/kill")
        .args(["-15", &pid.to_string()])
        .output()
        .map_err(|e| format!("Failed to send signal: {}", e))?;

    if output.status.success() {
        Ok(format!("Sent quit signal to process {}", pid))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to quit process {}: {}", pid, err.trim()))
    }
}

/// Force kill a process (SIGKILL).
pub fn force_quit_process(pid: u32) -> Result<String, String> {
    let output = std::process::Command::new("/bin/kill")
        .args(["-9", &pid.to_string()])
        .output()
        .map_err(|e| format!("Failed to send signal: {}", e))?;

    if output.status.success() {
        Ok(format!("Force quit process {}", pid))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "Failed to force quit process {}: {}",
            pid,
            err.trim()
        ))
    }
}

/// Quit all processes in a group by name (sends SIGTERM to each).
/// Returns (succeeded_count, failed_count).
pub fn quit_group(pids: Vec<u32>) -> (u32, u32) {
    let mut ok = 0u32;
    let mut fail = 0u32;
    for pid in pids {
        if quit_process(pid).is_ok() {
            ok += 1;
        } else {
            fail += 1;
        }
    }
    (ok, fail)
}

// ---------------------------------------------------------------------------
// Thermal state detection
// ---------------------------------------------------------------------------

/// Get the current thermal state.
/// Uses `pmset -g therm` which returns thermal warning levels without sudo.
fn get_thermal_state() -> ThermalState {
    // Try pmset first — use absolute path for bundled .app compatibility.
    // In a sandboxed/bundled macOS app the PATH may be minimal, so we
    // always specify the full path to avoid silent lookup failures.
    let output = std::process::Command::new("/usr/bin/pmset")
        .args(["-g", "therm"])
        .output();

    if let Ok(o) = output {
        if o.status.success() {
            let text = String::from_utf8_lossy(&o.stdout).to_lowercase();
            // pmset output contains "CPU_Speed_Limit" and "CPU_Available_CPUs"
            // When throttling: the speed limit drops below 100.
            // Also look for "thermal warning level" lines.

            // Check for explicit thermal warnings
            if text.contains("cpu_speed_limit") {
                // Parse speed limit
                for line in text.lines() {
                    if line.contains("cpu_speed_limit") {
                        if let Some(val) = extract_number(line) {
                            if val < 50 {
                                return ThermalState::Critical;
                            } else if val < 70 {
                                return ThermalState::Serious;
                            } else if val < 90 {
                                return ThermalState::Fair;
                            }
                            // 90-100 = nominal
                        }
                    }
                }
            }

            // Also check for the warning count
            if text.contains("thermal warning count") {
                // Non-zero warning count suggests thermal concern
                for line in text.lines() {
                    if line.contains("thermal warning count") {
                        if let Some(val) = extract_number(line) {
                            if val > 0 {
                                return ThermalState::Fair;
                            }
                        }
                    }
                }
            }

            return ThermalState::Nominal;
        }
    }

    // Fallback: estimate from CPU load
    // If 1-minute load average exceeds 2x cores, likely hot
    let cores = get_cpu_core_count();
    let load_1m = get_load_averages().0;
    if load_1m > cores as f64 * 2.5 {
        ThermalState::Serious
    } else if load_1m > cores as f64 * 1.5 {
        ThermalState::Fair
    } else {
        ThermalState::Nominal
    }
}

fn describe_thermal(state: &ThermalState) -> String {
    match state {
        ThermalState::Nominal => "System is cool and running at full performance.".to_string(),
        ThermalState::Fair => {
            "System is warm. Performance may be slightly reduced to manage heat.".to_string()
        }
        ThermalState::Serious => {
            "System is running hot. macOS is throttling CPU to reduce temperature.".to_string()
        }
        ThermalState::Critical => {
            "System is critically hot. Significant performance throttling active.".to_string()
        }
        ThermalState::Unknown => "Unable to determine thermal state.".to_string(),
    }
}

// ---------------------------------------------------------------------------
// System load
// ---------------------------------------------------------------------------

fn get_system_load() -> SystemLoad {
    let cores = get_cpu_core_count();
    let (l1, l5, l15) = get_load_averages();

    // CPU usage estimate: load_1m / cores * 100 (capped at 100)
    let cpu_usage = ((l1 / cores as f64) * 100.0).min(100.0).max(0.0);

    let (uptime_secs, uptime_display) = get_uptime();

    SystemLoad {
        load_1m: l1,
        load_5m: l5,
        load_15m: l15,
        cpu_cores: cores,
        cpu_usage_percent: cpu_usage,
        uptime_seconds: uptime_secs,
        uptime_display,
    }
}

fn get_cpu_core_count() -> u32 {
    // Absolute path: sysctl lives at /usr/sbin/sysctl which may not be
    // on PATH inside a bundled .app.
    let output = std::process::Command::new("/usr/sbin/sysctl")
        .args(["-n", "hw.ncpu"])
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            text.trim().parse::<u32>().unwrap_or_else(|_| {
                eprintln!("[vitals] hw.ncpu parse failed, raw: {:?}", text.trim());
                8
            })
        }
        Ok(o) => {
            eprintln!(
                "[vitals] sysctl hw.ncpu failed: status={}, stderr={:?}",
                o.status,
                String::from_utf8_lossy(&o.stderr).trim()
            );
            8
        }
        Err(e) => {
            eprintln!("[vitals] sysctl hw.ncpu exec error: {}", e);
            8
        }
    }
}

fn get_load_averages() -> (f64, f64, f64) {
    // Absolute path — /usr/sbin/sysctl may not be on PATH in bundled .app
    // LC_NUMERIC=C forces dot decimal separators regardless of user locale.
    let output = std::process::Command::new("/usr/sbin/sysctl")
        .args(["-n", "vm.loadavg"])
        .env("LC_NUMERIC", "C")
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            // Format: "{ 2.34 1.56 1.12 }" or "{ 2,34 1,56 1,12 }" (locale-dependent)
            let nums: Vec<f64> = text
                .trim()
                .trim_matches(|c| c == '{' || c == '}')
                .split_whitespace()
                .filter_map(|s| s.replace(',', ".").parse().ok())
                .collect();
            if nums.len() >= 3 {
                (nums[0], nums[1], nums[2])
            } else {
                eprintln!("[vitals] vm.loadavg unexpected format: {:?}", text.trim());
                (0.0, 0.0, 0.0)
            }
        }
        Ok(o) => {
            eprintln!(
                "[vitals] sysctl vm.loadavg failed: status={}, stderr={:?}",
                o.status,
                String::from_utf8_lossy(&o.stderr).trim()
            );
            (0.0, 0.0, 0.0)
        }
        Err(e) => {
            eprintln!("[vitals] sysctl vm.loadavg exec error: {}", e);
            (0.0, 0.0, 0.0)
        }
    }
}

fn get_uptime() -> (u64, String) {
    // sysctl kern.boottime returns a struct; easier to parse `uptime` command
    // Absolute paths for bundled .app compatibility
    let output = std::process::Command::new("/usr/bin/uptime").output();

    if let Ok(o) = output {
        if o.status.success() {
            let text = String::from_utf8_lossy(&o.stdout);
            // Parse the human-readable part and estimate seconds
            let display = parse_uptime_display(&text);

            // For seconds, use kern.boottime
            let boot_output = std::process::Command::new("/usr/sbin/sysctl")
                .args(["-n", "kern.boottime"])
                .output();
            if let Ok(bo) = boot_output {
                if bo.status.success() {
                    let bt_text = String::from_utf8_lossy(&bo.stdout);
                    // Format: "{ sec = 1234567890, usec = 123456 }"
                    if let Some(sec_str) = bt_text.split("sec = ").nth(1) {
                        if let Some(sec_end) = sec_str.find(',') {
                            if let Ok(boot_sec) = sec_str[..sec_end].trim().parse::<u64>() {
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs();
                                return (now.saturating_sub(boot_sec), display);
                            }
                        }
                    }
                }
            }

            return (0, display);
        }
    }

    (0, "Unknown".to_string())
}

fn parse_uptime_display(text: &str) -> String {
    // uptime output: " 22:15  up 14 days,  3:42, 4 users, load averages: 2.34 1.56 1.12"
    // We want: "14 days, 3:42"
    if let Some(up_idx) = text.find("up ") {
        let after_up = &text[up_idx + 3..];
        // Find the part before "user" or "load"
        let end = after_up
            .find(" user")
            .or_else(|| after_up.find(" load"))
            .unwrap_or(after_up.len());
        let uptime_part = after_up[..end].trim().trim_end_matches(',').trim();

        // Clean up: "14 days,  3:42" -> "14 days, 3h 42m"
        let parts: Vec<&str> = uptime_part.split(',').map(|s| s.trim()).collect();
        let mut result = Vec::new();
        for part in parts {
            if part.contains(':') {
                // "3:42" -> "3h 42m"
                let hm: Vec<&str> = part.split(':').collect();
                if hm.len() == 2 {
                    let h = hm[0].trim();
                    let m = hm[1].trim();
                    if h != "0" {
                        result.push(format!("{}h {}m", h, m));
                    } else {
                        result.push(format!("{}m", m));
                    }
                } else {
                    result.push(part.to_string());
                }
            } else {
                result.push(part.to_string());
            }
        }

        return result.join(", ");
    }
    "Unknown".to_string()
}

// ---------------------------------------------------------------------------
// Process scanning (with CPU%)
// ---------------------------------------------------------------------------

struct RawProcess {
    pid: u32,
    ppid: u32,
    cpu_percent: f64,
    rss_bytes: u64,
    name: String,
    command: String,
}

fn get_all_processes_with_cpu() -> Vec<RawProcess> {
    // ps -eo pid,ppid,%cpu,rss,comm — includes CPU% per process.
    // Use absolute path /bin/ps for bundled .app compatibility.
    // LC_NUMERIC=C forces dot decimal separators regardless of user locale.
    let output = std::process::Command::new("/bin/ps")
        .args(["-eo", "pid,ppid,%cpu,rss,comm"])
        .env("LC_NUMERIC", "C")
        .output();

    let text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        Ok(o) => {
            eprintln!(
                "[vitals] ps failed: status={}, stderr={:?}",
                o.status,
                String::from_utf8_lossy(&o.stderr).trim()
            );
            return vec![];
        }
        Err(e) => {
            eprintln!("[vitals] ps exec error: {}", e);
            return vec![];
        }
    };

    let mut processes = Vec::new();
    let line_count = text.lines().count();
    if line_count <= 1 {
        eprintln!(
            "[vitals] ps returned only {} lines (header only or empty)",
            line_count
        );
    }

    for line in text.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        let pid: u32 = match parts[0].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ppid: u32 = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let cpu: f64 = match parts[2].replace(',', ".").parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let rss_kb: u64 = match parts[3].replace(',', ".").parse::<f64>() {
            Ok(v) => v as u64,
            Err(_) => continue,
        };

        let comm = parts[4..].join(" ");
        let name = comm.rsplit('/').next().unwrap_or(&comm).to_string();

        processes.push(RawProcess {
            pid,
            ppid,
            cpu_percent: cpu,
            rss_bytes: rss_kb * 1024,
            name,
            command: comm,
        });
    }

    processes
}

// ---------------------------------------------------------------------------
// Process classification (shared pattern with memory.rs)
// ---------------------------------------------------------------------------

fn classify_process(
    proc: &RawProcess,
    dict: &HashMap<&str, (&str, &str)>,
    app_mappings: &[(&str, &str, &str)],
) -> (String, String, String, String, String) {
    // 1. App bundle matching
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

    // 2. Known daemon dictionary
    if let Some((desc, category)) = dict.get(proc.name.as_str()) {
        return (
            format!("sys:{}", category),
            "macOS System".to_string(),
            "system".to_string(),
            "Core macOS services and daemons".to_string(),
            desc.to_string(),
        );
    }

    // 3. Looks like an app (capitalized name, no dots, >1MB)
    if proc
        .name
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
        && !proc.name.contains('.')
        && proc.rss_bytes > 1024 * 1024
    {
        return (
            format!("app:{}", proc.name),
            proc.name.clone(),
            "app".to_string(),
            String::new(),
            String::new(),
        );
    }

    // 4. com.apple.* = system
    if proc.command.starts_with("com.apple.") || proc.name.starts_with("com.apple.") {
        return (
            "sys:system".to_string(),
            "macOS System".to_string(),
            "system".to_string(),
            "Core macOS services and daemons".to_string(),
            format!(
                "Apple system service ({})",
                proc.name.trim_start_matches("com.apple.")
            ),
        );
    }

    // 5. Other
    (
        "other".to_string(),
        "Other Processes".to_string(),
        "background".to_string(),
        "Background processes and daemons".to_string(),
        String::new(),
    )
}

// ---------------------------------------------------------------------------
// Smart remediation suggestions
// ---------------------------------------------------------------------------

fn generate_suggestion(
    name: &str,
    category: &str,
    cpu: f64,
    rss: u64,
    process_count: u32,
    idle: bool,
    thermal: &ThermalState,
    _load: &SystemLoad,
) -> (Option<String>, Option<String>) {
    let rss_gb = rss as f64 / (1024.0 * 1024.0 * 1024.0);
    let rss_mb = rss as f64 / (1024.0 * 1024.0);

    // Don't suggest anything for system processes
    if category == "system" {
        if cpu > 30.0 && name.contains("System") {
            return (
                Some("High system CPU is often caused by Spotlight indexing, Time Machine, or macOS updates. It usually resolves on its own.".to_string()),
                Some("info".to_string()),
            );
        }
        return (None, None);
    }

    // Critical CPU hog (>50% CPU)
    if cpu > 50.0 {
        let msg = if *thermal == ThermalState::Serious || *thermal == ThermalState::Critical {
            format!("{} is using {:.0}% CPU and is a primary contributor to your Mac overheating. Consider quitting it.", name, cpu)
        } else {
            format!(
                "{} is using {:.0}% CPU. Quitting it would significantly reduce system load.",
                name, cpu
            )
        };
        return (Some(msg), Some("critical".to_string()));
    }

    // High CPU (>20%)
    if cpu > 20.0 {
        let msg = format!(
            "{} is using {:.0}% CPU across {} process{}.",
            name,
            cpu,
            process_count,
            if process_count > 1 { "es" } else { "" }
        );
        return (Some(msg), Some("warning".to_string()));
    }

    // Browsers with many processes (likely many tabs)
    if (name.contains("Chrome")
        || name.contains("Edge")
        || name.contains("Brave")
        || name.contains("Firefox")
        || name.contains("Arc"))
        && process_count > 10
    {
        let msg = format!("{} has {} processes running (likely many open tabs). Closing unused tabs would free memory and reduce CPU.", name, process_count);
        return (Some(msg), Some("warning".to_string()));
    }

    // High memory idle app (>1GB, <1% CPU)
    if idle && rss_gb > 1.0 {
        let msg = format!(
            "{} is using {:.1} GB of memory but appears idle. Quitting it would free up RAM.",
            name, rss_gb
        );
        return (Some(msg), Some("info".to_string()));
    }

    // Moderate memory idle app (>200MB, <1% CPU)
    if idle && rss_mb > 200.0 {
        let msg = format!("{} is idle but using {:.0} MB of memory.", name, rss_mb);
        return (Some(msg), Some("info".to_string()));
    }

    // Docker specifically
    if name.contains("Docker") && rss_gb > 2.0 {
        let msg = format!("{} is using {:.1} GB of memory. If you're not actively using containers, quitting Docker Desktop would reclaim significant resources.", name, rss_gb);
        return (Some(msg), Some("warning".to_string()));
    }

    // Electron apps using lots of memory
    if process_count > 5 && rss_mb > 500.0 && category == "app" {
        let msg = format!(
            "{} (Electron app) has {} helper processes using {:.0} MB total.",
            name, process_count, rss_mb
        );
        return (Some(msg), Some("info".to_string()));
    }

    (None, None)
}

// ---------------------------------------------------------------------------
// Headline generation — the answer to "why is my Mac hot?"
// ---------------------------------------------------------------------------

fn generate_headline(thermal: &ThermalState, load: &SystemLoad, groups: &[VitalsGroup]) -> String {
    // Find the top CPU consumer
    let top = groups.first();

    match thermal {
        ThermalState::Critical => {
            if let Some(g) = top {
                format!(
                    "Your Mac is critically hot. {} ({:.0}% CPU) is the primary cause.",
                    g.name, g.total_cpu_percent
                )
            } else {
                "Your Mac is critically hot. Close resource-heavy apps to cool down.".to_string()
            }
        }
        ThermalState::Serious => {
            if let Some(g) = top {
                if g.total_cpu_percent > 30.0 {
                    format!(
                        "Running hot. {} is using {:.0}% CPU and driving up temperature.",
                        g.name, g.total_cpu_percent
                    )
                } else {
                    "Running hot, but no single app stands out. Combined load is heating your Mac."
                        .to_string()
                }
            } else {
                "Running hot. Consider closing some applications.".to_string()
            }
        }
        ThermalState::Fair => {
            if let Some(g) = top {
                if g.total_cpu_percent > 20.0 {
                    format!(
                        "System is warm. {} is the heaviest app at {:.0}% CPU.",
                        g.name, g.total_cpu_percent
                    )
                } else {
                    "System is warm but well within normal range.".to_string()
                }
            } else {
                "System is warm but running fine.".to_string()
            }
        }
        ThermalState::Nominal | ThermalState::Unknown => {
            if load.cpu_usage_percent > 80.0 {
                if let Some(g) = top {
                    format!(
                        "CPU is busy ({:.0}% load). {} is using the most at {:.0}% CPU.",
                        load.cpu_usage_percent, g.name, g.total_cpu_percent
                    )
                } else {
                    format!("CPU is busy at {:.0}% load.", load.cpu_usage_percent)
                }
            } else if load.cpu_usage_percent > 50.0 {
                "Moderate CPU activity. System is running normally.".to_string()
            } else {
                let idle_hogs: Vec<&VitalsGroup> = groups
                    .iter()
                    .filter(|g| g.likely_idle && g.total_rss_bytes > 500 * 1024 * 1024)
                    .collect();
                if !idle_hogs.is_empty() {
                    let names: Vec<&str> =
                        idle_hogs.iter().map(|g| g.name.as_str()).take(3).collect();
                    let ram: u64 = idle_hogs.iter().map(|g| g.total_rss_bytes).sum();
                    let ram_gb = ram as f64 / (1024.0 * 1024.0 * 1024.0);
                    format!(
                        "System is cool. {} idle app{} using {:.1} GB of RAM: {}.",
                        idle_hogs.len(),
                        if idle_hogs.len() > 1 { "s" } else { "" },
                        ram_gb,
                        names.join(", ")
                    )
                } else {
                    "System is cool and running smoothly.".to_string()
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Background agents count
// ---------------------------------------------------------------------------

fn count_launch_agents() -> u32 {
    let mut count = 0u32;

    // User launch agents
    if let Some(home) = dirs_home() {
        let user_agents = format!("{}/Library/LaunchAgents", home);
        if let Ok(entries) = std::fs::read_dir(&user_agents) {
            count += entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|x| x == "plist")
                        .unwrap_or(false)
                })
                .count() as u32;
        }
    }

    count
}

fn dirs_home() -> Option<String> {
    std::env::var("HOME").ok()
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

fn extract_number(line: &str) -> Option<u64> {
    // Find the last number in a line
    line.split_whitespace()
        .filter_map(|w| w.trim_end_matches('%').parse::<u64>().ok())
        .last()
}

// ---------------------------------------------------------------------------
// Battery
// ---------------------------------------------------------------------------

/// Read battery info from ioreg. Returns None if no battery is installed
/// (e.g. on a desktop Mac).
fn get_battery_info() -> Option<BatteryInfo> {
    let output = std::process::Command::new("/usr/sbin/ioreg")
        .args(["-rn", "AppleSmartBattery"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);

    // If ioreg returns nothing meaningful, no battery present.
    if !text.contains("BatteryInstalled") {
        return None;
    }

    let installed = ioreg_bool(&text, "BatteryInstalled");
    if !installed {
        return None;
    }

    let charge_percent = ioreg_u32(&text, "CurrentCapacity").unwrap_or(0);
    let is_charging = ioreg_bool(&text, "IsCharging");
    let ac_connected = ioreg_bool(&text, "ExternalConnected");
    let cycle_count = ioreg_u32(&text, "CycleCount").unwrap_or(0);
    let design_capacity_mah = ioreg_u32(&text, "DesignCapacity").unwrap_or(0);
    let raw_max = ioreg_u32(&text, "AppleRawMaxCapacity").unwrap_or(0);
    let temperature_raw = ioreg_u32(&text, "Temperature").unwrap_or(0);
    let temperature_celsius = temperature_raw as f64 / 100.0;

    // Health: current max capacity vs original design capacity
    let health_percent = if design_capacity_mah > 0 && raw_max > 0 {
        ((raw_max as f64 / design_capacity_mah as f64) * 100.0).round() as u32
    } else {
        100
    };

    // Condition from system_profiler (slower but gives Apple's official string).
    // We cache-friendly: only call once per scan cycle since vitals refreshes every 3s.
    let condition = get_battery_condition();

    let power_source = if ac_connected {
        "AC Power".to_string()
    } else {
        "Battery Power".to_string()
    };

    Some(BatteryInfo {
        installed,
        charge_percent,
        is_charging,
        ac_connected,
        health_percent,
        cycle_count,
        max_capacity_mah: raw_max,
        design_capacity_mah,
        temperature_celsius,
        condition,
        power_source,
    })
}

/// Extract a u32 value from ioreg output for a given key.
/// Matches lines like `"KeyName" = 1234`
fn ioreg_u32(text: &str, key: &str) -> Option<u32> {
    let pattern = format!("\"{}\" = ", key);
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&pattern) {
            return rest.trim().parse().ok();
        }
        // Also handle the format with leading whitespace and quotes
        if trimmed.contains(&pattern) {
            if let Some(pos) = trimmed.find(&pattern) {
                let after = &trimmed[pos + pattern.len()..];
                return after.trim().parse().ok();
            }
        }
    }
    None
}

/// Extract a boolean from ioreg output ("Yes"/"No").
fn ioreg_bool(text: &str, key: &str) -> bool {
    let pattern = format!("\"{}\" = ", key);
    for line in text.lines() {
        if line.contains(&pattern) {
            return line.contains("Yes");
        }
    }
    false
}

/// Get battery condition from system_profiler (cached — it rarely changes).
/// Returns "Normal", "Service Recommended", "Replace Soon", "Replace Now", or "Unknown".
fn get_battery_condition() -> String {
    use std::sync::Mutex;
    use std::time::Instant;

    static CACHE: Mutex<Option<(String, Instant)>> = Mutex::new(None);
    const CACHE_TTL_SECS: u64 = 300; // 5 minutes

    if let Ok(guard) = CACHE.lock() {
        if let Some((ref cached, ref when)) = *guard {
            if when.elapsed().as_secs() < CACHE_TTL_SECS {
                return cached.clone();
            }
        }
    }

    let result = match std::process::Command::new("/usr/sbin/system_profiler")
        .args(["SPPowerDataType"])
        .output()
    {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            text.lines()
                .find(|l| l.contains("Condition:"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        }
        _ => "Unknown".to_string(),
    };

    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some((result.clone(), Instant::now()));
    }

    result
}
