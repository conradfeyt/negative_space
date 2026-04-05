// browser.rs — Browser data scanning and cleanup module for Negative _.
//
// Detects installed browsers (Safari, Chrome, Firefox, Brave, Edge, Arc,
// Opera, Vivaldi, Chromium) and enumerates their cache, cookies, history,
// sessions, and other cleanable data.
//
// TCC CONSIDERATIONS:
//   - Safari data lives under ~/Library/Safari/ and ~/Library/Cookies/ — both
//     are TCC-protected. Without Full Disk Access, we CANNOT enumerate or size
//     these directories. We use `du -sk` (subprocess) which gives size 0 or
//     an error on TCC-protected dirs without FDA. With FDA, du works fine.
//   - Chrome, Firefox, Brave, Edge, etc. store data under
//     ~/Library/Application Support/<BrowserName>/ — these are generally NOT
//     TCC-protected and can be sized/cleaned without FDA.
//   - We use `du -sk` (subprocess) for sizing, NOT walkdir, to be safe
//     against any TCC edge cases. `du` reports 0 or errors silently on
//     permission-denied paths instead of triggering modal dialogs.
//   - For deletion, we use `rm -rf` (subprocess) for the same TCC safety.

use serde::{Deserialize, Serialize};

// We use helpers from our commands module.
use crate::commands;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Represents a single browser detected on the system.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BrowserInfo {
    /// Internal identifier (e.g. "chrome", "safari", "firefox")
    pub id: String,
    /// Display name (e.g. "Google Chrome", "Safari")
    pub name: String,
    /// Path to the .app bundle (e.g. "/Applications/Google Chrome.app")
    pub app_path: String,
    /// Whether the browser is currently installed
    pub installed: bool,
    /// Individual data categories found for this browser
    pub data_categories: Vec<BrowserDataCategory>,
    /// Total size of all cleanable data in bytes
    pub total_size: u64,
}

/// A single category of browser data (cache, cookies, history, etc.)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BrowserDataCategory {
    /// Internal identifier (e.g. "cache", "cookies", "history")
    pub id: String,
    /// Human-readable label (e.g. "Cache", "Cookies", "Browsing History")
    pub label: String,
    /// Filesystem path(s) that make up this category
    pub paths: Vec<String>,
    /// Total size in bytes
    pub size: u64,
    /// Whether this is safe to clean (some categories need warnings)
    pub safe_to_clean: bool,
    /// Warning text if not fully safe (e.g. "Will log you out of websites")
    pub warning: String,
    /// Whether this path is TCC-protected (needs FDA to access)
    pub tcc_protected: bool,
}

/// Result of a full browser scan.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BrowserScanResult {
    /// All detected browsers with their data categories
    pub browsers: Vec<BrowserInfo>,
    /// Total cleanable size across all browsers
    pub total_size: u64,
    /// Whether Full Disk Access is available (affects Safari scanning)
    pub has_fda: bool,
}

/// Result of cleaning browser data.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BrowserCleanResult {
    /// Whether the overall operation succeeded
    pub success: bool,
    /// Total bytes freed
    pub freed_bytes: u64,
    /// Number of items/paths deleted
    pub deleted_count: u64,
    /// Human-readable error messages for any items that failed
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Browser definitions
// ---------------------------------------------------------------------------

/// Internal struct to define where a browser stores its data.
/// Not serialized — only used internally to drive the scan logic.
struct BrowserDef {
    /// Internal id
    id: &'static str,
    /// Display name
    name: &'static str,
    /// Path to the .app bundle (relative to /Applications or absolute)
    app_path: &'static str,
    /// Function that returns data category definitions given the home directory.
    /// Each tuple: (id, label, paths_relative_to_home, safe_to_clean, warning, tcc_protected)
    categories: fn(&str) -> Vec<CategoryDef>,
}

/// Internal definition for a data category (before resolving sizes).
struct CategoryDef {
    id: &'static str,
    label: &'static str,
    /// Paths are absolute (already resolved with home dir)
    paths: Vec<String>,
    safe_to_clean: bool,
    warning: &'static str,
    tcc_protected: bool,
}

// ---------------------------------------------------------------------------
// Browser data path definitions
// ---------------------------------------------------------------------------
// Each function returns the category definitions for a specific browser.
// Paths are absolute, resolved from the user's home directory.

/// Safari data paths.
/// IMPORTANT: Safari data is under ~/Library/Safari/ and ~/Library/Cookies/ —
/// both TCC-protected. Without FDA, `du -sk` returns 0 or errors out.
fn safari_categories(home: &str) -> Vec<CategoryDef> {
    vec![
        CategoryDef {
            id: "cache",
            label: "Cache",
            paths: vec![
                format!("{}/Library/Caches/com.apple.Safari", home),
                format!("{}/Library/Caches/com.apple.Safari.SafeBrowsing", home),
                // Safari also uses WebKit shared caches
                format!("{}/Library/Caches/com.apple.WebKit.Networking", home),
            ],
            safe_to_clean: true,
            warning: "",
            tcc_protected: true,
        },
        CategoryDef {
            id: "history",
            label: "Browsing History",
            paths: vec![
                format!("{}/Library/Safari/History.db", home),
                format!("{}/Library/Safari/History.db-shm", home),
                format!("{}/Library/Safari/History.db-wal", home),
            ],
            safe_to_clean: false,
            warning: "Will permanently delete your Safari browsing history",
            tcc_protected: true,
        },
        CategoryDef {
            id: "cookies",
            label: "Cookies",
            paths: vec![format!("{}/Library/Cookies/Cookies.binarycookies", home)],
            safe_to_clean: false,
            warning: "Will log you out of all websites in Safari",
            tcc_protected: true,
        },
        CategoryDef {
            id: "local_storage",
            label: "Website Data",
            paths: vec![
                format!("{}/Library/Safari/LocalStorage", home),
                format!("{}/Library/Safari/Databases", home),
            ],
            safe_to_clean: false,
            warning: "Will clear saved website data and preferences",
            tcc_protected: true,
        },
    ]
}

/// Google Chrome data paths.
/// Chrome stores everything under ~/Library/Application Support/Google/Chrome/
/// which is NOT TCC-protected — safe to scan without FDA.
fn chrome_categories(home: &str) -> Vec<CategoryDef> {
    let chrome_base = format!("{}/Library/Application Support/Google/Chrome", home);
    let chrome_cache = format!("{}/Library/Caches/Google/Chrome", home);
    let mut cache_paths = discover_chromium_profile_caches(&chrome_cache);
    cache_paths.extend([
        format!("{}/Default/Service Worker/CacheStorage", chrome_base),
        format!("{}/Default/Code Cache", chrome_base),
        format!("{}/Default/GPUCache", chrome_base),
        format!("{}/ShaderCache", chrome_base),
        format!("{}/GrShaderCache", chrome_base),
    ]);
    vec![
        CategoryDef {
            id: "cache",
            label: "Cache",
            paths: cache_paths,
            safe_to_clean: true,
            warning: "",
            tcc_protected: false,
        },
        CategoryDef {
            id: "history",
            label: "Browsing History",
            paths: vec![
                format!("{}/Default/History", chrome_base),
                format!("{}/Default/History-journal", chrome_base),
                format!("{}/Default/Visited Links", chrome_base),
                format!("{}/Default/Top Sites", chrome_base),
            ],
            safe_to_clean: false,
            warning: "Will permanently delete your Chrome browsing history",
            tcc_protected: false,
        },
        CategoryDef {
            id: "cookies",
            label: "Cookies",
            paths: vec![
                format!("{}/Default/Cookies", chrome_base),
                format!("{}/Default/Cookies-journal", chrome_base),
            ],
            safe_to_clean: false,
            warning: "Will log you out of all websites in Chrome",
            tcc_protected: false,
        },
        CategoryDef {
            id: "session",
            label: "Session Data",
            paths: vec![
                format!("{}/Default/Sessions", chrome_base),
                format!("{}/Default/Session Storage", chrome_base),
            ],
            safe_to_clean: false,
            warning: "Will close all saved tabs and sessions",
            tcc_protected: false,
        },
    ]
}

/// Mozilla Firefox data paths.
/// Firefox uses ~/Library/Application Support/Firefox/Profiles/<random>.default*/
/// We need to discover the profile directory first.
fn firefox_categories(home: &str) -> Vec<CategoryDef> {
    let firefox_base = format!("{}/Library/Application Support/Firefox", home);
    let firefox_cache = format!("{}/Library/Caches/Firefox", home);

    // Discover Firefox profile directories.
    // Firefox names them like "abc123.default-release" or "xyz789.default".
    let profiles = discover_firefox_profiles(&firefox_base);

    let mut cache_paths = vec![firefox_cache.clone()];
    let mut history_paths = Vec::new();
    let mut cookie_paths = Vec::new();
    let mut session_paths = Vec::new();

    for profile in &profiles {
        cache_paths.push(format!("{}/cache2", profile));
        cache_paths.push(format!("{}/startupCache", profile));
        history_paths.push(format!("{}/places.sqlite", profile));
        history_paths.push(format!("{}/places.sqlite-wal", profile));
        cookie_paths.push(format!("{}/cookies.sqlite", profile));
        cookie_paths.push(format!("{}/cookies.sqlite-wal", profile));
        session_paths.push(format!("{}/sessionstore-backups", profile));
        session_paths.push(format!("{}/sessionstore.jsonlz4", profile));
    }

    vec![
        CategoryDef {
            id: "cache",
            label: "Cache",
            paths: cache_paths,
            safe_to_clean: true,
            warning: "",
            tcc_protected: false,
        },
        CategoryDef {
            id: "history",
            label: "Browsing History",
            paths: history_paths,
            safe_to_clean: false,
            warning: "Will permanently delete your Firefox browsing history",
            tcc_protected: false,
        },
        CategoryDef {
            id: "cookies",
            label: "Cookies",
            paths: cookie_paths,
            safe_to_clean: false,
            warning: "Will log you out of all websites in Firefox",
            tcc_protected: false,
        },
        CategoryDef {
            id: "session",
            label: "Session Data",
            paths: session_paths,
            safe_to_clean: false,
            warning: "Will remove saved tab sessions and session backups",
            tcc_protected: false,
        },
    ]
}

/// Brave browser data paths.
/// Brave is Chromium-based and uses the same layout as Chrome, under a
/// different directory name.
fn brave_categories(home: &str) -> Vec<CategoryDef> {
    let brave_base = format!(
        "{}/Library/Application Support/BraveSoftware/Brave-Browser",
        home
    );
    let brave_cache = format!("{}/Library/Caches/BraveSoftware/Brave-Browser", home);
    let mut cache_paths = discover_chromium_profile_caches(&brave_cache);
    cache_paths.extend([
        format!("{}/Default/Service Worker/CacheStorage", brave_base),
        format!("{}/Default/Code Cache", brave_base),
        format!("{}/Default/GPUCache", brave_base),
        format!("{}/ShaderCache", brave_base),
    ]);
    vec![
        CategoryDef {
            id: "cache",
            label: "Cache",
            paths: cache_paths,
            safe_to_clean: true,
            warning: "",
            tcc_protected: false,
        },
        CategoryDef {
            id: "history",
            label: "Browsing History",
            paths: vec![
                format!("{}/Default/History", brave_base),
                format!("{}/Default/History-journal", brave_base),
            ],
            safe_to_clean: false,
            warning: "Will permanently delete your Brave browsing history",
            tcc_protected: false,
        },
        CategoryDef {
            id: "cookies",
            label: "Cookies",
            paths: vec![
                format!("{}/Default/Cookies", brave_base),
                format!("{}/Default/Cookies-journal", brave_base),
            ],
            safe_to_clean: false,
            warning: "Will log you out of all websites in Brave",
            tcc_protected: false,
        },
        CategoryDef {
            id: "session",
            label: "Session Data",
            paths: vec![
                format!("{}/Default/Sessions", brave_base),
                format!("{}/Default/Session Storage", brave_base),
            ],
            safe_to_clean: false,
            warning: "Will close all saved tabs and sessions",
            tcc_protected: false,
        },
    ]
}

/// Microsoft Edge data paths (Chromium-based, same layout as Chrome).
fn edge_categories(home: &str) -> Vec<CategoryDef> {
    let edge_base = format!("{}/Library/Application Support/Microsoft Edge", home);
    let edge_cache = format!("{}/Library/Caches/Microsoft Edge", home);
    let mut cache_paths = discover_chromium_profile_caches(&edge_cache);
    cache_paths.extend([
        format!("{}/Default/Service Worker/CacheStorage", edge_base),
        format!("{}/Default/Code Cache", edge_base),
        format!("{}/Default/GPUCache", edge_base),
        format!("{}/ShaderCache", edge_base),
    ]);
    vec![
        CategoryDef {
            id: "cache",
            label: "Cache",
            paths: cache_paths,
            safe_to_clean: true,
            warning: "",
            tcc_protected: false,
        },
        CategoryDef {
            id: "history",
            label: "Browsing History",
            paths: vec![
                format!("{}/Default/History", edge_base),
                format!("{}/Default/History-journal", edge_base),
            ],
            safe_to_clean: false,
            warning: "Will permanently delete your Edge browsing history",
            tcc_protected: false,
        },
        CategoryDef {
            id: "cookies",
            label: "Cookies",
            paths: vec![
                format!("{}/Default/Cookies", edge_base),
                format!("{}/Default/Cookies-journal", edge_base),
            ],
            safe_to_clean: false,
            warning: "Will log you out of all websites in Edge",
            tcc_protected: false,
        },
        CategoryDef {
            id: "session",
            label: "Session Data",
            paths: vec![
                format!("{}/Default/Sessions", edge_base),
                format!("{}/Default/Session Storage", edge_base),
            ],
            safe_to_clean: false,
            warning: "Will close all saved tabs and sessions",
            tcc_protected: false,
        },
    ]
}

/// Arc browser data paths (Chromium-based).
fn arc_categories(home: &str) -> Vec<CategoryDef> {
    let arc_base = format!("{}/Library/Application Support/Arc", home);
    let arc_cache = format!("{}/Library/Caches/company.thebrowser.Browser", home);
    vec![
        CategoryDef {
            id: "cache",
            label: "Cache",
            paths: vec![
                arc_cache.clone(),
                format!("{}/User Data/Default/Service Worker/CacheStorage", arc_base),
                format!("{}/User Data/Default/Code Cache", arc_base),
                format!("{}/User Data/Default/GPUCache", arc_base),
            ],
            safe_to_clean: true,
            warning: "",
            tcc_protected: false,
        },
        CategoryDef {
            id: "history",
            label: "Browsing History",
            paths: vec![
                format!("{}/User Data/Default/History", arc_base),
                format!("{}/User Data/Default/History-journal", arc_base),
            ],
            safe_to_clean: false,
            warning: "Will permanently delete your Arc browsing history",
            tcc_protected: false,
        },
        CategoryDef {
            id: "cookies",
            label: "Cookies",
            paths: vec![
                format!("{}/User Data/Default/Cookies", arc_base),
                format!("{}/User Data/Default/Cookies-journal", arc_base),
            ],
            safe_to_clean: false,
            warning: "Will log you out of all websites in Arc",
            tcc_protected: false,
        },
        CategoryDef {
            id: "session",
            label: "Session Data",
            paths: vec![
                format!("{}/User Data/Default/Sessions", arc_base),
                format!("{}/User Data/Default/Session Storage", arc_base),
            ],
            safe_to_clean: false,
            warning: "Will close all saved tabs and sessions",
            tcc_protected: false,
        },
    ]
}

/// Opera browser data paths (Chromium-based).
fn opera_categories(home: &str) -> Vec<CategoryDef> {
    let opera_base = format!(
        "{}/Library/Application Support/com.operasoftware.Opera",
        home
    );
    let opera_cache = format!("{}/Library/Caches/com.operasoftware.Opera", home);
    let mut cache_paths = discover_chromium_profile_caches(&opera_cache);
    cache_paths.extend([
        format!("{}/Default/Service Worker/CacheStorage", opera_base),
        format!("{}/Default/Code Cache", opera_base),
        format!("{}/Default/GPUCache", opera_base),
    ]);
    vec![
        CategoryDef {
            id: "cache",
            label: "Cache",
            paths: cache_paths,
            safe_to_clean: true,
            warning: "",
            tcc_protected: false,
        },
        CategoryDef {
            id: "history",
            label: "Browsing History",
            paths: vec![
                format!("{}/Default/History", opera_base),
                format!("{}/Default/History-journal", opera_base),
            ],
            safe_to_clean: false,
            warning: "Will permanently delete your Opera browsing history",
            tcc_protected: false,
        },
        CategoryDef {
            id: "cookies",
            label: "Cookies",
            paths: vec![
                format!("{}/Default/Cookies", opera_base),
                format!("{}/Default/Cookies-journal", opera_base),
            ],
            safe_to_clean: false,
            warning: "Will log you out of all websites in Opera",
            tcc_protected: false,
        },
    ]
}

/// Vivaldi browser data paths (Chromium-based).
fn vivaldi_categories(home: &str) -> Vec<CategoryDef> {
    let vivaldi_base = format!("{}/Library/Application Support/Vivaldi", home);
    let vivaldi_cache = format!("{}/Library/Caches/Vivaldi", home);
    let mut cache_paths = discover_chromium_profile_caches(&vivaldi_cache);
    cache_paths.extend([
        format!("{}/Default/Service Worker/CacheStorage", vivaldi_base),
        format!("{}/Default/Code Cache", vivaldi_base),
        format!("{}/Default/GPUCache", vivaldi_base),
    ]);
    vec![
        CategoryDef {
            id: "cache",
            label: "Cache",
            paths: cache_paths,
            safe_to_clean: true,
            warning: "",
            tcc_protected: false,
        },
        CategoryDef {
            id: "history",
            label: "Browsing History",
            paths: vec![
                format!("{}/Default/History", vivaldi_base),
                format!("{}/Default/History-journal", vivaldi_base),
            ],
            safe_to_clean: false,
            warning: "Will permanently delete your Vivaldi browsing history",
            tcc_protected: false,
        },
        CategoryDef {
            id: "cookies",
            label: "Cookies",
            paths: vec![
                format!("{}/Default/Cookies", vivaldi_base),
                format!("{}/Default/Cookies-journal", vivaldi_base),
            ],
            safe_to_clean: false,
            warning: "Will log you out of all websites in Vivaldi",
            tcc_protected: false,
        },
    ]
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Discover per-profile Cache directories inside a Chromium-based browser's Caches dir.
/// Chromium stores caches under `<cache_dir>/Default/Cache`, `<cache_dir>/Profile 1/Cache`, etc.
/// Returning only the `Cache` subdir (not the whole profile dir) avoids "Directory not empty"
/// errors when the browser is running and has other profile files locked open.
fn discover_chromium_profile_caches(cache_dir: &str) -> Vec<String> {
    let output = std::process::Command::new("ls").arg(cache_dir).output();
    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            text.lines()
                .filter(|line| !line.is_empty())
                .filter_map(|line| {
                    let cache_subdir = format!("{}/{}/Cache", cache_dir, line.trim());
                    let exists = std::process::Command::new("test")
                        .args(["-d", &cache_subdir])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false);
                    if exists { Some(cache_subdir) } else { None }
                })
                .collect()
        }
        _ => vec![],
    }
}

/// Discover Firefox profile directories.
/// Firefox profiles live under ~/Library/Application Support/Firefox/Profiles/
/// and are named like "abc123.default-release" or "xyz789.default".
fn discover_firefox_profiles(firefox_base: &str) -> Vec<String> {
    let profiles_dir = format!("{}/Profiles", firefox_base);

    // Use `ls` subprocess to list profile directories — safe from TCC.
    let output = std::process::Command::new("ls").arg(&profiles_dir).output();

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            text.lines()
                .filter(|line| !line.is_empty())
                .map(|line| format!("{}/{}", profiles_dir, line.trim()))
                .filter(|path| {
                    // Verify it's actually a directory via subprocess.
                    std::process::Command::new("test")
                        .args(["-d", path])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false)
                })
                .collect()
        }
        _ => vec![],
    }
}

/// Get the size of a path using `du -sk` (subprocess — TCC-safe).
/// Returns size in bytes. Returns 0 if the path doesn't exist or can't be read.
fn get_du_size(path: &str) -> u64 {
    crate::commands::get_du_size(path)
}

/// Check if a path exists using `test -e` (subprocess — TCC-safe).
fn path_exists(path: &str) -> bool {
    std::process::Command::new("test")
        .args(["-e", path])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if a .app bundle is installed by checking its existence.
fn is_app_installed(app_path: &str) -> bool {
    std::process::Command::new("test")
        .args(["-d", app_path])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Scan a single browser data category, resolving paths and computing sizes.
///
/// Returns `Some((category, size, has_nonzero_data))` if the category should be
/// included in the result, or `None` if it should be skipped (no paths exist and
/// not TCC-protected).
fn scan_browser_category(cat_def: &CategoryDef, fda: bool) -> Option<(BrowserDataCategory, u64, bool)> {
    // For TCC-protected paths: skip sizing if we don't have FDA.
    if cat_def.tcc_protected && !fda {
        let category = BrowserDataCategory {
            id: cat_def.id.to_string(),
            label: cat_def.label.to_string(),
            paths: cat_def.paths.clone(),
            size: 0,
            safe_to_clean: cat_def.safe_to_clean,
            warning: cat_def.warning.to_string(),
            tcc_protected: true,
        };
        return Some((category, 0, false));
    }

    // Sum up sizes of all paths in this category.
    let mut cat_size: u64 = 0;
    let mut existing_paths: Vec<String> = Vec::new();
    let mut has_nonzero_data = false;

    for path in &cat_def.paths {
        if path_exists(path) {
            let size = get_du_size(path);
            cat_size += size;
            existing_paths.push(path.clone());
            if size > 0 {
                has_nonzero_data = true;
            }
        }
    }

    // Only include the category if at least one path exists or it's TCC-protected.
    if existing_paths.is_empty() && !cat_def.tcc_protected {
        return None;
    }

    let category = BrowserDataCategory {
        id: cat_def.id.to_string(),
        label: cat_def.label.to_string(),
        paths: if existing_paths.is_empty() {
            cat_def.paths.clone()
        } else {
            existing_paths
        },
        size: cat_size,
        safe_to_clean: cat_def.safe_to_clean,
        warning: cat_def.warning.to_string(),
        tcc_protected: cat_def.tcc_protected,
    };

    Some((category, cat_size, has_nonzero_data))
}

/// Run a full browser scan — detect installed browsers and enumerate their data.
///
/// The `fda` parameter indicates whether we have Full Disk Access. This affects
/// whether we can scan Safari's data (which is TCC-protected).
pub fn run_browser_scan(fda: bool) -> BrowserScanResult {
    let home = match commands::home_dir() {
        Some(h) => h,
        None => {
            return BrowserScanResult {
                browsers: vec![],
                total_size: 0,
                has_fda: fda,
            };
        }
    };

    // Define all known browsers.
    let browser_defs: Vec<BrowserDef> = vec![
        BrowserDef {
            id: "safari",
            name: "Safari",
            app_path: "/Applications/Safari.app",
            categories: safari_categories,
        },
        BrowserDef {
            id: "chrome",
            name: "Google Chrome",
            app_path: "/Applications/Google Chrome.app",
            categories: chrome_categories,
        },
        BrowserDef {
            id: "firefox",
            name: "Firefox",
            app_path: "/Applications/Firefox.app",
            categories: firefox_categories,
        },
        BrowserDef {
            id: "brave",
            name: "Brave Browser",
            app_path: "/Applications/Brave Browser.app",
            categories: brave_categories,
        },
        BrowserDef {
            id: "edge",
            name: "Microsoft Edge",
            app_path: "/Applications/Microsoft Edge.app",
            categories: edge_categories,
        },
        BrowserDef {
            id: "arc",
            name: "Arc",
            app_path: "/Applications/Arc.app",
            categories: arc_categories,
        },
        BrowserDef {
            id: "opera",
            name: "Opera",
            app_path: "/Applications/Opera.app",
            categories: opera_categories,
        },
        BrowserDef {
            id: "vivaldi",
            name: "Vivaldi",
            app_path: "/Applications/Vivaldi.app",
            categories: vivaldi_categories,
        },
    ];

    let mut browsers: Vec<BrowserInfo> = Vec::new();
    let mut grand_total: u64 = 0;

    for def in &browser_defs {
        let installed = is_app_installed(def.app_path);

        // Even if the browser isn't installed, data directories might remain
        // (leftover data from a previous installation). We check anyway.
        let cat_defs = (def.categories)(&home);
        let mut data_categories: Vec<BrowserDataCategory> = Vec::new();
        let mut browser_total: u64 = 0;
        let mut has_any_data = false;

        for cat_def in cat_defs {
            if let Some((category, cat_size, cat_has_data)) = scan_browser_category(&cat_def, fda) {
                if cat_has_data {
                    has_any_data = true;
                }
                browser_total += cat_size;
                data_categories.push(category);
            }
        }

        grand_total += browser_total;

        // Include the browser if it's installed OR has leftover data.
        if installed || has_any_data {
            browsers.push(BrowserInfo {
                id: def.id.to_string(),
                name: def.name.to_string(),
                app_path: def.app_path.to_string(),
                installed,
                data_categories,
                total_size: browser_total,
            });
        }
    }

    // Sort: installed browsers first (by total size desc), then uninstalled
    browsers.sort_by(|a, b| {
        b.installed
            .cmp(&a.installed)
            .then_with(|| b.total_size.cmp(&a.total_size))
    });

    BrowserScanResult {
        browsers,
        total_size: grand_total,
        has_fda: fda,
    }
}

/// Clean specific browser data paths.
///
/// Takes a list of absolute paths to delete. Uses `rm -rf` subprocess for
/// TCC safety — even if a path is in a TCC-protected area, the subprocess
/// won't trigger a modal dialog in our app thread (it'll just fail silently).
///
/// IMPORTANT: The caller (frontend) should confirm with the user before
/// calling this, especially for non-safe categories (cookies, history).
pub fn clean_browser_data(paths: Vec<String>) -> BrowserCleanResult {
    let mut freed_bytes: u64 = 0;
    let mut deleted_count: u64 = 0;
    let mut errors: Vec<String> = Vec::new();

    for path_str in &paths {
        // Check if the path exists first.
        if !path_exists(path_str) {
            // Not an error — the path may have already been cleaned.
            continue;
        }

        // Measure size before deletion so we can report freed space.
        let size = get_du_size(path_str);

        // Use `rm -rf` as a subprocess — TCC-safe.
        // For files (like Cookies database), rm -f works.
        // For directories (like cache dirs), rm -rf works.
        let result = std::process::Command::new("rm")
            .args(["-rf", path_str])
            .output();

        match result {
            Ok(o) if o.status.success() => {
                freed_bytes += size;
                deleted_count += 1;
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                if !stderr.trim().is_empty() {
                    errors.push(format!("Failed to delete {}: {}", path_str, stderr.trim()));
                }
            }
            Err(e) => {
                errors.push(format!("Failed to run rm on {}: {}", path_str, e));
            }
        }
    }

    BrowserCleanResult {
        success: errors.is_empty(),
        freed_bytes,
        deleted_count,
        errors,
    }
}
