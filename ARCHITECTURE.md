# Negativ_ Architecture Decisions

## Overview

Negativ_ is an open-source macOS system cleaner (CleanMyMac alternative) built with
Tauri v2 (Rust backend) + Vue 3 (TypeScript frontend). This document records
architectural and functional decisions with justifications.

---

## Core Architecture

### Decision: Tauri v2 with synchronous-logic async commands
**Choice:** All Tauri commands are declared `async fn` but contain synchronous logic.
**Justification:** Tauri v2 runs non-async commands on the main thread, which blocks
the webview event loop and causes the macOS spinning beach ball. Declaring commands
`async` moves them to a background thread pool. The internal logic remains synchronous
(subprocess calls, walkdir) because the operations are inherently blocking I/O -- 
wrapping them in tokio async would add complexity without benefit.

### Decision: Subprocess-first filesystem access
**Choice:** Use subprocesses (`du`, `ls`, `test`, `find`, `sqlite3`) instead of
in-process Rust filesystem calls (`fs::read_dir`, `walkdir`, `.exists()`) for any
path that might be TCC-protected.
**Justification:** macOS TCC (Transparency, Consent, and Control) triggers BLOCKING
modal permission dialogs when an app's process (or its child processes for `readdir`
operations) accesses protected directories. These dialogs freeze the app thread.
Subprocesses that use `stat`-based operations (`du`, `test -e`) do NOT trigger
dialogs -- they get "Operation not permitted" errors silently. However, subprocesses
that do `readdir` (`ls`) DO trigger dialogs because macOS attributes the access to
the parent app's bundle ID. Therefore:
- `du -sk` = safe (stat-based)
- `test -e`, `test -d`, `test -r` = safe (stat-based) 
- `find ... -delete` = safe (operates on already-known paths)
- `ls <dir>` = UNSAFE for TCC dirs (triggers readdir -> dialog)
- `fs::read_dir` = UNSAFE (in-process readdir -> dialog)

### Decision: Whitelist-based scanning without FDA
**Choice:** Without Full Disk Access, scan only known-safe directories (developer
tools, package managers, /Applications, /tmp) rather than walking from ~ with a
blocklist of TCC paths.
**Justification:** The blocklist approach is a whack-a-mole game. Apple adds new
TCC-protected paths with each macOS version (~Library/Mobile Documents for iCloud,
Photos library, etc.). We discovered this empirically -- even after listing 25+
protected paths, iCloud Drive and Photo Library prompts still appeared. The whitelist
approach guarantees zero TCC dialogs at the cost of reduced scan coverage, which is
the correct tradeoff for an unsigned open-source app.

### Decision: TCC database query for FDA detection
**Choice:** Query `~/Library/Application Support/com.apple.TCC/TCC.db` via `sqlite3`
to check if our app has Full Disk Access, with `test -r` on a Safari file as fallback.
**Justification:** We cannot use `readdir` on TCC-protected paths to probe access
(triggers dialogs). The TCC database is a SQLite file that records all grants/denials
per-app. Querying it with `sqlite3` is a read-only operation that doesn't trigger any
TCC prompts.

### Decision: User-initiated TCC prompts only
**Choice:** Permission dialogs are ONLY triggered when the user explicitly toggles on
a directory in Settings. Never during scans, navigation, or on app launch.
**Justification:** Unexpected permission dialogs are the #1 UX complaint. CleanMyMac
solves this by requiring FDA upfront. We offer a softer approach: work with limited
access by default, let users opt-in to per-directory access, and explain clearly when
FDA would help.

---

## Security Module

### Decision: Persistence/trust audit as primary security feature (not antivirus)
**Choice:** Focus on launch agents/daemons inspection, login items, app trust analysis
(code signing, notarization), and shell init auditing. Defer ClamAV/YARA to later
phases.
**Justification:**
1. **Value per complexity:** Most Mac users don't have malware, but many have forgotten
   launch agents, unsigned startup items, and suspicious shell modifications. The
   persistence audit catches real problems users actually have.
2. **No external dependencies:** ClamAV requires a ~300MB signature database and a
   daemon or CLI tool. YARA requires either a native library or CLI. Both add
   installation friction for an app that's currently 8.4 MB self-contained.
3. **False positive risk:** ClamAV's macOS coverage is limited; most signatures target
   Windows malware. Running it on macOS produces noise. YARA is better but requires
   quality rulesets that need ongoing maintenance.
4. **Honest positioning:** We can truthfully say "security audit" and deliver real
   value. Claiming "malware scanner" with just ClamAV would be misleading.

**Deferred (Phase 2):**
- YARA scanning with bundled macOS-specific rules (lightweight, high-value)
- Browser extension audit (moderate complexity, good value)

**Deferred (Phase 3):**
- ClamAV integration as optional "deep scan" when user has it installed

### Decision: Severity classification system
**Choice:** Four severity levels for security findings:
- `malicious` -- known malware signature match (future, via YARA/ClamAV)
- `likely_unwanted` -- adware, PUPs, known browser hijackers
- `suspicious` -- unsigned startup items, broken signatures, unusual locations
- `informational` -- items worth reviewing but not necessarily problematic
**Justification:** Aligns with GPT-5.4's recommendation and industry practice.
Avoids fear-based reporting while still surfacing real issues.

### Decision: Conservative remediation
**Choice:** Never auto-delete. Offer: reveal in Finder, copy path, disable launch
agent (with confirmation), move to Trash (with explicit approval).
**Justification:** False positives in security tools cause real damage. A user who
deletes a legitimate launch agent can break their system. We show evidence and let
the user decide.

---

## Browser Cleanup

### Decision: Direct file deletion for browser caches
**Choice:** Locate and delete browser cache/cookie/history files directly on disk
rather than using browser APIs or extensions.
**Justification:** Browser APIs require the browser to be running and granting access.
Direct file access is simpler, works when browsers are closed, and aligns with how
CleanMyMac and CCleaner operate. Supported browsers (8 total):
- **Safari**: `~/Library/Safari/`, `~/Library/Caches/com.apple.Safari/`, `~/Library/Cookies/` — all TCC-protected, requires FDA
- **Chrome**: `~/Library/Application Support/Google/Chrome/Default/`, `~/Library/Caches/Google/Chrome/` — not TCC-protected
- **Firefox**: `~/Library/Application Support/Firefox/Profiles/*/` — profile directory auto-discovery
- **Brave, Edge, Arc, Opera, Vivaldi**: Chromium-based, same data layout as Chrome under different paths

### Decision: Data categories with safety classification
**Choice:** Each browser's data is broken into categories (Cache, Cookies, History,
Session Data) with explicit safe/unsafe classification and warning text.
**Justification:** Cache is always safe to clean (the browser regenerates it). Cookies,
history, and sessions are user data that cause real impact if deleted (logged out of
sites, lost history, closed tabs). Requiring explicit user selection per category and
showing a confirmation dialog for unsafe categories prevents accidental data loss.

### Decision: TCC-gated Safari data visibility
**Choice:** Without FDA, Safari categories appear in the scan results but show "--"
for size and are disabled for selection. With FDA, sizes are shown and cleanup works.
**Justification:** Safari data is under TCC-protected paths. `du -sk` returns 0 on
these without FDA. Showing "0 B" would be misleading. Showing "--" with a "Needs FDA"
badge clearly communicates what's happening without triggering TCC dialogs.

### Decision: Cleanup via `rm -rf` subprocess
**Choice:** Delete browser data with `rm -rf` subprocess instead of `fs::remove_dir_all`.
**Justification:** Consistent with our subprocess-first TCC safety pattern. Even for
non-TCC paths, `rm -rf` handles permission errors gracefully without blocking the app.

---

## System Maintenance

### Decision: Task-based UI with individual execution
**Choice:** Present maintenance operations as a list of discrete tasks that the user
runs individually, rather than a "Run All" batch operation.
**Justification:**
1. Several tasks (Spotlight rebuild, font cache clear) have significant side effects
   (hours of re-indexing, requires restart). Users should understand and choose.
2. Admin privilege escalation via osascript triggers macOS's native password dialog.
   Batching would mean repeated password prompts or a single prompt for a long-running
   command that's hard to cancel.
3. Task statuses update independently (running/success/error), giving clear feedback.

### Decision: `osascript` for admin privilege escalation
**Choice:** Use `osascript -e 'do shell script "..." with administrator privileges'`
for tasks requiring root, rather than embedding a helper tool or using `sudo`.
**Justification:**
- `sudo` doesn't work in GUI apps (no terminal for password input)
- macOS's Security framework via `osascript` shows the native authentication dialog
- Users see a familiar macOS prompt, not a custom password field we'd have to build
- Apple's recommended approach for GUI apps needing occasional privilege escalation

### Decision: Six maintenance tasks in Phase 1
**Choice:** Ship with: Flush DNS, Free Purgeable Space, Rebuild Launch Services,
Rebuild Spotlight, Clear Font Caches, Flush Memory.
**Justification:**
- **Flush DNS**: Common troubleshooting task, low risk, immediate effect
- **Free Purgeable**: Unique value — users see "purgeable" in About This Mac but can't
  reclaim it manually. We use `tmutil thinlocalsnapshots` which is safe and effective
- **Rebuild Launch Services**: Fixes duplicate "Open With" entries, common annoyance
- **Rebuild Spotlight**: Fixes broken search, high value when needed
- **Clear Font Caches**: Fixes font rendering issues, low frequency but important
- **Flush Memory**: Marginal value (macOS manages memory well) but users expect it
  from system cleaning tools. Honest description explains this

### Decision: Full transparency on maintenance tasks
**Choice:** Each maintenance task displays exact shell commands, affected services,
affected filesystem paths, destructive/reversible classification, and (for reversible
tasks) a specific explanation of how the effect reverses.
**Justification:** Maintenance tasks touch system services and caches. Users should
be able to make informed decisions. Showing the exact `dscacheutil -flushcache` command
and "mDNSResponder (sent SIGHUP to reload)" is more trustworthy than vague descriptions.
The "How this reverses" section prevents anxiety -- e.g., users learn that DNS cache
rebuilds automatically within seconds as they browse, or that Spotlight re-indexes
in the background over 1-4 hours.

---

## Duplicate Finder

### Decision: Three-stage hashing pipeline (size -> partial hash -> full hash)
**Choice:** Find duplicates by (1) grouping files by byte size, (2) hashing the first
4 KB of same-size files via BLAKE3, (3) full-hashing only files that match on both
size AND partial hash.
**Justification:**
- Stage 1 eliminates ~90% of files with zero I/O (just stat metadata)
- Stage 2 eliminates ~95% of remaining candidates with minimal I/O (4 KB per file)
- Stage 3 confirms true duplicates and operates on the smallest possible set
- BLAKE3 is 3-5x faster than SHA-256 and is designed for streaming (64 KB chunks)
- This approach makes a full-home-directory duplicate scan feasible in 30-60 seconds
  rather than the minutes it would take with naive full-file hashing

### Decision: Skip .git directories and sparse files
**Choice:** Exclude `.git/` directories from duplicate scanning, and skip sparse files
(where actual disk usage < 50% of apparent size).
**Justification:**
- `.git/` contains many small identical blob objects (pack files, index entries) that
  are internal to git and not user-visible duplicates. Including them produces noise.
- Sparse files (like Docker.raw) report large apparent sizes but use little actual
  disk space. Deduplicating them wouldn't save real disk space.

### Decision: "Keep" badge on first file in each group
**Choice:** The first file listed in each duplicate group gets a "Keep" badge. The
"Select duplicates" button selects all files except the first.
**Justification:** Users need guidance on which copy to keep. The first file is an
arbitrary but consistent choice. Users can override by manually selecting/deselecting.
This is how CleanMyMac and Gemini handle it.

---

## Space Visualization

### Decision: Three complementary visualisation modes
**Choice:** Three distinct views of the same disk data — Sunburst (D3 zoomable radial),
Galactic (force-directed star field), and Voronoi (d3-voronoi-treemap).
**Justification:**
- **Sunburst** excels at showing hierarchical proportions with drill-down. D3's partition
  layout and arc transitions make it the clearest "what's using my disk" view.
- **Galactic** uses D3 force simulation to give an intuitive spatial sense of relative
  file sizes as gravitational bodies — more exploratory, less precise.
- **Voronoi** uses `d3-voronoi-treemap` to fill an arbitrary polygon with cells
  proportional to size. The dragonfly-wing aesthetic (white opacity cells, dark veins,
  pastel gradient background showing through) makes it the most visually distinctive
  mode. Consolidated view uses a rectangle; cluster view packs circles per top-level
  directory with nested voronoi subdivision inside each.

### Decision: Two-level lazy expansion with backend drill-down
**Choice:** Initially show top-level directories with one level of children pre-expanded
(for dirs > 100 MB). Further drill-down triggers a backend call (`expand_disk_node`)
that sizes the children of the clicked directory.
**Justification:**
- Running `du -sk` on every subdirectory at all levels would take minutes
- Pre-expanding the first two levels covers the most important data
- Lazy expansion via click means deeper exploration only costs I/O when requested
- The breadcrumb navigation lets users drill down and back up freely

### Decision: Breadcrumb navigation with directory list
**Choice:** Show both a visual treemap and a tabular directory list below it, with
breadcrumb navigation for drill-down.
**Justification:** The treemap gives an intuitive visual overview, but exact sizes and
percentages are hard to read from block proportions alone. The directory list provides
precise numbers. Breadcrumbs let users navigate the hierarchy without losing context.

---

## Frontend Architecture

### Decision: Global reactive store (not Pinia)
**Choice:** Plain Vue 3 `ref()` exports in `scanStore.ts` rather than Pinia or Vuex.
**Justification:** The app has a single concern (scan state) with no complex
getters/actions that benefit from a formal store library. Exported refs are simpler,
have zero dependencies, and work well with Vue's reactivity system. If the app grows
significantly in complexity, migration to Pinia would be straightforward.

### Decision: View-per-feature with shared scan store
**Choice:** Each feature gets its own `.vue` view. All scan results and loading states
live in the global store so navigation doesn't reset scan state.
**Justification:** Users should be able to start a scan, navigate to another view, and
come back without losing results. This was a key requirement from the project brief.

---

## Distribution

### Decision: No Apple code signing
**Choice:** Distribute unsigned via Homebrew Cask and direct DMG download.
**Justification:** Apple Developer Program costs $99/year and requires annual renewal.
For an open-source tool, unsigned distribution via Homebrew Cask is the standard
approach (similar to how many open-source Mac tools are distributed). Users bypass
Gatekeeper with right-click > Open on first launch.
