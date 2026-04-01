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

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A detected package manager (e.g. Homebrew, pip, npm).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PackageManagerInfo {
    /// Identifier: "homebrew", "pip", "npm", "cargo", etc.
    pub id: String,
    /// Human-readable name: "Homebrew", "pip (Python)", etc.
    pub name: String,
    /// Version string of the manager itself.
    pub version: String,
    /// Where the manager stores data on disk.
    pub install_path: String,
    /// Total disk usage of the manager's data in bytes.
    pub total_size: u64,
    /// Top-level (explicitly installed) packages.
    pub packages: Vec<InstalledPackage>,
    /// Number of total packages including dependencies.
    pub total_package_count: usize,
    /// Whether this manager was detected on the system.
    pub detected: bool,
    /// Shell command to remove the manager itself (informational).
    pub uninstall_hint: String,
}

/// A single installed package.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstalledPackage {
    /// Package name.
    pub name: String,
    /// Installed version.
    pub version: String,
    /// Size in bytes (0 if unknown).
    pub size: u64,
    /// Whether this was explicitly installed vs pulled in as a dependency.
    pub is_top_level: bool,
    /// Direct dependency names (populated on demand for some managers).
    pub dependencies: Vec<String>,
    /// Shell command to remove this package.
    pub uninstall_command: String,
    /// Consequences of removal — what might break.
    pub removal_warning: String,
}

/// A detected runtime or language installation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RuntimeInfo {
    /// Identifier: "java", "node-nvm", "rust", "go", "flutter", etc.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// How it was installed: "homebrew", "nvm", "rustup", "manual", etc.
    pub install_method: String,
    /// Where it lives on disk.
    pub install_path: String,
    /// Total disk usage in bytes.
    pub total_size: u64,
    /// Installed versions (for version managers that support multiple).
    pub versions: Vec<RuntimeVersion>,
    /// Shell command to remove it entirely (informational).
    pub uninstall_hint: String,
    /// What might break if removed.
    pub removal_warning: String,
}

/// A specific version of a runtime.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RuntimeVersion {
    /// Version string.
    pub version: String,
    /// Whether this is the currently active/default version.
    pub active: bool,
    /// Path to this version's installation directory.
    pub path: String,
    /// Size in bytes.
    pub size: u64,
}

/// Complete scan result returned to the frontend.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PackageScanResult {
    /// All detected package managers.
    pub managers: Vec<PackageManagerInfo>,
    /// All detected runtimes / language installations.
    pub runtimes: Vec<RuntimeInfo>,
    /// Total disk usage across everything.
    pub total_size: u64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get user's home directory.
fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/Users/unknown".to_string())
}

/// Run a command and return stdout as a trimmed String, or empty on failure.
fn run_cmd(program: &str, args: &[&str]) -> String {
    match Command::new(program).args(args).output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => String::new(),
    }
}

/// Check if a command exists on the system.
fn command_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get directory size via `du -sk`. Returns bytes.
fn dir_size(path: &str) -> u64 {
    let output = Command::new("du").args(["-sk", path]).output();
    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            text.split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0)
                * 1024
        }
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Main scan entry point
// ---------------------------------------------------------------------------

/// Run a full package and runtime scan. Detects everything present on the system.
pub fn scan_all() -> PackageScanResult {
    let mut managers: Vec<PackageManagerInfo> = Vec::new();
    let mut runtimes: Vec<RuntimeInfo> = Vec::new();

    // Package managers
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

    // FUTURE: MacPorts, Nix, Conda, gem, Composer, etc.

    // Runtimes
    runtimes.extend(scan_java_runtimes());
    runtimes.extend(scan_nvm());
    runtimes.extend(scan_rust_toolchains());
    runtimes.extend(scan_go_runtime());
    runtimes.extend(scan_flutter_runtime());

    // FUTURE: pyenv, rbenv, rvm, sdkman, volta, fnm, mise, asdf, .NET, etc.

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
    if !command_exists("brew") {
        return None;
    }

    let version = run_cmd("brew", &["--version"]);
    // First line: "Homebrew 4.5.2"
    let version = version.lines().next().unwrap_or("").to_string();

    let prefix = run_cmd("brew", &["--prefix"]);
    let cellar_path = format!("{}/Cellar", prefix);
    let caskroom_path = format!("{}/Caskroom", prefix);

    // Total size = Cellar + Caskroom + anything else under prefix.
    // We size just Cellar + Caskroom to avoid counting system-level stuff.
    let cellar_size = dir_size(&cellar_path);
    let caskroom_size = dir_size(&caskroom_path);
    let total_size = cellar_size + caskroom_size;

    // Top-level formulae (explicitly installed, not deps).
    let leaves_output = run_cmd("brew", &["leaves"]);
    let leaves: std::collections::HashSet<String> = leaves_output
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // All installed formulae with versions.
    let formulae_output = run_cmd("brew", &["list", "--formulae", "--versions"]);
    // Format: "git 2.45.0" or "python@3.13 3.13.0"

    // Get dependency info: "pkg: dep1 dep2 dep3" per line.
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

    // Also list casks.
    let casks_output = run_cmd("brew", &["list", "--cask", "--versions"]);
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
    if !command_exists("pip3") {
        return None;
    }

    let version_output = run_cmd("pip3", &["--version"]);
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

    // Top-level packages (not required by anything else).
    let top_level_output = run_cmd("pip3", &["list", "--not-required", "--format=json"]);
    let all_output = run_cmd("pip3", &["list", "--format=json"]);

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
    if !command_exists("npm") {
        return None;
    }

    let version = run_cmd("npm", &["--version"]);
    let global_root = run_cmd("npm", &["root", "-g"]);
    // e.g. "/opt/homebrew/lib/node_modules" or "~/.nvm/versions/node/v20/lib/node_modules"

    let total_size = if !global_root.is_empty() {
        dir_size(&global_root)
    } else {
        0
    };

    // npm list -g --depth=0 --json gives us the top-level global packages.
    let list_output = run_cmd("npm", &["list", "-g", "--depth=0", "--json"]);

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
    })
}

// ---------------------------------------------------------------------------
// Cargo (Rust global installs)
// ---------------------------------------------------------------------------

/// Detect cargo and list globally installed binaries.
fn scan_cargo() -> Option<PackageManagerInfo> {
    let home = home_dir();
    let cargo_path = format!("{}/.cargo", home);

    if !Path::new(&cargo_path).is_dir() {
        return None;
    }

    let version = run_cmd("cargo", &["--version"]);
    // "cargo 1.82.0 (8f40fc59f 2024-08-21)"
    let version = version.split_whitespace().nth(1).unwrap_or("").to_string();

    let total_size = dir_size(&cargo_path);

    // `cargo install --list` outputs lines like:
    // "package-name v1.0.0:"
    // "    binary-name"
    let list_output = run_cmd("cargo", &["install", "--list"]);

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
    })
}

// ---------------------------------------------------------------------------
// Java runtimes
// ---------------------------------------------------------------------------

/// Detect installed Java Virtual Machines via /usr/libexec/java_home.
fn scan_java_runtimes() -> Vec<RuntimeInfo> {
    let mut results = Vec::new();

    // /usr/libexec/java_home -V writes to STDERR (yes, really).
    let output = Command::new("/usr/libexec/java_home").arg("-V").output();

    let text = match output {
        Ok(o) => {
            // java_home writes the JVM list to stderr.
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            stderr
        }
        _ => return results,
    };

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
    let home = home_dir();
    let rustup_dir = format!("{}/.rustup", home);

    if !Path::new(&rustup_dir).is_dir() {
        return results;
    }

    let total_size = dir_size(&rustup_dir);

    // Get installed toolchains.
    let toolchains_output = run_cmd("rustup", &["toolchain", "list"]);
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

    if !command_exists("go") {
        return results;
    }

    let version = run_cmd("go", &["version"]);
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
    });

    results
}

// ---------------------------------------------------------------------------
// Flutter / Dart
// ---------------------------------------------------------------------------

/// Detect Flutter SDK installation.
fn scan_flutter_runtime() -> Vec<RuntimeInfo> {
    let mut results = Vec::new();

    if !command_exists("flutter") {
        return results;
    }

    let version_output = run_cmd("flutter", &["--version"]);
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
