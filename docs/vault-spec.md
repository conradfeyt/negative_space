# Negativ_ Vault — Compress Instead of Delete

## Overview

The Vault is a managed compression archive that lets users reclaim disk space without permanently losing files. Files are compressed into a managed location with full metadata preservation, and can be restored to their original path at any time.

This addresses the core anxiety of disk cleanup: "I need space but I'm afraid to delete things I might need."

---

## User Flow

1. **Discover** — User navigates to the Vault view (new sidebar item under Cleanup)
2. **Scan** — Negativ_ scans for compression candidates: large, stale, compressible files
3. **Review** — User sees a ranked list with estimated savings per file and total
4. **Compress** — User selects files and compresses them into the vault
5. **Browse** — Vault view shows all archived files with original path, date, savings
6. **Restore** — One-click restore puts the file back where it was

---

## Compression Candidates

### Good candidates (prioritize these)
- Files not accessed in 30+ days (configurable threshold)
- Size > 10MB (below this, savings aren't worth the overhead)
- High-compressibility formats:
  - Code/project directories (node_modules excluded — better to just delete)
  - Log files, CSVs, TSVs, JSON, XML, SQL dumps
  - Database files (.db, .sqlite)
  - Documents (.pages, .docx — internal XML)
  - Virtual machine snapshots, disk images (non-compressed)
  - Old Xcode archives, derived data
  - Build artifacts, .o files, .class files

### Skip (already compressed or poor candidates)
- Images: .jpg, .jpeg, .png, .gif, .webp, .heic, .raw, .tiff
- Video: .mp4, .mov, .avi, .mkv, .wmv, .webm
- Audio: .mp3, .aac, .flac, .ogg, .wav (wav compresses but is often large)
- Archives: .zip, .gz, .bz2, .xz, .zst, .7z, .rar, .tar.gz, .dmg
- App bundles: .app (breaks code signatures)
- System files: anything under /System, /Library/Apple
- Currently open files (check with `lsof` or skip recently-modified <1min)

### Estimation
Before compressing, sample the first 64KB of each file and run a quick zstd compression to estimate the ratio. Show this in the UI as "~X% smaller" or "saves ~Y MB". Files with <20% estimated savings should be deprioritized (shown but greyed/dimmed).

---

## Vault Storage

### Location
```
~/Library/Application Support/Negativ_/vault/
  manifest.json          # metadata for all archived files
  data/                  # compressed files, named by hash
    <blake3_hash>.zst    # compressed file data
```

### Manifest Schema (JSON)
```json
{
  "version": 1,
  "entries": [
    {
      "id": "uuid-v4",
      "original_path": "/Users/alice/projects/old-thing/data.csv",
      "original_size": 524288000,
      "compressed_size": 87234560,
      "compression_ratio": 0.166,
      "blake3_hash": "a1b2c3d4...",
      "vault_filename": "a1b2c3d4...zst",
      "archived_at": "2026-03-25T20:00:00Z",
      "original_modified": "2025-11-14T09:30:00Z",
      "original_accessed": "2025-12-01T14:00:00Z",
      "permissions": 33188,
      "owner_uid": 501,
      "group_gid": 20,
      "file_type": "csv",
      "tags": []
    }
  ]
}
```

Using JSON for v1 simplicity. Can migrate to SQLite later if the manifest grows large (>10K entries).

### Naming Convention
Compressed files are named by their BLAKE3 hash of the original content: `<hash>.zst`. This:
- Avoids filename collisions
- Makes deduplication trivial (same content = same hash = already archived)
- Decouples storage from the original filesystem structure

---

## Compression Format

**zstd (Zstandard)** at compression level 3 (default).
- Fast compression: ~400 MB/s
- Fast decompression: ~1.5 GB/s
- Good ratios on text/data files: typically 3-10x
- Rust crate: `zstd` (well-maintained, wraps the C library)
- Single-file compression (not tar) — each file is independently restorable

### Why not tar.zst?
We need individual file restore without decompressing everything. File-by-file zstd means restoring one file is O(1), not O(n).

### Why not APFS transparent compression?
APFS compression (lzfse/lzvn) works at the filesystem level but:
- Not user-controllable
- No way to track what's compressed
- Can't "restore" — it's transparent
- Ratios are lower than zstd for most content

---

## Operations

### Compress (Archive)

```
Input: list of file paths to archive
For each file:
  1. Verify file exists and is readable
  2. Compute BLAKE3 hash of original
  3. Check if hash already in vault (dedup)
  4. Compress with zstd level 3 → write to vault/data/<hash>.zst
  5. Verify: decompress in memory, compare hash (integrity check)
  6. Record metadata in manifest
  7. Delete original file
  8. Report: original size, compressed size, savings
```

**Error handling:** If any step fails after compression but before deletion, the compressed copy stays in the vault and the original is untouched. Never delete the original unless the compressed copy is verified.

### Restore

```
Input: vault entry ID or original path
  1. Look up entry in manifest
  2. Read compressed file from vault
  3. Decompress to original path
     - Recreate parent directories if needed
     - Restore original permissions (chmod)
  4. Verify BLAKE3 hash matches
  5. Remove entry from manifest
  6. Delete compressed file from vault
```

**Conflict handling:** If a file already exists at the original path, prompt the user: overwrite, restore to a different location, or cancel.

### Scan for Candidates

```
Input: scan root (default ~), min_size_mb, min_age_days
  1. Walk filesystem (same FDA-aware logic as large file scanner)
  2. Filter: size >= min_size, last_accessed >= min_age_days ago
  3. Filter: not in skip-list extensions
  4. Filter: not already in vault
  5. For each candidate: estimate compression ratio (sample 64KB)
  6. Sort by estimated savings descending
  7. Return list with: path, size, estimated_compressed_size, last_accessed, file_type
```

---

## UI: Vault View

### Layout

```
+--------------------------------------------------+
| Vault                                    [Scan]  |
| Compress files to reclaim space.                 |
| Restore anytime.                                 |
+--------------------------------------------------+
| VAULT SUMMARY                                    |
| [3 files archived] [1.2 GB saved] [340 MB used] |
+--------------------------------------------------+
|                                                  |
| ARCHIVED FILES  (sort: savings / date / name)    |
| +----------------------------------------------+|
| | data.csv                    saves 437 MB     ||
| | /Users/alice/projects/old/data.csv           ||
| | Archived 2 weeks ago  |  524 MB → 87 MB     ||
| |                          [Restore]           ||
| +----------------------------------------------+|
| | ...                                          ||
+--------------------------------------------------+
|                                                  |
| COMPRESSION CANDIDATES  (after scan)             |
| +----------------------------------------------+|
| | [ ] big-export.json         ~saves 890 MB    ||
| |     /Users/alice/Downloads  |  1.2 GB        ||
| |     Not accessed in 94 days                  ||
| +----------------------------------------------+|
| | [ ] old-project/            ~saves 340 MB    ||
| |     ...                                      ||
| +----------------------------------------------+|
| [Compress Selected]                              |
+--------------------------------------------------+
```

### Candidate warnings
Files accessed within the last 7 days or located inside app bundles show an inline warning:
> "This file was recently accessed. Check that no apps depend on it after compression. You can restore it instantly if something breaks."

### Vault health
If the vault itself is using significant space (>5GB), show a note:
> "Your vault is using X GB. Consider permanently deleting archived files you no longer need."

With a "Review vault" action that shows the oldest/largest archived files.

---

## Rust Implementation Plan

### New module: `src-tauri/src/vault.rs`

**Data structures:**
- `VaultManifest` — the full manifest with version + entries vec
- `VaultEntry` — one archived file's metadata
- `CompressionCandidate` — scan result for a potential file
- `VaultSummary` — total files, total savings, vault size
- `CompressResult` — result of a compress operation
- `RestoreResult` — result of a restore operation

**Tauri commands:**
- `scan_vault_candidates(path, min_size_mb, min_age_days, fda)` → `Vec<CompressionCandidate>`
- `compress_files(paths)` → `CompressResult`
- `restore_file(entry_id)` → `RestoreResult`
- `get_vault_summary()` → `VaultSummary`
- `get_vault_entries()` → `Vec<VaultEntry>`
- `delete_vault_entry(entry_id)` → permanently remove from vault
- `estimate_compression(path)` → quick ratio estimate for a single file

**Dependencies to add:**
- `zstd = "0.13"` (Rust zstd bindings)
- `uuid = { version = "1", features = ["v4"] }` (entry IDs)

### Frontend

**New files:**
- `src/views/Vault.vue` — main vault view
- Route: `/vault`, sidebar under Cleanup section

**Store additions (scanStore.ts):**
- Vault state refs (summary, entries, candidates, scanning, errors)
- Functions: scanVaultCandidates, compressFiles, restoreFile, loadVault, deleteVaultEntry

---

## Edge Cases

**Duplicate content:** If two files have the same BLAKE3 hash, only one compressed copy is stored. Both manifest entries point to the same vault file. On restore of either, the compressed file remains until all entries referencing it are restored.

**Disk full during compression:** If the disk fills up while writing the compressed file, abort and clean up the partial .zst file. Original is untouched.

**Original path has special characters:** Manifest stores the raw path string. Rust's std::fs handles Unicode paths natively.

**Symlinks:** Skip symlinks during scanning. Only compress regular files.

**Large files (>4GB):** zstd handles arbitrary file sizes with streaming compression. No special handling needed.

**Vault corruption:** On startup, optionally verify manifest entries match actual .zst files in the vault. Flag orphaned files or missing entries.

---

## Future Enhancements (not v1)

- **Directory compression:** Archive entire directories as tar.zst with a single manifest entry. Useful for old project folders.
- **Scheduled compression:** Auto-archive files matching certain criteria on a weekly basis.
- **Vault encryption:** Encrypt compressed files at rest with a user-provided passphrase.
- **Cloud offload:** Move vault to an external drive or cloud storage for even more savings.
- **Spotlight integration:** Keep archived files searchable via Spotlight metadata.
