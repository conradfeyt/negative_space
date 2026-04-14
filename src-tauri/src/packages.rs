// packages.rs — Detect installed package managers, their packages, and runtimes.
//
// This module scans the system for:
//   1. Package managers (Homebrew, MacPorts, Nix, Conda, pip, npm, cargo, gem, etc.)
//   2. Runtimes & version managers (Java, Node/nvm, Rust/rustup, Go, Flutter, Python, etc.)
//
// Detection is done via subprocess calls (`which`, `brew list`, etc.) and
// directory checks (`~/.nvm`, `~/.cargo`, etc.). All commands are non-destructive
// and read-only.
//
// FUTURE: MacPorts, Nix, Conda, mise, asdf, Composer, gem, R, Lua, .NET, PHP,
//         pyenv, rbenv, rvm, sdkman, volta, fnm.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A detected package manager (e.g. Homebrew, pip, npm).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PackageManagerInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub install_path: String,
    pub total_size: u64,
    pub packages: Vec<InstalledPackage>,
    pub total_package_count: usize,
    pub detected: bool,
    pub uninstall_hint: String,
    #[serde(default)]
    pub is_custom: bool,
    #[serde(default)]
    pub commands_run: Vec<CommandRecord>,
}

/// A single installed package.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub size: u64,
    pub is_top_level: bool,
    pub dependencies: Vec<String>,
    pub uninstall_command: String,
    pub removal_warning: String,
}

/// A detected runtime or language installation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RuntimeInfo {
    pub id: String,
    pub name: String,
    pub install_method: String,
    pub install_path: String,
    pub total_size: u64,
    pub versions: Vec<RuntimeVersion>,
    pub uninstall_hint: String,
    pub removal_warning: String,
    #[serde(default)]
    pub is_custom: bool,
    #[serde(default)]
    pub commands_run: Vec<CommandRecord>,
}

/// A specific version of a runtime.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RuntimeVersion {
    pub version: String,
    pub active: bool,
    pub path: String,
    pub size: u64,
}

/// Complete scan result returned to the frontend.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PackageScanResult {
    pub managers: Vec<PackageManagerInfo>,
    pub runtimes: Vec<RuntimeInfo>,
    pub total_size: u64,
}

// ---------------------------------------------------------------------------
// Command tracing — records what ran during a scan
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommandRecord {
    pub program: String,
    pub args: Vec<String>,
    pub purpose: String,
    pub success: bool,
    pub duration_ms: u64,
    pub output_preview: String,
}

// ---------------------------------------------------------------------------
// Custom probes — user-defined package/runtime checks
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProbeCommand {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeType {
    Manager,
    Runtime,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ParseMode {
    Lines,
    Json,
    None,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CustomProbe {
    pub id: String,
    pub name: String,
    pub probe_type: ProbeType,
    pub enabled: bool,
    pub detect: ProbeCommand,
    pub version: Option<ProbeCommand>,
    pub list_packages: Option<ProbeCommand>,
    pub list_parse_mode: ParseMode,
    pub size_paths: Vec<String>,
    pub install_path: String,
    pub uninstall_hint: String,
}

fn probes_file_path() -> Result<String, String> {
    let home = crate::commands::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?;
    let dir = format!("{}/Library/Application Support/NegativeSpace", home);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;
    Ok(format!("{}/custom-probes.json", dir))
}

pub fn load_custom_probes() -> Result<Vec<CustomProbe>, String> {
    let path = probes_file_path()?;
    if !Path::new(&path).exists() {
        return Ok(vec![]);
    }
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read custom probes: {}", e))?;
    serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse custom probes: {}", e))
}

pub fn save_custom_probes(probes: &[CustomProbe]) -> Result<(), String> {
    let path = probes_file_path()?;
    let data = serde_json::to_string_pretty(probes)
        .map_err(|e| format!("Failed to serialize custom probes: {}", e))?;
    std::fs::write(&path, data)
        .map_err(|e| format!("Failed to write custom probes: {}", e))
}

pub fn delete_custom_probe(id: &str) -> Result<(), String> {
    let mut probes = load_custom_probes()?;
    probes.retain(|p| p.id != id);
    save_custom_probes(&probes)
}

pub fn test_probe_command(program: &str, args: &[String]) -> CommandRecord {
    run_cmd_traced(program, &args.iter().map(|s| s.as_str()).collect::<Vec<_>>(), "test")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get user's home directory.
fn home_dir() -> String {
    crate::commands::home_dir().unwrap_or_else(|| "/Users/unknown".to_string())
}

/// Augmented PATH for subprocess calls. macOS GUI apps don't inherit shell
/// PATH, so Homebrew, cargo, go, etc. won't be found without this.
fn augmented_path() -> String {
    let home = home_dir();
    let mut extra: Vec<String> = vec![
        "/opt/homebrew/bin".to_string(),
        "/opt/homebrew/sbin".to_string(),
        "/usr/local/bin".to_string(),
        "/usr/local/sbin".to_string(),
        format!("{}/.cargo/bin", home),
        format!("{}/go/bin", home),
        format!("{}/.pub-cache/bin", home),
        format!("{}/development/flutter/bin", home),
        format!("{}/.bun/bin", home),
        format!("{}/.deno/bin", home),
    ];

    // nvm: find active node version's bin directory
    let nvm_versions = format!("{}/.nvm/versions/node", home);
    if let Ok(entries) = std::fs::read_dir(&nvm_versions) {
        for entry in entries.filter_map(|e| e.ok()) {
            let bin = entry.path().join("bin");
            if bin.is_dir() {
                extra.push(bin.to_string_lossy().to_string());
            }
        }
    }

    let system_path = std::env::var("PATH").unwrap_or_default();
    format!("{}:{}", extra.join(":"), system_path)
}

/// Run a command with augmented PATH. Returns stdout trimmed, or empty on failure.
fn run_cmd(program: &str, args: &[&str]) -> String {
    match Command::new(program)
        .args(args)
        .env("PATH", augmented_path())
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => String::new(),
    }
}

/// Run a command and return a CommandRecord with timing and output info.
fn run_cmd_traced(program: &str, args: &[&str], purpose: &str) -> CommandRecord {
    let start = Instant::now();
    let result = Command::new(program)
        .args(args)
        .env("PATH", augmented_path())
        .output();
    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            let preview = if !stdout.is_empty() { &stdout } else { &stderr };
            CommandRecord {
                program: program.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
                purpose: purpose.to_string(),
                success: o.status.success(),
                duration_ms,
                output_preview: preview.chars().take(500).collect(),
            }
        }
        Err(e) => CommandRecord {
            program: program.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            purpose: purpose.to_string(),
            success: false,
            duration_ms,
            output_preview: format!("Error: {}", e),
        },
    }
}

/// Run a command with tracing, returning (full_stdout, record).
/// The record's output_preview is truncated to 500 chars; the returned string is full output.
fn run_cmd_with_trace(program: &str, args: &[&str], purpose: &str) -> (String, CommandRecord) {
    let start = Instant::now();
    let result = Command::new(program)
        .args(args)
        .env("PATH", augmented_path())
        .output();
    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            let preview = if !stdout.is_empty() { &stdout } else { &stderr };
            let rec = CommandRecord {
                program: program.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
                purpose: purpose.to_string(),
                success: o.status.success(),
                duration_ms,
                output_preview: preview.chars().take(500).collect(),
            };
            let full_out = if o.status.success() { stdout } else { String::new() };
            (full_out, rec)
        }
        Err(e) => {
            let rec = CommandRecord {
                program: program.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
                purpose: purpose.to_string(),
                success: false,
                duration_ms,
                output_preview: format!("Error: {}", e),
            };
            (String::new(), rec)
        }
    }
}

/// Run a command returning (stdout, stderr, record).
fn run_cmd_full_traced(program: &str, args: &[&str], purpose: &str) -> (String, String, CommandRecord) {
    let start = Instant::now();
    let result = Command::new(program)
        .args(args)
        .env("PATH", augmented_path())
        .output();
    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            let preview = if !stderr.is_empty() { &stderr } else { &stdout };
            let rec = CommandRecord {
                program: program.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
                purpose: purpose.to_string(),
                success: o.status.success(),
                duration_ms,
                output_preview: preview.chars().take(500).collect(),
            };
            (stdout, stderr, rec)
        }
        Err(e) => {
            let rec = CommandRecord {
                program: program.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
                purpose: purpose.to_string(),
                success: false,
                duration_ms,
                output_preview: format!("Error: {}", e),
            };
            (String::new(), String::new(), rec)
        }
    }
}

/// Check if a command exists on the system (with augmented PATH).
fn command_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .env("PATH", augmented_path())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if a command exists, returning a CommandRecord.
fn command_exists_traced(name: &str) -> (bool, CommandRecord) {
    let rec = run_cmd_traced("which", &[name], "detect");
    (rec.success, rec)
}

/// Get directory size via `du -sk`. Returns bytes.
fn dir_size(path: &str) -> u64 {
    crate::commands::get_du_size(path)
}

// ---------------------------------------------------------------------------
// Main scan entry point
// ---------------------------------------------------------------------------

/// Expand `~` to the user's home directory in a path string.
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") || path == "~" {
        let home = home_dir();
        path.replacen('~', &home, 1)
    } else {
        path.to_string()
    }
}

/// Execute all enabled custom probes and produce PackageManagerInfo/RuntimeInfo entries.
fn execute_custom_probes(managers: &mut Vec<PackageManagerInfo>, runtimes: &mut Vec<RuntimeInfo>) {
    let probes = match load_custom_probes() {
        Ok(p) => p,
        Err(_) => return,
    };

    for probe in probes.iter().filter(|p| p.enabled) {
        let mut trace: Vec<CommandRecord> = Vec::new();

        // 1. Detection
        let detect_rec = run_cmd_traced(
            &probe.detect.program,
            &probe.detect.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            "detect",
        );
        let detected = detect_rec.success;
        trace.push(detect_rec);

        if !detected {
            continue;
        }

        // 2. Version
        let version = if let Some(ref cmd) = probe.version {
            let (out, rec) = run_cmd_with_trace(
                &cmd.program,
                &cmd.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                "version",
            );
            trace.push(rec);
            out.lines().next().unwrap_or("").to_string()
        } else {
            String::new()
        };

        // 3. Size
        let total_size: u64 = probe.size_paths.iter()
            .map(|p| dir_size(&expand_tilde(p)))
            .sum();

        // 4. Install path
        let install_path = expand_tilde(&probe.install_path);

        match probe.probe_type {
            ProbeType::Manager => {
                // 5. List packages (optional)
                let mut packages: Vec<InstalledPackage> = Vec::new();

                if let Some(ref cmd) = probe.list_packages {
                    let (list_output, rec) = run_cmd_with_trace(
                        &cmd.program,
                        &cmd.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                        "list packages",
                    );
                    trace.push(rec);

                    match probe.list_parse_mode {
                        ParseMode::Lines => {
                            for line in list_output.lines() {
                                let trimmed = line.trim();
                                if trimmed.is_empty() { continue; }
                                let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
                                let name = parts[0].to_string();
                                let ver = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
                                packages.push(InstalledPackage {
                                    name: name.clone(),
                                    version: ver,
                                    size: 0,
                                    is_top_level: true,
                                    dependencies: vec![],
                                    uninstall_command: String::new(),
                                    removal_warning: String::new(),
                                });
                            }
                        }
                        ParseMode::Json => {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&list_output) {
                                if let Some(arr) = parsed.as_array() {
                                    for item in arr {
                                        let name = item.get("name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("").to_string();
                                        let ver = item.get("version")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("").to_string();
                                        if !name.is_empty() {
                                            packages.push(InstalledPackage {
                                                name,
                                                version: ver,
                                                size: 0,
                                                is_top_level: true,
                                                dependencies: vec![],
                                                uninstall_command: String::new(),
                                                removal_warning: String::new(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        ParseMode::None => {}
                    }
                }

                let total_package_count = packages.len();

                managers.push(PackageManagerInfo {
                    id: format!("custom-{}", probe.id),
                    name: probe.name.clone(),
                    version,
                    install_path,
                    total_size,
                    packages,
                    total_package_count,
                    detected: true,
                    uninstall_hint: probe.uninstall_hint.clone(),
                    is_custom: true,
                    commands_run: trace,
                });
            }
            ProbeType::Runtime => {
                let versions = vec![RuntimeVersion {
                    version: version.clone(),
                    active: true,
                    path: install_path.clone(),
                    size: total_size,
                }];

                runtimes.push(RuntimeInfo {
                    id: format!("custom-{}", probe.id),
                    name: probe.name.clone(),
                    install_method: "custom probe".to_string(),
                    install_path,
                    total_size,
                    versions,
                    uninstall_hint: probe.uninstall_hint.clone(),
                    removal_warning: String::new(),
                    is_custom: true,
                    commands_run: trace,
                });
            }
        }
    }
}

/// Run a full package and runtime scan. Detects everything present on the system.
pub fn scan_all() -> PackageScanResult {
    let mut managers: Vec<PackageManagerInfo> = Vec::new();
    let mut runtimes: Vec<RuntimeInfo> = Vec::new();

    // Built-in package managers
    if let Some(m) = scan_homebrew() {
        managers.push(m);
    }
    if let Some(m) = scan_pip() {
        managers.push(m);
    }
    if let Some(m) = scan_npm_global() {
        managers.push(m);
    }
    if let Some(m) = scan_cargo() {
        managers.push(m);
    }
    if let Some(m) = scan_pnpm() {
        managers.push(m);
    }
    if let Some(m) = scan_yarn() {
        managers.push(m);
    }
    if let Some(m) = scan_cocoapods() {
        managers.push(m);
    }
    if let Some(m) = scan_pub() {
        managers.push(m);
    }
    if let Some(m) = scan_gem() {
        managers.push(m);
    }

    // Built-in runtimes
    runtimes.extend(scan_java_runtimes());
    runtimes.extend(scan_nvm());
    runtimes.extend(scan_bun_runtime());
    runtimes.extend(scan_deno_runtime());
    runtimes.extend(scan_rust_toolchains());
    runtimes.extend(scan_go_runtime());
    runtimes.extend(scan_flutter_runtime());
    runtimes.extend(scan_gradle());

    // Custom probes
    execute_custom_probes(&mut managers, &mut runtimes);

    let total_size = managers.iter().map(|m| m.total_size).sum::<u64>()
        + runtimes.iter().map(|r| r.total_size).sum::<u64>();

    PackageScanResult {
        managers,
        runtimes,
        total_size,
    }
}

// ---------------------------------------------------------------------------
// Homebrew
// ---------------------------------------------------------------------------

/// Detect Homebrew and enumerate installed formulae and casks.
fn scan_homebrew() -> Option<PackageManagerInfo> {
    let mut trace: Vec<CommandRecord> = Vec::new();

    let (exists, rec) = command_exists_traced("brew");
    trace.push(rec);
    if !exists { return None; }

    let (version_raw, rec) = run_cmd_with_trace("brew", &["--version"], "version");
    trace.push(rec);
    let version = version_raw.lines().next().unwrap_or("").to_string();

    let prefix = run_cmd("brew", &["--prefix"]);
    let cellar_path = format!("{}/Cellar", prefix);
    let caskroom_path = format!("{}/Caskroom", prefix);

    let cellar_size = dir_size(&cellar_path);
    let caskroom_size = dir_size(&caskroom_path);
    let total_size = cellar_size + caskroom_size;

    let (leaves_output, rec) = run_cmd_with_trace("brew", &["leaves"], "list top-level");
    trace.push(rec);
    let leaves: std::collections::HashSet<String> = leaves_output
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let (formulae_output, rec) = run_cmd_with_trace("brew", &["list", "--formulae", "--versions"], "list formulae");
    trace.push(rec);

    let deps_output = run_cmd("brew", &["deps", "--installed"]);
    let mut deps_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for line in deps_output.lines() {
        if let Some((pkg, deps)) = line.split_once(':') {
            let pkg = pkg.trim().to_string();
            let deps: Vec<String> = deps.split_whitespace().map(|s| s.to_string()).collect();
            deps_map.insert(pkg, deps);
        }
    }

    let mut packages: Vec<InstalledPackage> = Vec::new();

    for line in formulae_output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let name = parts[0].to_string();
        let version = parts.get(1).unwrap_or(&"").to_string();
        let is_top_level = leaves.contains(&name);

        // Size of this formula's Cellar directory.
        let pkg_path = format!("{}/{}", cellar_path, name);
        let size = dir_size(&pkg_path);

        let dependencies = deps_map.get(&name).cloned().unwrap_or_default();

        // Build removal warning based on what depends on this package.
        let dependents = find_brew_dependents(&name, &deps_map);
        let removal_warning = if dependents.is_empty() {
            String::new()
        } else {
            format!(
                "Used by: {}. Removing may break these packages.",
                dependents.join(", ")
            )
        };

        packages.push(InstalledPackage {
            name: name.clone(),
            version,
            size,
            is_top_level,
            dependencies,
            uninstall_command: format!("brew uninstall {}", name),
            removal_warning,
        });
    }

    let (casks_output, rec) = run_cmd_with_trace("brew", &["list", "--cask", "--versions"], "list casks");
    trace.push(rec);
    for line in casks_output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let name = parts[0].to_string();
        let version = parts.get(1).unwrap_or(&"").to_string();
        let pkg_path = format!("{}/{}", caskroom_path, name);
        let size = dir_size(&pkg_path);

        packages.push(InstalledPackage {
            name: format!("{} (cask)", name),
            version,
            size,
            is_top_level: true, // casks are always explicitly installed
            dependencies: vec![],
            uninstall_command: format!("brew uninstall --cask {}", name),
            removal_warning: String::new(),
        });
    }

    let total_package_count = packages.len();

    // Sort: top-level first, then by size descending.
    packages.sort_by(|a, b| {
        b.is_top_level
            .cmp(&a.is_top_level)
            .then(b.size.cmp(&a.size))
    });

    Some(PackageManagerInfo {
        id: "homebrew".to_string(),
        name: "Homebrew".to_string(),
        version,
        install_path: prefix,
        total_size,
        packages,
        total_package_count,
        detected: true,
        uninstall_hint: "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/uninstall.sh)\"".to_string(),
        is_custom: false,
        commands_run: trace,
    })
}

/// Find which installed packages depend on `target`.
fn find_brew_dependents(
    target: &str,
    deps_map: &std::collections::HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut dependents = Vec::new();
    for (pkg, deps) in deps_map {
        if deps.iter().any(|d| d == target) {
            dependents.push(pkg.clone());
        }
    }
    dependents.sort();
    dependents
}

// ---------------------------------------------------------------------------
// pip (Python)
// ---------------------------------------------------------------------------

/// Detect pip and list globally installed Python packages.
fn scan_pip() -> Option<PackageManagerInfo> {
    let mut trace: Vec<CommandRecord> = Vec::new();

    let (exists, rec) = command_exists_traced("pip3");
    trace.push(rec);
    if !exists { return None; }

    let (version_output, rec) = run_cmd_with_trace("pip3", &["--version"], "version");
    trace.push(rec);
    // "pip 24.3.1 from /opt/homebrew/lib/python3.13/site-packages/pip (python 3.13)"
    let version = version_output
        .split_whitespace()
        .nth(1)
        .unwrap_or("")
        .to_string();

    // Extract site-packages path from the version output.
    let site_path = version_output
        .split("from ")
        .nth(1)
        .and_then(|s| s.split("/pip").next())
        .unwrap_or("")
        .to_string();

    let total_size = if !site_path.is_empty() {
        dir_size(&site_path)
    } else {
        0
    };

    let (top_level_output, rec) = run_cmd_with_trace("pip3", &["list", "--not-required", "--format=json"], "list top-level");
    trace.push(rec);
    let (all_output, rec) = run_cmd_with_trace("pip3", &["list", "--format=json"], "list all");
    trace.push(rec);

    // Parse JSON lists.
    let top_level_names: std::collections::HashSet<String> = parse_pip_json(&top_level_output)
        .into_iter()
        .map(|(name, _)| name.to_lowercase())
        .collect();

    let all_packages = parse_pip_json(&all_output);
    let total_package_count = all_packages.len();

    let mut packages: Vec<InstalledPackage> = Vec::new();
    for (name, ver) in &all_packages {
        let is_top_level = top_level_names.contains(&name.to_lowercase());

        // Get dependency info for top-level packages.
        let (dependencies, removal_warning) = if is_top_level {
            let show = run_cmd("pip3", &["show", name]);
            let deps = extract_pip_field(&show, "Requires");
            let required_by = extract_pip_field(&show, "Required-by");
            let warning = if required_by.is_empty() {
                String::new()
            } else {
                format!(
                    "Required by: {}. Removing may break these packages.",
                    required_by
                )
            };
            (
                deps.split(", ")
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                warning,
            )
        } else {
            (vec![], String::new())
        };

        packages.push(InstalledPackage {
            name: name.clone(),
            version: ver.clone(),
            size: 0, // pip doesn't expose per-package size easily
            is_top_level,
            dependencies,
            uninstall_command: format!("pip3 uninstall {}", name),
            removal_warning,
        });
    }

    // Sort: top-level first, then alphabetically.
    packages.sort_by(|a, b| {
        b.is_top_level
            .cmp(&a.is_top_level)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Some(PackageManagerInfo {
        id: "pip".to_string(),
        name: "pip (Python)".to_string(),
        version,
        install_path: site_path,
        total_size,
        packages,
        total_package_count,
        detected: true,
        uninstall_hint: "pip is part of your Python installation. Remove Python to remove pip."
            .to_string(),
        is_custom: false,
        commands_run: trace,
    })
}

/// Parse pip's JSON list output: [{"name": "foo", "version": "1.0"}, ...]
fn parse_pip_json(json: &str) -> Vec<(String, String)> {
    // Minimal JSON parsing without pulling in a full JSON crate for this.
    // pip list --format=json returns a simple array of {name, version} objects.
    let mut results = Vec::new();
    if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
        for item in parsed {
            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let version = item
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !name.is_empty() {
                results.push((name, version));
            }
        }
    }
    results
}

/// Extract a field value from `pip3 show` output.
/// Lines are like "Requires: foo, bar" or "Required-by: baz".
fn extract_pip_field(show_output: &str, field: &str) -> String {
    for line in show_output.lines() {
        if line.starts_with(field) {
            if let Some((_, value)) = line.split_once(':') {
                return value.trim().to_string();
            }
        }
    }
    String::new()
}

// ---------------------------------------------------------------------------
// npm (global)
// ---------------------------------------------------------------------------

/// Detect npm and list globally installed packages.
fn scan_npm_global() -> Option<PackageManagerInfo> {
    let mut trace: Vec<CommandRecord> = Vec::new();

    let (exists, rec) = command_exists_traced("npm");
    trace.push(rec);
    if !exists { return None; }

    let (version, rec) = run_cmd_with_trace("npm", &["--version"], "version");
    trace.push(rec);
    let global_root = run_cmd("npm", &["root", "-g"]);
    // e.g. "/opt/homebrew/lib/node_modules" or "~/.nvm/versions/node/v20/lib/node_modules"

    let total_size = if !global_root.is_empty() {
        dir_size(&global_root)
    } else {
        0
    };

    let (list_output, rec) = run_cmd_with_trace("npm", &["list", "-g", "--depth=0", "--json"], "list global packages");
    trace.push(rec);

    let mut packages: Vec<InstalledPackage> = Vec::new();

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&list_output) {
        if let Some(deps) = parsed.get("dependencies").and_then(|d| d.as_object()) {
            for (name, info) in deps {
                let version = info
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let pkg_path = format!("{}/{}", global_root, name);
                let size = dir_size(&pkg_path);

                packages.push(InstalledPackage {
                    name: name.clone(),
                    version,
                    size,
                    is_top_level: true,
                    dependencies: vec![],
                    uninstall_command: format!("npm uninstall -g {}", name),
                    removal_warning: String::new(),
                });
            }
        }
    }

    let total_package_count = packages.len();

    // Sort by size descending.
    packages.sort_by(|a, b| b.size.cmp(&a.size));

    // Also account for npm cache.
    let home = home_dir();
    let cache_path = format!("{}/.npm", home);
    let cache_size = dir_size(&cache_path);

    Some(PackageManagerInfo {
        id: "npm".to_string(),
        name: "npm (Node.js)".to_string(),
        version,
        install_path: global_root,
        total_size: total_size + cache_size,
        packages,
        total_package_count,
        detected: true,
        uninstall_hint: "npm is part of your Node.js installation. Run `npm cache clean --force` to clear the cache.".to_string(),
        is_custom: false,
        commands_run: trace,
    })
}

// ---------------------------------------------------------------------------
// Cargo (Rust global installs)
// ---------------------------------------------------------------------------

/// Detect cargo and list globally installed binaries.
fn scan_cargo() -> Option<PackageManagerInfo> {
    let mut trace: Vec<CommandRecord> = Vec::new();
    let home = home_dir();
    let cargo_path = format!("{}/.cargo", home);

    if !Path::new(&cargo_path).is_dir() {
        return None;
    }

    let (version_raw, rec) = run_cmd_with_trace("cargo", &["--version"], "version");
    trace.push(rec);
    let version = version_raw.split_whitespace().nth(1).unwrap_or("").to_string();

    let total_size = dir_size(&cargo_path);

    let (list_output, rec) = run_cmd_with_trace("cargo", &["install", "--list"], "list installed");
    trace.push(rec);

    let mut packages: Vec<InstalledPackage> = Vec::new();

    for line in list_output.lines() {
        // Package lines end with ":"
        if line.ends_with(':') && !line.starts_with(' ') {
            let parts: Vec<&str> = line.trim_end_matches(':').split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let ver = parts[1].trim_start_matches('v').to_string();

                packages.push(InstalledPackage {
                    name: name.clone(),
                    version: ver,
                    size: 0, // individual binary sizes are hard to attribute
                    is_top_level: true,
                    dependencies: vec![],
                    uninstall_command: format!("cargo uninstall {}", name),
                    removal_warning: String::new(),
                });
            }
        }
    }

    let total_package_count = packages.len();

    // Sort alphabetically.
    packages.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Break down sizes for display.
    let _bin_size = dir_size(&format!("{}/bin", cargo_path));
    let registry_size = dir_size(&format!("{}/registry", cargo_path));

    Some(PackageManagerInfo {
        id: "cargo".to_string(),
        name: "Cargo (Rust)".to_string(),
        version,
        install_path: cargo_path,
        total_size,
        packages,
        total_package_count,
        detected: true,
        uninstall_hint: format!(
            "Registry cache: {} (run `cargo cache -a` to clear). Uninstall Rust entirely with `rustup self uninstall`.",
            format_size_approx(registry_size)
        ),
        is_custom: false,
        commands_run: trace,
    })
}

// ---------------------------------------------------------------------------
// Java runtimes
// ---------------------------------------------------------------------------

/// Detect installed Java Virtual Machines via /usr/libexec/java_home.
fn scan_java_runtimes() -> Vec<RuntimeInfo> {
    let mut results = Vec::new();
    let mut trace: Vec<CommandRecord> = Vec::new();

    let (_, text, rec) = run_cmd_full_traced("/usr/libexec/java_home", &["-V"], "detect JVMs");
    trace.push(rec);
    if text.is_empty() {
        return results;
    }

    // Parse lines like:
    //   17.0.12 (arm64) "Oracle Corporation" - "Java SE 17.0.12" /Library/Java/...
    let mut versions: Vec<RuntimeVersion> = Vec::new();
    let mut total_size: u64 = 0;

    // The last non-indented line is the default JAVA_HOME path.
    let default_home = text.lines().last().unwrap_or("").trim().to_string();

    for line in text.lines() {
        let trimmed = line.trim();
        // JVM lines start with a version number and are indented.
        if !trimmed.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }

        // Extract version and path. Path is the last space-separated token.
        // Format: "17.0.12 (arm64) "Oracle Corporation" - "Java SE 17.0.12" /path"
        // We find the path by looking for the last token that starts with /.
        let path = trimmed
            .rsplit_once(' ')
            .map(|(_, p)| p.to_string())
            .unwrap_or_default();

        let ver = trimmed.split_whitespace().next().unwrap_or("").to_string();

        if path.is_empty() || ver.is_empty() {
            continue;
        }

        // The JDK directory is two levels up from Contents/Home.
        // /Library/Java/JavaVirtualMachines/jdk-17.jdk/Contents/Home
        // → size the jdk-17.jdk directory.
        let jdk_dir = Path::new(&path)
            .parent() // Contents
            .and_then(|p| p.parent()) // jdk-17.jdk
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(path.clone());

        let size = dir_size(&jdk_dir);
        total_size += size;

        let active = path == default_home || jdk_dir.contains(&ver);

        versions.push(RuntimeVersion {
            version: ver,
            active,
            path: jdk_dir,
            size,
        });
    }

    if !versions.is_empty() {
        results.push(RuntimeInfo {
            id: "java".to_string(),
            name: "Java (JDK)".to_string(),
            install_method: "manual / installer".to_string(),
            install_path: "/Library/Java/JavaVirtualMachines".to_string(),
            total_size,
            versions,
            uninstall_hint: "Remove the JDK directory from /Library/Java/JavaVirtualMachines/".to_string(),
            removal_warning: "Some applications (e.g. Minecraft, Android Studio, IntelliJ) require a Java runtime. Check what depends on Java before removing.".to_string(),
            is_custom: false,
            commands_run: trace,
        });
    }

    results
}

// ---------------------------------------------------------------------------
// nvm (Node Version Manager)
// ---------------------------------------------------------------------------

/// Detect nvm and list installed Node.js versions.
fn scan_nvm() -> Vec<RuntimeInfo> {
    let mut results = Vec::new();
    let home = home_dir();
    let nvm_dir = format!("{}/.nvm", home);

    if !Path::new(&nvm_dir).is_dir() {
        return results;
    }

    let versions_dir = format!("{}/versions/node", nvm_dir);
    if !Path::new(&versions_dir).is_dir() {
        return results;
    }

    let total_size = dir_size(&nvm_dir);

    // Detect the active/default version.
    // nvm stores the default alias in ~/.nvm/alias/default
    let default_alias_path = format!("{}/alias/default", nvm_dir);
    let default_version = if Path::new(&default_alias_path).exists() {
        std::fs::read_to_string(&default_alias_path)
            .unwrap_or_default()
            .trim()
            .to_string()
    } else {
        String::new()
    };

    let mut versions: Vec<RuntimeVersion> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&versions_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with('v') {
                continue;
            }
            let ver = name.trim_start_matches('v').to_string();
            let path = entry.path().to_string_lossy().to_string();
            let size = dir_size(&path);

            // Check if this version matches the default alias.
            let active = name == default_version
                || ver == default_version
                || default_version.starts_with(&ver);

            versions.push(RuntimeVersion {
                version: ver,
                active,
                path,
                size,
            });
        }
    }

    // Sort by version descending (simple string sort, good enough for semver).
    versions.sort_by(|a, b| b.version.cmp(&a.version));

    if !versions.is_empty() {
        results.push(RuntimeInfo {
            id: "node-nvm".to_string(),
            name: "Node.js (nvm)".to_string(),
            install_method: "nvm".to_string(),
            install_path: nvm_dir,
            total_size,
            versions,
            uninstall_hint: "Remove a version: `nvm uninstall <version>`. Remove nvm entirely: delete ~/.nvm and remove nvm lines from your shell config.".to_string(),
            removal_warning: "Node.js is required for JavaScript/TypeScript development and many build tools (Vite, webpack, etc.). Global npm packages are installed per-version.".to_string(),
            is_custom: false,
            commands_run: vec![],
        });
    }

    results
}

// ---------------------------------------------------------------------------
// Rust (rustup + cargo)
// ---------------------------------------------------------------------------

/// Detect rustup toolchains.
fn scan_rust_toolchains() -> Vec<RuntimeInfo> {
    let mut results = Vec::new();
    let mut trace: Vec<CommandRecord> = Vec::new();
    let home = home_dir();
    let rustup_dir = format!("{}/.rustup", home);

    if !Path::new(&rustup_dir).is_dir() {
        return results;
    }

    let total_size = dir_size(&rustup_dir);

    let (toolchains_output, rec) = run_cmd_with_trace("rustup", &["toolchain", "list"], "list toolchains");
    trace.push(rec);
    // Lines like: "stable-aarch64-apple-darwin (default)"

    let mut versions: Vec<RuntimeVersion> = Vec::new();

    for line in toolchains_output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let active = trimmed.contains("(default)") || trimmed.contains("(active)");
        let name = trimmed.split_whitespace().next().unwrap_or("").to_string();

        let toolchain_path = format!("{}/toolchains/{}", rustup_dir, name);
        let size = dir_size(&toolchain_path);

        versions.push(RuntimeVersion {
            version: name,
            active,
            path: toolchain_path,
            size,
        });
    }

    if !versions.is_empty() {
        results.push(RuntimeInfo {
            id: "rust".to_string(),
            name: "Rust (rustup)".to_string(),
            install_method: "rustup".to_string(),
            install_path: rustup_dir,
            total_size,
            versions,
            uninstall_hint: "Remove a toolchain: `rustup toolchain uninstall <name>`. Uninstall Rust entirely: `rustup self uninstall`.".to_string(),
            removal_warning: "Rust is needed to compile Rust projects. Cargo-installed binaries in ~/.cargo/bin/ will also stop working.".to_string(),
            is_custom: false,
            commands_run: trace,
        });
    }

    results
}

// ---------------------------------------------------------------------------
// Go
// ---------------------------------------------------------------------------

/// Detect Go installation and workspace.
fn scan_go_runtime() -> Vec<RuntimeInfo> {
    let mut results = Vec::new();
    let mut trace: Vec<CommandRecord> = Vec::new();

    let (exists, rec) = command_exists_traced("go");
    trace.push(rec);
    if !exists { return results; }

    let (version, rec) = run_cmd_with_trace("go", &["version"], "version");
    trace.push(rec);
    // "go version go1.22.4 darwin/arm64"
    let ver = version
        .split_whitespace()
        .nth(2)
        .unwrap_or("")
        .trim_start_matches("go")
        .to_string();

    let goroot = run_cmd("go", &["env", "GOROOT"]);
    let gopath = run_cmd("go", &["env", "GOPATH"]);
    let gomodcache = run_cmd("go", &["env", "GOMODCACHE"]);

    let goroot_size = if !goroot.is_empty() {
        dir_size(&goroot)
    } else {
        0
    };
    let gopath_size = if !gopath.is_empty() {
        dir_size(&gopath)
    } else {
        0
    };

    let total_size = goroot_size + gopath_size;

    let mut versions = vec![RuntimeVersion {
        version: ver.clone(),
        active: true,
        path: goroot.clone(),
        size: goroot_size,
    }];

    // Add GOPATH and module cache as separate "versions" for size visibility.
    if !gopath.is_empty() && gopath_size > 0 {
        let modcache_size = if !gomodcache.is_empty() {
            dir_size(&gomodcache)
        } else {
            0
        };
        let bin_size = dir_size(&format!("{}/bin", gopath));

        versions.push(RuntimeVersion {
            version: format!(
                "GOPATH (bins: {}, module cache: {})",
                format_size_approx(bin_size),
                format_size_approx(modcache_size)
            ),
            active: false,
            path: gopath.clone(),
            size: gopath_size,
        });
    }

    results.push(RuntimeInfo {
        id: "go".to_string(),
        name: "Go".to_string(),
        install_method: if goroot.contains("homebrew") {
            "homebrew".to_string()
        } else {
            "manual / installer".to_string()
        },
        install_path: goroot,
        total_size,
        versions,
        uninstall_hint: format!(
            "Remove Go SDK: delete GOROOT directory. Clear module cache: `go clean -modcache`. Clear GOPATH: delete {}",
            gopath
        ),
        removal_warning: "Go is required to compile Go projects. Installed Go binaries in $GOPATH/bin/ will stop working.".to_string(),
        is_custom: false,
        commands_run: trace,
    });

    results
}

// ---------------------------------------------------------------------------
// Flutter / Dart
// ---------------------------------------------------------------------------

/// Detect Flutter SDK installation.
fn scan_flutter_runtime() -> Vec<RuntimeInfo> {
    let mut results = Vec::new();
    let mut trace: Vec<CommandRecord> = Vec::new();

    let (exists, rec) = command_exists_traced("flutter");
    trace.push(rec);
    if !exists { return results; }

    let (version_output, rec) = run_cmd_with_trace("flutter", &["--version"], "version");
    trace.push(rec);
    // First line: "Flutter 3.24.0 • channel stable • https://..."
    let ver = version_output
        .lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .nth(1)
        .unwrap_or("")
        .to_string();

    // Find the flutter SDK path by resolving the binary.
    let which_flutter = run_cmd("which", &["flutter"]);
    // e.g. "/Users/foo/development/flutter/bin/flutter"
    let sdk_path = Path::new(&which_flutter)
        .parent() // bin/
        .and_then(|p| p.parent()) // flutter/
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    if sdk_path.is_empty() {
        return results;
    }

    let sdk_size = dir_size(&sdk_path);

    // Dart pub cache.
    let home = home_dir();
    let pub_cache = format!("{}/.pub-cache", home);
    let pub_cache_size = if Path::new(&pub_cache).is_dir() {
        dir_size(&pub_cache)
    } else {
        0
    };

    let total_size = sdk_size + pub_cache_size;

    let dart_version = run_cmd("dart", &["--version"]);
    // "Dart SDK version: 3.5.0 (stable) ..."
    let dart_ver = dart_version
        .split("version: ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .unwrap_or("")
        .to_string();

    let mut versions = vec![RuntimeVersion {
        version: format!("Flutter {}", ver),
        active: true,
        path: sdk_path.clone(),
        size: sdk_size,
    }];

    if pub_cache_size > 0 {
        versions.push(RuntimeVersion {
            version: format!("Dart pub cache ({})", format_size_approx(pub_cache_size)),
            active: false,
            path: pub_cache.clone(),
            size: pub_cache_size,
        });
    }

    if !dart_ver.is_empty() {
        versions.push(RuntimeVersion {
            version: format!("Dart SDK {}", dart_ver),
            active: true,
            path: format!("{}/bin/cache/dart-sdk", sdk_path),
            size: 0, // included in Flutter SDK size
        });
    }

    results.push(RuntimeInfo {
        id: "flutter".to_string(),
        name: "Flutter / Dart".to_string(),
        install_method: "manual".to_string(),
        install_path: sdk_path,
        total_size,
        versions,
        uninstall_hint: format!(
            "Delete the Flutter SDK directory and ~/.pub-cache. Remove Flutter from your PATH in shell config."
        ),
        removal_warning: "Flutter is required for Flutter/Dart development. Removing it will break all Flutter projects.".to_string(),
        is_custom: false,
        commands_run: trace,
    });

    results
}

// ---------------------------------------------------------------------------
// pnpm
// ---------------------------------------------------------------------------

fn scan_pnpm() -> Option<PackageManagerInfo> {
    let mut trace: Vec<CommandRecord> = Vec::new();

    let (exists, rec) = command_exists_traced("pnpm");
    trace.push(rec);
    if !exists { return None; }

    let (version, rec) = run_cmd_with_trace("pnpm", &["--version"], "version");
    trace.push(rec);
    let store_path = run_cmd("pnpm", &["store", "path"]);

    let store_size = if !store_path.is_empty() {
        dir_size(&store_path)
    } else {
        0
    };

    // Global packages
    let global_dir = run_cmd("pnpm", &["root", "-g"]);
    let global_size = if !global_dir.is_empty() { dir_size(&global_dir) } else { 0 };

    let list_output = run_cmd("pnpm", &["list", "-g", "--depth=0", "--json"]);
    let mut packages: Vec<InstalledPackage> = Vec::new();

    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&list_output) {
        // pnpm list -g --json returns an array
        let items = if parsed.is_array() { parsed.as_array().cloned().unwrap_or_default() } else { vec![parsed] };
        for item in items {
            if let Some(deps) = item.get("dependencies").and_then(|d| d.as_object()) {
                for (name, info) in deps {
                    let ver = info.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    packages.push(InstalledPackage {
                        name: name.clone(),
                        version: ver,
                        size: 0,
                        is_top_level: true,
                        dependencies: vec![],
                        uninstall_command: format!("pnpm remove -g {}", name),
                        removal_warning: String::new(),
                    });
                }
            }
        }
    }

    let total_package_count = packages.len();

    Some(PackageManagerInfo {
        id: "pnpm".to_string(),
        name: "pnpm".to_string(),
        version,
        install_path: store_path.clone(),
        total_size: store_size + global_size,
        packages,
        total_package_count,
        detected: true,
        uninstall_hint: format!(
            "Content-addressable store: {} (run `pnpm store prune` to clean).",
            format_size_approx(store_size)
        ),
        is_custom: false,
        commands_run: trace,
    })
}

// ---------------------------------------------------------------------------
// yarn
// ---------------------------------------------------------------------------

fn scan_yarn() -> Option<PackageManagerInfo> {
    let mut trace: Vec<CommandRecord> = Vec::new();

    let (exists, rec) = command_exists_traced("yarn");
    trace.push(rec);
    if !exists { return None; }

    let (version, rec) = run_cmd_with_trace("yarn", &["--version"], "version");
    trace.push(rec);
    let home = home_dir();

    // Yarn 1.x cache
    let cache_v1 = format!("{}/Library/Caches/Yarn/v6", home);
    // Yarn Berry (2+) cache
    let cache_berry = run_cmd("yarn", &["config", "get", "cacheFolder"]);

    let mut total_size: u64 = 0;
    let mut install_path = String::new();

    if Path::new(&cache_v1).is_dir() {
        total_size += dir_size(&cache_v1);
        install_path = cache_v1;
    }
    if !cache_berry.is_empty() && Path::new(&cache_berry).is_dir() {
        total_size += dir_size(&cache_berry);
        if install_path.is_empty() {
            install_path = cache_berry;
        }
    }

    // Global packages
    let global_dir = run_cmd("yarn", &["global", "dir"]);
    if !global_dir.is_empty() && Path::new(&global_dir).is_dir() {
        total_size += dir_size(&global_dir);
    }

    Some(PackageManagerInfo {
        id: "yarn".to_string(),
        name: "Yarn".to_string(),
        version,
        install_path,
        total_size,
        packages: vec![],
        total_package_count: 0,
        detected: true,
        uninstall_hint: "Clear cache: `yarn cache clean`.".to_string(),
        is_custom: false,
        commands_run: trace,
    })
}

// ---------------------------------------------------------------------------
// Bun
// ---------------------------------------------------------------------------

fn scan_bun_runtime() -> Vec<RuntimeInfo> {
    let mut results = Vec::new();
    let mut trace: Vec<CommandRecord> = Vec::new();
    let home = home_dir();
    let bun_dir = format!("{}/.bun", home);

    if !Path::new(&bun_dir).is_dir() {
        return results;
    }

    let (version, rec) = run_cmd_with_trace("bun", &["--version"], "version");
    trace.push(rec);
    if version.is_empty() {
        return results;
    }

    let total_size = dir_size(&bun_dir);
    let cache_size = dir_size(&format!("{}/install/cache", bun_dir));

    let mut versions = vec![RuntimeVersion {
        version: version.clone(),
        active: true,
        path: format!("{}/bin", bun_dir),
        size: dir_size(&format!("{}/bin", bun_dir)),
    }];

    if cache_size > 0 {
        versions.push(RuntimeVersion {
            version: format!("Module cache ({})", format_size_approx(cache_size)),
            active: false,
            path: format!("{}/install/cache", bun_dir),
            size: cache_size,
        });
    }

    results.push(RuntimeInfo {
        id: "bun".to_string(),
        name: "Bun".to_string(),
        install_method: if Path::new(&format!("{}/bin/bun", bun_dir)).exists() {
            "bun installer".to_string()
        } else {
            "unknown".to_string()
        },
        install_path: bun_dir,
        total_size,
        versions,
        uninstall_hint: "Remove ~/.bun and remove bun lines from shell config.".to_string(),
        removal_warning: "Bun is an all-in-one JavaScript runtime, bundler, and package manager.".to_string(),
        is_custom: false,
        commands_run: trace,
    });

    results
}

// ---------------------------------------------------------------------------
// Deno
// ---------------------------------------------------------------------------

fn scan_deno_runtime() -> Vec<RuntimeInfo> {
    let mut results = Vec::new();
    let mut trace: Vec<CommandRecord> = Vec::new();
    let home = home_dir();
    let deno_dir = format!("{}/.deno", home);

    if !Path::new(&deno_dir).is_dir() && !command_exists("deno") {
        return results;
    }

    let (version, rec) = run_cmd_with_trace("deno", &["--version"], "version");
    trace.push(rec);
    let ver = version.lines().next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("").to_string();

    if ver.is_empty() {
        return results;
    }

    // Deno stores caches in DENO_DIR (defaults to platform cache dir)
    let cache_dir = std::env::var("DENO_DIR").unwrap_or_else(|_|
        format!("{}/Library/Caches/deno", home)
    );

    let deno_size = dir_size(&deno_dir);
    let cache_size = if Path::new(&cache_dir).is_dir() { dir_size(&cache_dir) } else { 0 };
    let total_size = deno_size + cache_size;

    let mut versions = vec![RuntimeVersion {
        version: ver,
        active: true,
        path: deno_dir.clone(),
        size: deno_size,
    }];

    if cache_size > 0 {
        versions.push(RuntimeVersion {
            version: format!("Module cache ({})", format_size_approx(cache_size)),
            active: false,
            path: cache_dir,
            size: cache_size,
        });
    }

    results.push(RuntimeInfo {
        id: "deno".to_string(),
        name: "Deno".to_string(),
        install_method: "deno installer".to_string(),
        install_path: deno_dir,
        total_size,
        versions,
        uninstall_hint: "Remove ~/.deno and clear cache at ~/Library/Caches/deno.".to_string(),
        removal_warning: "Deno is a secure JavaScript/TypeScript runtime.".to_string(),
        is_custom: false,
        commands_run: trace,
    });

    results
}

// ---------------------------------------------------------------------------
// CocoaPods
// ---------------------------------------------------------------------------

fn scan_cocoapods() -> Option<PackageManagerInfo> {
    let mut trace: Vec<CommandRecord> = Vec::new();

    let (exists, rec) = command_exists_traced("pod");
    trace.push(rec);
    if !exists { return None; }

    let (version, rec) = run_cmd_with_trace("pod", &["--version"], "version");
    trace.push(rec);
    let home = home_dir();
    let repos_path = format!("{}/.cocoapods", home);
    let cache_path = format!("{}/Library/Caches/CocoaPods", home);

    let repos_size = dir_size(&repos_path);
    let cache_size = dir_size(&cache_path);
    let total_size = repos_size + cache_size;

    Some(PackageManagerInfo {
        id: "cocoapods".to_string(),
        name: "CocoaPods".to_string(),
        version,
        install_path: repos_path,
        total_size,
        packages: vec![],
        total_package_count: 0,
        detected: true,
        uninstall_hint: format!(
            "Clear cache: `pod cache clean --all` ({}). Remove repos: `rm -rf ~/.cocoapods`.",
            format_size_approx(cache_size)
        ),
        is_custom: false,
        commands_run: trace,
    })
}

// ---------------------------------------------------------------------------
// pub (Dart/Flutter packages)
// ---------------------------------------------------------------------------

fn scan_pub() -> Option<PackageManagerInfo> {
    let home = home_dir();
    let pub_cache = format!("{}/.pub-cache", home);

    if !Path::new(&pub_cache).is_dir() {
        return None;
    }

    let total_size = dir_size(&pub_cache);
    let hosted_dir = format!("{}/hosted/pub.dev", pub_cache);

    let mut packages: Vec<InstalledPackage> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&hosted_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            // Packages are dirs like "http-1.2.0"
            if let Some((pkg_name, ver)) = name.rsplit_once('-') {
                let size = dir_size(&entry.path().to_string_lossy());
                packages.push(InstalledPackage {
                    name: pkg_name.to_string(),
                    version: ver.to_string(),
                    size,
                    is_top_level: true,
                    dependencies: vec![],
                    uninstall_command: format!("dart pub cache clean {}", pkg_name),
                    removal_warning: String::new(),
                });
            }
        }
    }

    // Deduplicate: keep only the latest version of each package
    packages.sort_by(|a, b| a.name.cmp(&b.name).then(b.version.cmp(&a.version)));
    packages.dedup_by(|a, b| a.name == b.name);
    let total_package_count = packages.len();
    packages.sort_by(|a, b| b.size.cmp(&a.size));

    Some(PackageManagerInfo {
        id: "pub".to_string(),
        name: "pub (Dart)".to_string(),
        version: run_cmd("dart", &["--version"]).split("version: ").nth(1)
            .and_then(|s| s.split_whitespace().next())
            .unwrap_or("").to_string(),
        install_path: pub_cache,
        total_size,
        packages,
        total_package_count,
        detected: true,
        uninstall_hint: "Clear all cached packages: `dart pub cache clean`".to_string(),
        is_custom: false,
        commands_run: vec![],
    })
}

// ---------------------------------------------------------------------------
// gem (Ruby)
// ---------------------------------------------------------------------------

fn scan_gem() -> Option<PackageManagerInfo> {
    let mut trace: Vec<CommandRecord> = Vec::new();

    let (exists, rec) = command_exists_traced("gem");
    trace.push(rec);
    if !exists { return None; }

    let (version, rec) = run_cmd_with_trace("gem", &["--version"], "version");
    trace.push(rec);
    let gem_dir = run_cmd("gem", &["environment", "gemdir"]);

    if gem_dir.is_empty() {
        return None;
    }

    let total_size = dir_size(&gem_dir);

    let list_output = run_cmd("gem", &["list", "--local"]);
    let mut packages: Vec<InstalledPackage> = Vec::new();

    for line in list_output.lines() {
        // Lines like: "bundler (2.5.6, default: 2.5.6)"
        if let Some((name, rest)) = line.split_once(" (") {
            let ver = rest.trim_end_matches(')').split(',').next().unwrap_or("").trim();
            // Skip default gems that ship with Ruby
            if rest.contains("default:") && !rest.contains(',') {
                continue;
            }
            packages.push(InstalledPackage {
                name: name.trim().to_string(),
                version: ver.to_string(),
                size: 0,
                is_top_level: true,
                dependencies: vec![],
                uninstall_command: format!("gem uninstall {}", name.trim()),
                removal_warning: String::new(),
            });
        }
    }

    let total_package_count = packages.len();

    Some(PackageManagerInfo {
        id: "gem".to_string(),
        name: "gem (Ruby)".to_string(),
        version,
        install_path: gem_dir,
        total_size,
        packages,
        total_package_count,
        detected: true,
        uninstall_hint: "Clear gem cache: `gem cleanup`. Uninstall all: `gem uninstall --all`.".to_string(),
        is_custom: false,
        commands_run: trace,
    })
}

// ---------------------------------------------------------------------------
// Gradle
// ---------------------------------------------------------------------------

fn scan_gradle() -> Vec<RuntimeInfo> {
    let mut results = Vec::new();
    let home = home_dir();
    let gradle_home = format!("{}/.gradle", home);

    if !Path::new(&gradle_home).is_dir() {
        return results;
    }

    let total_size = dir_size(&gradle_home);
    let caches_size = dir_size(&format!("{}/caches", gradle_home));
    let wrapper_size = dir_size(&format!("{}/wrapper/dists", gradle_home));

    let mut versions: Vec<RuntimeVersion> = Vec::new();

    // Scan wrapper/dists for installed Gradle distributions
    let dists_dir = format!("{}/wrapper/dists", gradle_home);
    if let Ok(entries) = std::fs::read_dir(&dists_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("gradle-") {
                let ver = name.trim_start_matches("gradle-")
                    .split('-').next().unwrap_or("").to_string();
                let size = dir_size(&entry.path().to_string_lossy());
                versions.push(RuntimeVersion {
                    version: ver,
                    active: false,
                    path: entry.path().to_string_lossy().to_string(),
                    size,
                });
            }
        }
    }

    // Add caches as a visible entry
    if caches_size > 0 {
        versions.push(RuntimeVersion {
            version: format!("Build caches ({})", format_size_approx(caches_size)),
            active: false,
            path: format!("{}/caches", gradle_home),
            size: caches_size,
        });
    }

    versions.sort_by(|a, b| b.size.cmp(&a.size));

    results.push(RuntimeInfo {
        id: "gradle".to_string(),
        name: "Gradle".to_string(),
        install_method: "wrapper".to_string(),
        install_path: gradle_home,
        total_size,
        versions,
        uninstall_hint: format!(
            "Clear caches: `rm -rf ~/.gradle/caches` ({}). Clear wrapper distributions: `rm -rf ~/.gradle/wrapper/dists` ({}).",
            format_size_approx(caches_size),
            format_size_approx(wrapper_size),
        ),
        removal_warning: "Gradle caches speed up Android/JVM builds. Clearing them causes a full re-download on next build.".to_string(),
        is_custom: false,
        commands_run: vec![],
    });

    results
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Format bytes to a rough human-readable string (for use in warning messages).
fn format_size_approx(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.0} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
