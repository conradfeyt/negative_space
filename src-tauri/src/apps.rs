// apps.rs — Application scanning, uninstallation, and leftover detection.
//
// Lists installed .app bundles from /Applications and ~/Applications, computes
// their disk footprint (bundle + leftover Library data), extracts icons, and
// detects install source (Homebrew, App Store, or manual).

use crate::commands::{self, AppInfo, CleanResult};
use std::fs;
use std::path::Path;

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
pub async fn scan_apps(has_fda: Option<bool>) -> Result<Vec<AppInfo>, String> {
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
        let size = commands::get_du_size(&app_path_str);

        // Bundle ID from Info.plist.
        let plist_path = format!("{}/Contents/Info.plist", app_path_str);
        let bundle_id = get_bundle_id(&plist_path);

        // Leftover detection — only with FDA to avoid TCC modal dialogs.
        let (leftover_paths, leftover_size) = if fda {
            let paths = find_leftover_paths(&home, &app_name, &bundle_id);
            let total: u64 = paths.iter().map(|p| commands::get_du_size(p)).sum();
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

/// Move an .app to the Trash via Finder (so it goes to the Trash instead of
/// being permanently deleted), and optionally remove leftover Library files.
#[tauri::command]
pub async fn uninstall_app(app_path: String, remove_leftovers: bool) -> CleanResult {
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
    let app_size = commands::get_du_size(&app_path);

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

            let size = commands::get_du_size(leftover);

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
// Helper functions (not commands — internal to this module)
// ---------------------------------------------------------------------------

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
///   3. Convert .icns -> 64x64 PNG using macOS built-in `sips`
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

    // Step 3: Convert .icns -> 64x64 PNG using `sips` (built into macOS).
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
    // Bundle ID "com.docker.docker" -> look for dirs containing "com.docker".
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
                                // "AltTab.app" -> "AltTab"
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
    // This is imperfect but catches simple cases like "docker" -> "Docker".
    if cask_apps.is_empty() {
        for token in &tokens {
            // Simple title-case: "alt-tab" -> "Alt-Tab"
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
