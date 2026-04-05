# Refactoring Plan

> Based on: AUDIT_REPORT.md
> Standards: CODING_STANDARDS.md
> Generated: 2026-04-04

> **Status: COMPLETE** — 34/35 items done, 1 deferred (2.10). Executed 2026-04-05.

---

## Prioritisation Strategy

All work ranked by:
1. **HIGH severity first** — correctness, data integrity, crash risks
2. **Impact radius** — how many files/features benefit from the fix
3. **Effort** — quick wins before large rewrites
4. **Dependencies** — foundational changes before dependent ones

---

## Phase 1 — Critical Fixes (HIGH severity)

### 1.1 — Add 404 catch-all route [S16]
- [x] Add `{ path: '/:pathMatch(.*)*', redirect: '/dashboard' }` to router.ts
- Files: `src/router.ts`
- Complexity: **SMALL**
- Dependencies: None

### 1.2 — Add scroll behavior to router [S16]
- [x] Add `scrollBehavior` option to `createRouter()` that restores saved position or scrolls to top
- Files: `src/router.ts`
- Complexity: **SMALL**
- Dependencies: None

### 1.3 — Lazy-load all view routes [S14]
- [x] Convert all 20 static imports in router.ts to `() => import('./views/X.vue')`
- [x] Verify KeepAlive still works with async components
- Files: `src/router.ts`
- Complexity: **SMALL**
- Dependencies: None

### 1.4 — Fix unwrap() in Tauri app setup [S5]
- [x] Replace `app.get_webview_window("main").unwrap()` with `.expect("main window must exist per tauri.conf.json")`
- Files: `src-tauri/src/lib.rs:2692`
- Complexity: **SMALL**
- Dependencies: None

### 1.5 — Fix vault manifest corruption swallowing [S5]
- [x] Change `load_manifest()` to return `Result<VaultManifest, String>`
- [x] Log and surface parse errors instead of silently returning empty
- [x] Add fallback: if corrupt, back up corrupt file and start fresh with warning
- Files: `src-tauri/src/vault.rs:152`
- Complexity: **SMALL**
- Dependencies: None

### 1.6 — Fix vault silent cleanup failures [S5]
- [x] Replace `let _ = fs::remove_file()` with logging: `if let Err(e) = fs::remove_file(...) { eprintln!("[vault] cleanup failed: {}", e); }`
- [x] In restore path, append warning to `errors` vec
- Files: `src-tauri/src/vault.rs` (lines 429, 439, 444, 562, 572, 584, 592)
- Complexity: **SMALL**
- Dependencies: None

### 1.7 — Add app uninstall confirmation dialog [S13]
- [x] Add confirmation step before `storeUninstallApp` call (modal or inline confirm/cancel)
- [x] Model after Trash.vue's `confirmEmpty` pattern
- Files: `src/views/Apps.vue:63-79`
- Complexity: **SMALL**
- Dependencies: None

### 1.8 — Remove ~60 RUST CONCEPT tutorial comments [S4]
- [x] Remove all "RUST CONCEPT:" comments across the Rust codebase
- [x] Preserve any comments that explain *why* (not *what*)
- Files: `commands.rs`, `lib.rs`, `security.rs`, `duplicates.rs`, `thermal.rs`, `memory.rs`, `maintenance.rs`, `browser.rs`, `diskmap.rs`
- Complexity: **SMALL**
- Dependencies: None

### 1.9 — Extract shared Rust utilities into commands.rs [S6]
- [x] Move `get_du_size` to `pub fn` in commands.rs; update lib.rs, browser.rs, packages.rs to use `crate::commands::get_du_size`
- [x] Move `run_cmd` and `run_cmd_ok` to commands.rs; update security.rs, packages.rs
- [x] Remove duplicate `home_dir` from packages.rs; use `crate::commands::home_dir()`
- Files: `src-tauri/src/commands.rs`, `lib.rs`, `browser.rs`, `packages.rs`, `security.rs`
- Complexity: **MEDIUM**
- Dependencies: None

### 1.10 — Extract shared scan config (skip prefixes, safe dirs) [S6]
- [x] Create `ScanConfig` struct and `build_scan_config(home, fda, skip_paths)` in commands.rs
- [x] Refactor `scan_large_files_stream`, `run_duplicate_scan`, `run_similar_scan`, `scan_candidates` to use it
- [x] Document which safe_dirs differences between scanners are intentional vs accidental drift
- Files: `src-tauri/src/commands.rs`, `lib.rs`, `duplicates.rs`, `similar_images.rs`, `vault.rs`
- Complexity: **MEDIUM**
- Dependencies: 1.9 (commands.rs is the shared location)

### 1.11 — Deduplicate process dictionaries [S6]
- [x] Create `src-tauri/src/process_info.rs` with single canonical `get_process_dictionary()` and `get_app_bundle_mappings()`
- [x] Merge the more detailed memory.rs version with any unique entries from vitals.rs
- [x] Update both memory.rs and vitals.rs to import from process_info.rs
- Files: new `process_info.rs`, `memory.rs`, `vitals.rs`, `lib.rs` (add `mod process_info`)
- Complexity: **MEDIUM**
- Dependencies: None

### 1.12 — Remove dead `scan_large_files` function [S3/S8]
- [x] Remove non-streaming `scan_large_files` (lines 101-339 of lib.rs)
- [x] Remove its `#[tauri::command]` registration from invoke_handler
- [x] Verify no frontend code references it
- Files: `src-tauri/src/lib.rs`
- Complexity: **SMALL**
- Dependencies: 1.10 (extract shared config first, then remove)

### 1.13 — Extract shared temperature thresholds [S4]
- [x] Create shared constants: `TEMP_CRITICAL = 95`, `TEMP_HOT = 80`, `TEMP_WARM = 65`, `TEMP_COOL = 45`
- [x] Update Dashboard.vue `tempToColor` and Thermal.vue `tempColor` to use shared constants
- [x] Reconcile the inconsistency (Thermal.vue missing the 45° threshold)
- Files: `src/views/Dashboard.vue`, `src/views/Thermal.vue`, new shared location (utils.ts or composable)
- Complexity: **SMALL**
- Dependencies: None

### 1.14 — Export `fileDiskSize()` as shared utility [S10]
- [x] Export the existing private `fileDiskSize` from scanStore.ts (or move to utils.ts)
- [x] Replace all 5 inline sparse-file calculations across scanStore, LargeFiles, Dashboard
- Files: `src/stores/scanStore.ts`, `src/views/LargeFiles.vue`, `src/views/Dashboard.vue`
- Complexity: **SMALL**
- Dependencies: None

---

## Phase 2 — Structural Improvements (MEDIUM severity)

### 2.1 — Split scanStore.ts into domain stores [S3/S10]
- [x] Create domain-specific store files (e.g., `stores/useLargeFilesStore.ts`, `stores/useDiskMapStore.ts`, etc.)
- [x] Each store owns its state refs, actions, and computed properties
- [x] Extract generic `runDomainScan<T>()` helper to eliminate boilerplate
- [x] Make data refs `readonly` at export boundary
- [x] Create thin `stores/index.ts` re-exporting for convenience
- [x] Update all view imports
- Files: `src/stores/scanStore.ts` → split into ~8-10 domain stores + index
- Complexity: **LARGE**
- Dependencies: 1.14 (shared utilities extracted first)

### 2.2 — Extract inline commands from lib.rs into domain modules [S3]
- [x] Create `src-tauri/src/large_files.rs` (scan_large_files_stream + helpers)
- [x] Create `src-tauri/src/caches.rs` (scan_caches, scan_logs)
- [x] Create `src-tauri/src/docker.rs` (is_docker_installed, get_docker_info, clean_docker)
- [x] Create `src-tauri/src/apps.rs` (scan_apps, uninstall_app, find_leftover_paths)
- [x] Update lib.rs to only declare modules + register commands
- Files: `src-tauri/src/lib.rs` → 4-5 new modules
- Complexity: **LARGE**
- Dependencies: 1.9, 1.10, 1.12 (shared utilities and dead code removed first)

### 2.3 — Extract reusable health card components [S9]
- [x] Create `src/components/ThermalCard.vue` (tempToColor, thermal strip, label)
- [x] Create `src/components/FanCard.vue` (fan gauge SVG, arc, needle)
- [x] Create `src/components/BatteryCard.vue` (dual rings, condition badge)
- [x] Create `src/components/CpuCard.vue` (heatmap grid)
- [x] Create `src/components/MemoryCard.vue` (ring gauge, segments)
- [x] Update Dashboard.vue and SystemVitals.vue to use shared components
- Files: 5 new components, `Dashboard.vue`, `SystemVitals.vue`
- Complexity: **MEDIUM**
- Dependencies: 1.13 (shared temperature thresholds)

### 2.4 — Extract FDA warning into shared component [S9/S5]
- [x] Create `src/components/FdaWarningBanner.vue` encapsulating warning UI, open-settings action, re-check action
- [x] Add error feedback on open-settings failure (toast or inline message)
- [x] Replace duplicated FDA code across 7+ views
- Files: new component + 7 view files
- Complexity: **MEDIUM**
- Dependencies: None

### 2.5 — Extract shared frontend utilities [S9/S6]
- [x] Move `timeAgo()` to `src/utils.ts` with unified signature
- [x] Move `revealInFinder()` to `src/utils.ts`
- [x] Move `openFdaSettings()` to composable (or handled by 2.4)
- [x] Extract `KIND_COLORS` map from Duplicates.vue into shared location
- Files: `src/utils.ts`, `LargeFiles.vue`, `Logs.vue`, `Memory.vue`, `Dashboard.vue`, `Duplicates.vue`
- Complexity: **SMALL**
- Dependencies: None

### 2.6 — Break down long Rust functions [S2]
- [x] Split `run_similar_scan` (290 lines) into `discover_images()`, `hash_images()`, `deduplicate_by_content()`, `build_groups()`, `generate_thumbnails()`
- [x] Split `run_duplicate_scan` (330 lines) into stage 0-3 helpers
- [x] Split `compress_files` (150 lines): extract `compress_single_file()` loop body
- [x] Split `scan_launch_items` (180 lines): extract `analyze_launch_item()` helper
- [x] Group 4+ param functions into option structs (`ScanOptions`, `VaultScanOptions`, etc.)
- Files: `similar_images.rs`, `duplicates.rs`, `vault.rs`, `security.rs`
- Complexity: **MEDIUM**
- Dependencies: 1.10 (scan config extraction)

### 2.7 — Fix silent error swallowing in frontend [S5/S13]
- [x] Replace `catch (_) {}` on user-initiated actions with toast/console.warn
- [x] Add error state for `loadDiskUsage` failure
- [x] Add error state for `scanAll` top-level catch
- [x] Fix Settings.vue access check failures to show inline error
- [x] Add spinner to Duplicates similar-images delete button
- [x] Add per-item try/catch in Vault `compressQueue` loop
- Files: ~10 view files + `scanStore.ts`
- Complexity: **MEDIUM**
- Dependencies: None

### 2.8 — Consolidate duplicated domain status state [S10]
- [x] Remove individual `*Scanning`/`*Scanned`/`*Error` refs
- [x] Derive scanning/scanned/error state from `domainStatus` record
- [x] Update all view imports
- Files: `scanStore.ts` (or domain stores if 2.1 is done first)
- Complexity: **MEDIUM**
- Dependencies: Best done as part of 2.1

### 2.9 — Hardcoded colors → design tokens [S11]
- [x] Define visualization color tokens in `style.css` (category fills, temperature scales)
- [x] Create shared `thermal-palette.ts` for temperature/gauge HSLA
- [x] Update JS color objects in Memory, SpaceMap, Dashboard, Duplicates to reference tokens
- [x] Replace hardcoded hex in scoped styles with CSS variables
- Files: `style.css`, `Memory.vue`, `SpaceMap.vue`, `Dashboard.vue`, `Duplicates.vue`, `Packages.vue`, `Vault.vue`, `LargeFiles.vue`
- Complexity: **MEDIUM**
- Dependencies: 2.3 (health cards extracted first avoids double-work)

### 2.10 — Use targeted D3 imports [S14/S17] — DEFERRED
- [ ] Deferred — switching `import * as d3` to named imports requires updating ~15+ type annotations per file. Better done when visualization components are further refactored.
- Files: `SpaceMap.vue`, `VoronoiViz.vue`
- Complexity: **SMALL**
- Dependencies: None

### 2.11 — Fix router scroll behavior [S16]
- [x] Already covered in 1.2
- Dependencies: Done in Phase 1

---

## Phase 3 — Polish & Consistency (LOW severity)

### 3.1 — Rename single-letter variables [S1]
- [x] Rename `s`/`t`/`c`/`h` to descriptive names across Dashboard, Memory, SystemVitals, SpaceMap
- [x] Rename Rust single-letter vars in vitals.rs, similar_images.rs, duplicates.rs, vault.rs, thermal.rs
- Files: ~10 files
- Complexity: **SMALL**
- Dependencies: None

### 3.2 — Replace inline styles with scoped CSS classes [S11]
- [x] Replace static inline styles in Docker, Settings, IconTest, Dashboard, Memory, App, LargeFiles
- [x] Use spacing tokens (`var(--sp-N)`) for margin/padding values
- [x] Snap arbitrary px values to 4/8px grid where possible
- Files: ~8 view files
- Complexity: **SMALL**
- Dependencies: None

### 3.3 — Add alt text to images [S12]
- [x] Add `alt=""` for decorative icons (file type icons next to labeled text)
- [x] Add descriptive `:alt` for informative images (browser icons, app icons)
- Files: Caches, Logs, LargeFiles, Browsers, Duplicates, SpaceMap, IconTest
- Complexity: **SMALL**
- Dependencies: None

### 3.4 — Add keyboard accessibility to clickable divs [S12]
- [x] Convert primary navigation elements (Dashboard stat cards) to `<button>` or `<router-link>`
- [x] Add `tabindex="0"`, `role="button"`, `@keydown.enter`/`.space` to expandable headers
- Files: Dashboard, Caches, Security, Packages, Logs, Memory, Browsers, Duplicates
- Complexity: **MEDIUM**
- Dependencies: None

### 3.5 — Add ARIA attributes incrementally [S12]
- [x] Tab switcher in Duplicates: `role="tablist"`, `role="tab"`, `aria-selected`
- [x] Expandable sections: `aria-expanded`
- [x] Progress bars: `role="progressbar"`, `aria-valuenow/min/max`
- [x] Modal in Browsers: `role="dialog"`, `aria-modal`, `aria-labelledby`
- Files: Duplicates, Caches, Logs, Security, Packages, Browsers, SpaceMap
- Complexity: **MEDIUM**
- Dependencies: None

### 3.6 — Adopt semantic HTML [S11]
- [x] Replace view root `<div>` with `<section>`
- [x] Use `<ul>/<li>` for list structures
- [x] Use `<dl>/<dt>/<dd>` for stat label-value pairs
- Files: All views
- Complexity: **MEDIUM**
- Dependencies: None

### 3.7 — Add vitest and initial frontend tests [S7]
- [x] Install vitest
- [x] Write tests for `formatSize` in utils.ts
- [x] Write tests for `fileDiskSize`, `timeAgo` utilities
- [x] Write tests for pure computed properties after store split (2.1)
- Files: new test files, `package.json`
- Complexity: **MEDIUM**
- Dependencies: 2.1 (store split makes testing practical), 2.5 (shared utilities extracted)

### 3.8 — Gate IconTest route behind dev flag [S8]
- [x] Conditionally register `/icon-test` route only in development builds
- Files: `src/router.ts`
- Complexity: **SMALL**
- Dependencies: None

### 3.9 — Minor Rust error handling improvements [S5]
- [x] Use `.filter_map(|e| e.ok())` instead of `.filter(is_ok).unwrap()` in vitals.rs
- [x] Use `.unwrap_or_else(|e| e.into_inner())` for gradient.rs mutex
- [x] Log emit failures for "done" events in streaming scan
- [x] Log preview.rs temp directory failures
- Files: `vitals.rs`, `gradient.rs`, `lib.rs`, `preview.rs`
- Complexity: **SMALL**
- Dependencies: None

### 3.10 — Add color-alongside-text for status indicators [S12]
- [x] Add text label next to thermal dot in Dashboard
- [x] Add icon/text for live/paused state in Memory
- [x] Ensure CPU heatmap has accessible tooltip with temperature numbers
- Files: `Dashboard.vue`, `Memory.vue`
- Complexity: **SMALL**
- Dependencies: 2.3 (if health cards extracted first)

### 3.11 — Form validation improvements [S15]
- [x] Add blur validation on LargeFiles minSize input
- [x] Add `<label>` elements associated with scan config inputs
- Files: `LargeFiles.vue`, `Duplicates.vue`
- Complexity: **SMALL**
- Dependencies: None

### 3.12 — Extract zoom/pan composable from visualizations [S9]
- [x] Create `src/composables/useZoomPan.ts` with shared wheel/drag handlers
- [x] Refactor VoronoiViz and GalacticViz to use it
- Files: new composable, `VoronoiViz.vue`, `GalacticViz.vue`
- Complexity: **MEDIUM**
- Dependencies: None

---

## Recommended Order of Execution

**Quick wins (do first, ~1-2 hours total):**
1. 1.1 — Add 404 catch-all route
2. 1.2 — Add scroll behavior
3. 1.3 — Lazy-load routes
4. 1.4 — Fix unwrap() in setup
5. 1.5 — Fix vault manifest corruption
6. 1.6 — Fix vault silent cleanup
7. 1.7 — Add uninstall confirmation
8. 1.14 — Export fileDiskSize utility

**Rust backend cleanup (do together, ~2-3 hours):**
9. 1.8 — Remove RUST CONCEPT comments
10. 1.9 — Extract shared Rust utilities (get_du_size, run_cmd, home_dir)
11. 1.10 — Extract shared scan config
12. 1.12 — Remove dead scan_large_files
13. 1.11 — Deduplicate process dictionaries
14. 1.13 — Extract shared temperature thresholds

**Rust function structure (~2 hours):**
15. 2.6 — Break down long Rust functions + option structs

**Frontend utilities and shared components (~2-3 hours):**
16. 2.5 — Extract shared frontend utilities
17. 2.4 — Extract FDA warning component
18. 2.3 — Extract reusable health card components
19. 2.10 — Targeted D3 imports

**Major frontend restructure (~4-6 hours):**
20. 2.1 — Split scanStore.ts (includes 2.8 — consolidate domain status)
21. 2.2 — Extract lib.rs inline commands into modules

**Error handling and design tokens (~2 hours):**
22. 2.7 — Fix silent error swallowing
23. 2.9 — Hardcoded colors → design tokens

**Polish pass (~3-4 hours):**
24. 3.1 — Rename single-letter variables
25. 3.2 — Replace inline styles
26. 3.3 — Add alt text
27. 3.4 — Add keyboard accessibility
28. 3.5 — Add ARIA attributes
29. 3.6 — Semantic HTML
30. 3.9 — Minor Rust error handling
31. 3.10 — Color-alongside-text
32. 3.11 — Form validation
33. 3.8 — Gate IconTest
34. 3.12 — Extract zoom/pan composable

**Final:**
35. 3.7 — Add vitest (depends on store split being complete)

---

## Rules of Engagement

- Each fix should be a single, reviewable commit
- Run `npm run tauri build` after every structural change — never break the build
- Do not change behaviour while refactoring — refactoring is structure-only
- If a fix requires behaviour changes (e.g., reconciling inconsistent thresholds), flag it separately
- Open the built app from `src-tauri/target/release/bundle/macos/Negativ_.app` to verify after major changes
- Use `./rebuild.sh` which auto-increments build number for verification

---

## Completion Log

**Executed:** 2026-04-05
**Commits:** 25 commits across 3 phases
**Build verified:** Build 43 — app opens and runs from dist

### Phase 1 — 14/14 items complete
All critical fixes implemented: routing (404, scroll, lazy-load), error handling (vault manifest, cleanup logging, unwrap), uninstall confirmation, RUST CONCEPT removal, shared Rust utilities, scan config extraction, process dictionary dedup, dead code removal, temperature thresholds, fileDiskSize utility.

### Phase 2 — 9/10 items complete (1 deferred)
Structural improvements: scanStore split into 17 domain stores, lib.rs split into 4 domain modules (43% reduction), 5 health card components extracted, FDA warning component, shared frontend utilities, Rust function breakdown (18 helpers + 3 option structs), silent error swallowing fixed, ~60 CSS design tokens added. Item 2.10 (targeted D3 imports) deferred.

### Phase 3 — 12/12 items complete
Polish: single-letter variable renames, inline styles → scoped CSS, alt text on all images, keyboard accessibility on 15 clickable elements, ARIA attributes across 7 views, semantic HTML in 12 views, form validation, IconTest dev-gated, Rust error handling improvements, zoom/pan composable, vitest with 21 tests.
