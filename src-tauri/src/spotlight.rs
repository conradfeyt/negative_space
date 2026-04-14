// spotlight.rs — Core Spotlight integration for Negative _.
//
// Makes archived/vaulted entries searchable from macOS Spotlight by
// creating sidecar files that Spotlight can index.

use std::fs;
use std::path::Path;

use crate::vault::VaultEntry;

/// Directory for Spotlight metadata sidecar files.
fn spotlight_dir() -> Option<String> {
    let home = crate::commands::home_dir()?;
    let dir = format!("{}/Library/Application Support/NegativeSpace/spotlight", home);
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Index a vault entry in Spotlight by creating a searchable sidecar file.
/// The sidecar is a plain text file with metadata that Spotlight indexes.
pub fn index_vault_entry(entry: &VaultEntry) {
    let dir = match spotlight_dir() {
        Some(d) => d,
        None => return,
    };

    let filename = entry.original_path.rsplit('/').next().unwrap_or("unknown");
    let sidecar_path = format!("{}/{}.txt", dir, entry.id);

    let content = format!(
        "Negative _ Compressed Entry\n\
         Original: {}\n\
         Name: {}\n\
         Type: {}\n\
         Size: {} -> {}\n\
         Archived: {}\n",
        entry.original_path,
        filename,
        entry.file_type,
        format_size(entry.original_size),
        format_size(entry.compressed_size),
        entry.archived_at,
    );

    if let Err(e) = fs::write(&sidecar_path, &content) {
        eprintln!("[spotlight] Failed to write sidecar for {}: {}", entry.id, e);
    }

    // Poke Spotlight to index the file
    let _ = std::process::Command::new("mdimport")
        .arg(&sidecar_path)
        .output();
}

/// Remove a vault entry from Spotlight.
pub fn deindex_vault_entry(entry_id: &str) {
    let dir = match spotlight_dir() {
        Some(d) => d,
        None => return,
    };

    let sidecar_path = format!("{}/{}.txt", dir, entry_id);
    if Path::new(&sidecar_path).exists() {
        let _ = fs::remove_file(&sidecar_path);
    }
}

fn format_size(bytes: u64) -> String {
    let gb = bytes as f64 / 1_073_741_824.0;
    let mb = bytes as f64 / 1_048_576.0;
    if gb >= 1.0 {
        format!("{:.1} GB", gb)
    } else {
        format!("{:.0} MB", mb)
    }
}
