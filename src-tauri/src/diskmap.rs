// diskmap.rs — Disk space visualization data for Negative _.
//
// Builds a tree of directory sizes for the treemap visualization.
// Each node represents a directory with its total size and top children.
//
// RUST CONCEPT: This is a module — declared in lib.rs with `mod diskmap;`.
//
// TCC CONSIDERATIONS:
//   We use `du -sk` (subprocess) for sizing individual directories. This is
//   TCC-safe — it returns 0 or errors silently on protected directories
//   without triggering modal permission dialogs.
//
//   For the top-level scan, we enumerate known directories rather than
//   walking from ~. Without FDA, we show only accessible directories.
//   With FDA, we show everything.
//
// DESIGN:
//   Instead of doing a deep recursive walk (expensive), we do a two-level
//   approach:
//   1. Size the top-level directories under ~ (and a few system locations)
//   2. For the largest directories, size their immediate children
//   This gives the treemap enough detail without excessive I/O.

use serde::{Deserialize, Serialize};

use crate::commands;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A single node in the disk usage tree.
/// Represents either a directory or a "rest" bucket for small items.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DiskNode {
    /// Display name (directory name, or "Other" for the rollup)
    pub name: String,
    /// Absolute path (empty for synthetic nodes like "Other")
    pub path: String,
    /// Total size in bytes
    pub size: u64,
    /// Whether this node has been expanded (has children populated)
    pub expanded: bool,
    /// Child nodes (only populated for expanded directories)
    pub children: Vec<DiskNode>,
    /// Color hint for the treemap (category-based)
    pub category: String,
    /// Last-modified time as Unix timestamp (seconds since epoch).
    /// `None` until enriched by the async `enrich_disk_nodes` command.
    /// Used for the "Recency" color overlay in the sunburst visualization.
    pub modified: Option<u64>,
}

/// Result of a disk map scan.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DiskMapResult {
    /// Root node (typically ~ or /)
    pub root: DiskNode,
    /// Total disk size from df
    pub disk_total: u64,
    /// Used disk space from df
    pub disk_used: u64,
    /// Free disk space from df
    pub disk_free: u64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get the size of a directory using `du -sk` (subprocess — TCC-safe).
fn du_size(path: &str) -> u64 {
    let output = std::process::Command::new("du")
        .args(["-sk", path])
        .output();
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

/// Check if a directory exists via subprocess.
fn dir_exists(path: &str) -> bool {
    std::process::Command::new("test")
        .args(["-d", path])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// List immediate subdirectories of a path using `ls` subprocess.
/// Returns a list of (name, full_path) pairs.
fn list_subdirs(path: &str) -> Vec<(String, String)> {
    let output = std::process::Command::new("ls").args(["-1", path]).output();

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            text.lines()
                .filter(|line| !line.is_empty())
                .map(|name| {
                    let full = format!("{}/{}", path, name.trim());
                    (name.trim().to_string(), full)
                })
                .filter(|(_, full)| dir_exists(full))
                .collect()
        }
        _ => vec![],
    }
}

/// Categorize a directory for treemap coloring.
/// Categories align with macOS System Settings > Storage where possible.
fn categorize(name: &str, path: &str) -> String {
    let lower = name.to_lowercase();

    // Applications
    if lower == "applications" || lower.ends_with(".app") {
        return "applications".to_string();
    }

    // Documents (Desktop, Documents, Downloads)
    if lower == "desktop" || lower == "documents" || lower == "downloads" {
        return "documents".to_string();
    }

    // Developer tools
    if lower == "developer"
        || lower == "deriveddata"
        || lower == "coresimulator"
        || path.contains("/Developer/")
        || lower == ".cargo"
        || lower == ".rustup"
        || lower == ".npm"
        || lower == ".gradle"
        || lower == ".m2"
        || lower == "node_modules"
    {
        return "developer".to_string();
    }

    // Bin / Trash
    if lower == ".trash" {
        return "bin".to_string();
    }

    // Books (matches macOS "Books" category)
    if lower == "books"
        || lower == "audiobooks"
        || path.contains("/BKAgentService")
        || path.contains("/com.apple.BKAgentService")
    {
        return "books".to_string();
    }

    // iCloud Drive
    if lower == "mobile documents"
        || path.contains("/Mobile Documents")
    {
        return "icloud".to_string();
    }

    // Mail
    if lower == "mail"
        || path.contains("/com.apple.mail")
        || (path.contains("/Library/Mail") && !path.contains("/Library/Mail Downloads"))
    {
        return "mail".to_string();
    }

    // Photos
    if lower == "photos library.photoslibrary"
        || path.contains("/Photos Library.photoslibrary")
        || path.contains("/com.apple.Photos")
    {
        return "photos".to_string();
    }

    // Music
    if lower == "music" {
        return "music".to_string();
    }

    // Music Creation (GarageBand, Logic)
    if path.contains("/GarageBand")
        || path.contains("/Logic Pro")
        || path.contains("/com.apple.garageband")
        || path.contains("/Library/Audio")
    {
        return "music_creation".to_string();
    }

    // Messages
    if lower == "messages"
        || path.contains("/com.apple.MobileSMS")
        || path.contains("/Library/Messages")
    {
        return "messages".to_string();
    }

    // Podcasts
    if path.contains("/com.apple.podcasts")
        || path.contains("/Podcasts")
    {
        return "podcasts".to_string();
    }

    // iOS Files (device backups)
    if path.contains("/MobileSync/Backup")
        || path.contains("/com.apple.iTunes")
    {
        return "ios_files".to_string();
    }

    // TV / Movies
    if lower == "movies"
        || path.contains("/com.apple.TV")
    {
        return "media".to_string();
    }

    // Pictures (not Photos library) goes to documents
    if lower == "pictures" {
        return "documents".to_string();
    }

    // Other Users & Shared
    if path.starts_with("/Users/Shared")
        || (path.starts_with("/Users/") && !path.starts_with(&format!("/Users/{}", lower)))
    {
        return "other_users".to_string();
    }

    // macOS system files
    if path.starts_with("/System")
        || path.starts_with("/usr")
        || path.starts_with("/sbin")
        || path.starts_with("/bin")
        || (lower == "system" && path.contains("/System"))
    {
        return "macos".to_string();
    }

    // Docker
    if lower == ".docker" || lower == "docker" || lower.contains("docker") {
        return "docker".to_string();
    }

    // Caches (explicit cache dirs)
    if lower == "caches" || lower == ".cache" || lower == "cache" {
        return "caches".to_string();
    }

    // System Data (Library, hidden dirs, everything system-level under ~)
    if lower == "library"
        || lower.starts_with(".")
        || path.starts_with("/Library")
    {
        return "system_data".to_string();
    }

    "other".to_string()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build a disk usage map for the treemap visualization.
///
/// Strategy:
///   1. Get disk-level totals from `df`
///   2. Size the top-level directories under ~
///   3. For the top N largest directories, size their immediate children
///   4. Roll up small items into "Other" nodes
///
/// `fda`: whether Full Disk Access is available.
/// `depth`: how many levels deep to expand (1 = top dirs only, 2 = one level of children).
pub fn build_disk_map(fda: bool, depth: u8) -> DiskMapResult {
    let home = match commands::home_dir() {
        Some(h) => h,
        None => {
            return DiskMapResult {
                root: DiskNode {
                    name: "~".to_string(),
                    path: String::new(),
                    size: 0,
                    expanded: false,
                    children: vec![],
                    category: "other".to_string(),
                    modified: None,
                },
                disk_total: 0,
                disk_used: 0,
                disk_free: 0,
            };
        }
    };

    // Get disk-level totals.
    let (disk_total, disk_used, disk_free) = get_disk_totals();

    // Build the top-level node list.
    // We enumerate specific directories rather than listing ~ (which would
    // trigger TCC on protected dirs without FDA).
    let mut top_dirs: Vec<(&str, String)> = vec![
        ("Library", format!("{}/Library", home)),
        ("Applications", "/Applications".to_string()),
    ];

    // System-level directories (matching macOS System Settings categories)
    let system_dirs = vec![
        (".Trash", format!("{}/.Trash", home)),
        ("Books", format!("{}/Library/Containers/com.apple.BKAgentService", home)),
        ("Mail", format!("{}/Library/Mail", home)),
        ("Messages", format!("{}/Library/Messages", home)),
        ("Photos", format!("{}/Pictures/Photos Library.photoslibrary", home)),
        ("Mobile Documents", format!("{}/Library/Mobile Documents", home)),
        ("MobileSync", format!("{}/Library/Application Support/MobileSync/Backup", home)),
        ("Podcasts", format!("{}/Library/Group Containers/243LU875E5.groups.com.apple.podcasts", home)),
        ("GarageBand", format!("{}/Library/Audio", home)),
    ];

    for (name, path) in &system_dirs {
        if dir_exists(&path) || std::path::Path::new(&path).exists() {
            top_dirs.push((name, path.clone()));
        }
    }

    // Directories that are safe without FDA
    let safe_dirs = vec![
        ("Projects", format!("{}/Projects", home)),
        ("projects", format!("{}/projects", home)),
        ("src", format!("{}/src", home)),
        ("dev", format!("{}/dev", home)),
        ("code", format!("{}/code", home)),
        ("workspace", format!("{}/workspace", home)),
        ("go", format!("{}/go", home)),
        (".cargo", format!("{}/.cargo", home)),
        (".rustup", format!("{}/.rustup", home)),
        (".npm", format!("{}/.npm", home)),
        (".gradle", format!("{}/.gradle", home)),
        (".docker", format!("{}/.docker", home)),
        (".local", format!("{}/.local", home)),
        (".cache", format!("{}/.cache", home)),
        (".cocoapods", format!("{}/.cocoapods", home)),
        (".android", format!("{}/.android", home)),
    ];

    // Directories that need FDA
    let fda_dirs = vec![
        ("Desktop", format!("{}/Desktop", home)),
        ("Documents", format!("{}/Documents", home)),
        ("Downloads", format!("{}/Downloads", home)),
        ("Movies", format!("{}/Movies", home)),
        ("Music", format!("{}/Music", home)),
        ("Pictures", format!("{}/Pictures", home)),
    ];

    for (name, path) in &safe_dirs {
        if dir_exists(path) {
            top_dirs.push((name, path.clone()));
        }
    }

    if fda {
        for (name, path) in &fda_dirs {
            if dir_exists(path) {
                top_dirs.push((name, path.clone()));
            }
        }
    }

    // Size each top-level directory.
    let mut children: Vec<DiskNode> = Vec::new();
    for (name, path) in &top_dirs {
        let size = du_size(path);
        if size == 0 {
            continue;
        }

        let category = categorize(name, path);
        let mut node = DiskNode {
            name: name.to_string(),
            path: path.clone(),
            size,
            expanded: false,
            children: vec![],
            category,
            modified: None,
        };

        // Expand large directories deeper if requested.
        // depth=2 → one level of children, depth=3 → two levels, etc.
        if depth >= 2 && size > 100 * 1024 * 1024 {
            // > 100MB
            expand_node_recursive(&mut node, &home, fda, depth - 1);
        }

        children.push(node);
    }

    // Sort by size descending.
    children.sort_by(|a, b| b.size.cmp(&a.size));

    let total_mapped: u64 = children.iter().map(|c| c.size).sum();

    let root = DiskNode {
        name: "~".to_string(),
        path: home.clone(),
        size: total_mapped,
        expanded: true,
        children,
        category: "other".to_string(),
        modified: None,
    };

    DiskMapResult {
        root,
        disk_total,
        disk_used,
        disk_free,
    }
}

/// Expand a directory node: size its immediate children.
fn expand_node(node: &mut DiskNode, home: &str, fda: bool) {
    let subdirs = list_subdirs(&node.path);
    let mut child_nodes: Vec<DiskNode> = Vec::new();
    let mut accounted: u64 = 0;

    // Known TCC prefixes — skip without FDA.
    let tcc_prefixes = vec![
        format!("{}/Desktop", home),
        format!("{}/Documents", home),
        format!("{}/Downloads", home),
        format!("{}/Movies", home),
        format!("{}/Music", home),
        format!("{}/Pictures", home),
        format!("{}/Library/Mail", home),
        format!("{}/Library/Messages", home),
        format!("{}/Library/Safari", home),
        format!("{}/Library/Containers", home),
        format!("{}/Library/Mobile Documents", home),
    ];

    for (name, full_path) in &subdirs {
        // Skip TCC-protected children without FDA.
        if !fda
            && tcc_prefixes
                .iter()
                .any(|prefix| full_path.starts_with(prefix.as_str()))
        {
            continue;
        }

        let size = du_size(full_path);
        if size == 0 {
            continue;
        }
        accounted += size;

        let category = categorize(name, full_path);
        child_nodes.push(DiskNode {
            name: name.clone(),
            path: full_path.clone(),
            size,
            expanded: false,
            children: vec![],
            category,
            modified: None,
        });
    }

    // Sort children by size descending.
    child_nodes.sort_by(|a, b| b.size.cmp(&a.size));

    // If there are many small children, roll them up into "Other".
    // Keep the top 15, roll up the rest.
    if child_nodes.len() > 15 {
        let (keep, rest) = child_nodes.split_at(15);
        let other_size: u64 = rest.iter().map(|n| n.size).sum();
        let mut kept = keep.to_vec();
        if other_size > 0 {
            kept.push(DiskNode {
                name: format!("{} other items", rest.len()),
                path: String::new(),
                size: other_size,
                expanded: false,
                children: vec![],
                category: "other".to_string(),
                modified: None,
            });
        }
        child_nodes = kept;
    }

    // If accounted size is less than the node's total, add an "Other" node for
    // files directly in this directory (not in subdirs).
    if accounted < node.size {
        let remainder = node.size - accounted;
        if remainder > 1024 * 1024 {
            // > 1MB
            child_nodes.push(DiskNode {
                name: "Files in this directory".to_string(),
                path: String::new(),
                size: remainder,
                expanded: false,
                children: vec![],
                category: "other".to_string(),
                modified: None,
            });
        }
    }

    node.expanded = !child_nodes.is_empty();
    node.children = child_nodes;
}

/// Recursively expand a node and its large children up to `remaining_depth` levels.
///
/// This gives the sunburst 3-4 rings of real data instead of just 2.
/// Only expands children larger than 50MB to keep subprocess count manageable.
fn expand_node_recursive(node: &mut DiskNode, home: &str, fda: bool, remaining_depth: u8) {
    // First, expand this node's immediate children.
    expand_node(node, home, fda);

    // Then, if we have depth budget left, recurse into children.
    // No size threshold — let natural directory structure determine depth.
    // Flat directories terminate early, nested ones go further.
    if remaining_depth > 1 {
        for child in &mut node.children {
            if !child.path.is_empty() {
                expand_node_recursive(child, home, fda, remaining_depth - 1);
            }
        }
    }
}

/// Get disk totals from `df -k /System/Volumes/Data`.
fn get_disk_totals() -> (u64, u64, u64) {
    let output = std::process::Command::new("df")
        .args(["-k", "/System/Volumes/Data"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = text.lines().collect();
            if lines.len() < 2 {
                return (0, 0, 0);
            }
            let parts: Vec<&str> = lines[1].split_whitespace().collect();
            if parts.len() < 4 {
                return (0, 0, 0);
            }
            let total = parts[1].parse::<u64>().unwrap_or(0) * 1024;
            let used = parts[2].parse::<u64>().unwrap_or(0) * 1024;
            let free = parts[3].parse::<u64>().unwrap_or(0) * 1024;
            (total, used, free)
        }
        _ => (0, 0, 0),
    }
}

/// Expand a specific directory by path (called from frontend for drill-down).
/// Returns the expanded node with sized children.
pub fn expand_directory(path: &str, fda: bool) -> DiskNode {
    let home = commands::home_dir().unwrap_or_default();
    let size = du_size(path);
    let name = std::path::Path::new(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let category = categorize(&name, path);

    let mut node = DiskNode {
        name,
        path: path.to_string(),
        size,
        expanded: false,
        children: vec![],
        category,
        modified: None,
    };

    expand_node(&mut node, &home, fda);
    node
}
