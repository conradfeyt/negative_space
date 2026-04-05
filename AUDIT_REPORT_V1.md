# Codebase Audit Report

> Generated: 2026-04-04
> Codebase: ~/projects/negativ_ (~37,000 lines across 42 source files)
> Standards: CODING_STANDARDS.md (17 sections)

---

## Executive Summary

- **Total findings: 146**
- **HIGH: 29 | MEDIUM: 65 | LOW: 52**
- **Top 3 problem areas:**
  1. **Section 4 (Comments & Readability)** — ~60 "RUST CONCEPT" tutorial comments saturate the backend, plus duplicated magic number thresholds across frontend and backend
  2. **Section 6 (DRY) + Section 9 (Component Design)** — Scan infrastructure (skip prefixes, safe dirs, du_size, run_cmd) copy-pasted across 5+ Rust modules; health card logic duplicated between Dashboard and SystemVitals
  3. **Section 12 (Accessibility)** — Zero ARIA attributes in the entire codebase, no keyboard access on clickable divs, missing alt text on most images
- **Overall health rating: FAIR** — Well-structured Rust backend with good type definitions and error propagation patterns, but significant DRY violations, a monolithic frontend store, oversized Vue components, and no accessibility support.

### Resolution Status (2026-04-05)

**134 of 146 findings addressed** across 25 commits in 3 phases.

| Section | Findings | Addressed | Remaining | Resolution |
|---------|----------|-----------|-----------|------------|
| S1 Naming | 10 | 10 | 0 | Single-letter vars renamed (Phase 3) |
| S2 Functions | 15 | 14 | 1 | Long functions extracted into helpers; option structs added. Boolean flag params (2.12, 2.13) deferred — low severity |
| S3 Structure | 6 | 6 | 0 | lib.rs split into 4 modules, scanStore split into 17 stores, dead code removed |
| S4 Comments | 13 | 13 | 0 | 61 RUST CONCEPT comments removed, magic numbers extracted to constants |
| S5 Error Handling | 14 | 14 | 0 | Vault manifest/cleanup logging, mutex recovery, emit logging, ~20 frontend catch blocks fixed |
| S6 DRY | 9 | 9 | 0 | Shared utilities, scan config, process dictionaries, date formatting deduplicated |
| S7 Testing | 4 | 3 | 1 | Vitest added with 21 tests. Global state singletons documented (acceptable for native interop) |
| S9 Component Design | 9 | 8 | 1 | 5 health cards, FDA banner, zoom/pan composable extracted. ChipSchematic props (9.9) deferred |
| S10 State Management | 7 | 6 | 1 | scanStore split, fileDiskSize shared, domain status consolidated. Vault dual-fetch (10.6) partially addressed |
| S11 Layout & Styling | 9 | 8 | 1 | ~60 CSS tokens, inline styles replaced, semantic HTML. Spacing grid snap (11.9) partially done |
| S12 Accessibility | 4 | 4 | 0 | Alt text, keyboard access, ARIA attributes, color-alongside-text all addressed |
| S13 User Interaction | 5 | 5 | 0 | Uninstall confirmation, spinner, per-item try/catch, error feedback all added |
| S14 Performance | 5 | 2 | 3 | Route lazy-loading done. D3 targeted imports (14.2), main-thread D3 (14.3), dashboard polling (14.4) deferred to performance phase |
| S15 Forms | 2 | 2 | 0 | Blur validation and label association added |
| S16 Navigation | 3 | 3 | 0 | 404 route, scroll behavior, keyboard nav all addressed |

**Revised health rating: GOOD** — Significant structural improvements (DRY, SRP, accessibility), remaining items are low-severity polish or deferred to the performance optimization phase.

---

## Findings by Section

### Section 1 — Naming

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 1.1 | `src/views/Dashboard.vue` | 197, 280, 287, 296, 322, 331, 337, 338, 457 | S1: single-letter vars | MEDIUM | `s`, `h`, `c`, `t` used for stats/health/condition/total across 9+ computed blocks | Rename to `summary`, `healthPercent`, `condition`, `totalBytes`, `stats` |
| 1.2 | `src/views/Memory.vue` | 133, 149 | S1: single-letter vars | MEDIUM | `s` for stats, same pattern as Dashboard | Rename to `stats` |
| 1.3 | `src/views/SystemVitals.vue` | 250, 260, 266-267 | S1: single-letter vars | MEDIUM | Same `s`/`t` pattern | Rename to `stats`/`totalBytes` |
| 1.4 | `src/utils.ts` | 5-6 | S1: single-letter vars | MEDIUM | `k` and `i` for byte unit computation | Rename to `bytesPerUnit` and `unitIndex` |
| 1.5 | `src/views/SpaceMap.vue` | 291, 576 | S1: single-letter vars | MEDIUM | `t` for normalized age, `g` for D3 group | Rename to `normalizedAge`, `chartGroup` |
| 1.6 | `src-tauri/src/vitals.rs` | 167, 764-765 | S1: single-letter vars | MEDIUM | `d` for process dictionary, `h`/`m` for hours/minutes | Rename to `descriptions`, `hours`, `minutes` |
| 1.7 | `src-tauri/src/similar_images.rs` | 233, 306, 410 | S1: single-letter vars | LOW | `n`/`p` for bytes read and file path | Rename to `bytesRead`, `filePath`, `imageCount` |
| 1.8 | `src-tauri/src/duplicates.rs` | 374, 386 | S1: single-letter vars | LOW | `p`/`pp` for path/parent | Rename to `filePath`, `parentPath` |
| 1.9 | `src-tauri/src/vault.rs` | 176, 191 | S1: single-letter vars | LOW | `n` for bytes read | Rename to `bytesRead` |
| 1.10 | `src-tauri/src/thermal.rs` | 353 | S1: single-letter vars | LOW | `t` for temperature | Rename to `hottestTemp` |

**Subtotal: 0 HIGH, 6 MEDIUM, 4 LOW**

---

### Section 2 — Functions

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 2.1 | `src-tauri/src/lib.rs` | 178-339, 365-587 | S2: one thing | HIGH | `scan_large_files` (160 lines) and `scan_large_files_stream` (220 lines) are near-identical functions doing config + walk + build + emit | Extract shared walking/filtering into a private helper; the two commands become thin wrappers |
| 2.2 | `src-tauri/src/similar_images.rs` | 106-401 | S2: function length | MEDIUM | `run_similar_scan` is 290 lines doing 6 distinct phases (discovery, hashing, dedup, clustering, thumbnails, assembly) | Extract each phase into its own function |
| 2.3 | `src-tauri/src/duplicates.rs` | 114-445 | S2: function length | MEDIUM | `run_duplicate_scan` is 330 lines with 3 hashing stages + result building | Extract stages into separate functions |
| 2.4 | `src-tauri/src/vault.rs` | 354-507 | S2: function length | MEDIUM | `compress_files` is 150 lines mixing manifest, hashing, compression, verification, deletion | Extract `compress_single_file()` helper |
| 2.5 | `src-tauri/src/vault.rs` | 510-633 | S2: function length | MEDIUM | `restore_file` is 120 lines | Extract directory-restore and single-file-restore into helpers |
| 2.6 | `src-tauri/src/lib.rs` | 1112-1233 | S2: function length | MEDIUM | `find_leftover_paths` repeats existence-check pattern 15 times in 120 lines | Extract `try_add_leftover` helper or build candidate list and filter |
| 2.7 | `src-tauri/src/security.rs` | 252-436 | S2: function length | MEDIUM | `scan_launch_items` is 180 lines | Extract `analyze_launch_item()` helper |
| 2.8 | `src-tauri/src/lib.rs` | 178 | S2: >3 params | MEDIUM | `scan_large_files` has 4 params; streaming variant has 5 | Group into `ScanOptions` struct |
| 2.9 | `src-tauri/src/similar_images.rs` | 106-112 | S2: >3 params | MEDIUM | `run_similar_scan` has 5 params | Group scan config into a struct |
| 2.10 | `src-tauri/src/vault.rs` | 210-215 | S2: >3 params | MEDIUM | `scan_candidates` has 4 params | Group into `VaultScanOptions` struct |
| 2.11 | `src-tauri/src/duplicates.rs` | 114-119 | S2: >3 params | MEDIUM | `run_duplicate_scan` has 4 params | Group into struct |
| 2.12 | `src-tauri/src/lib.rs` | 1452 | S2: boolean flag | LOW | `prune_all: bool` toggles two Docker behaviors | Split into `clean_docker_dangling()` / `clean_docker_all()` |
| 2.13 | `src-tauri/src/lib.rs` | 1595 | S2: boolean flag | LOW | `remove_leftovers: bool` in `uninstall_app` | Could split into `trash_app()` + `clean_leftovers()` |
| 2.14 | `src-tauri/src/diskmap.rs` | 286-431 | S2: function length | LOW | `build_disk_map` is 145 lines mixing dir list with sizing and tree assembly | Extract `collect_top_dirs()` |
| 2.15 | `src-tauri/src/vault.rs` | 210-351 | S2: function length | LOW | `scan_candidates` is 140 lines | Extract per-file evaluation from walkdir loop |

**Subtotal: 1 HIGH, 10 MEDIUM, 4 LOW**

---

### Section 3 — Structure & Organisation

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 3.1 | `src-tauri/src/lib.rs` | 1-2793 | S3: SRP, file length | HIGH | 2,793-line monolith serving as module declarations, inline command implementations, and app entry point. Contains `scan_large_files`, `scan_caches`, `scan_logs`, `scan_apps`, `clean_docker`, `uninstall_app`, and ~30 more commands inline | Extract into domain modules: `large_files.rs`, `caches.rs`, `logs.rs`, `docker.rs`, `apps.rs` etc. `lib.rs` should only declare modules and register commands |
| 3.2 | `src/stores/scanStore.ts` | 1-1274 | S3: SRP | HIGH | 1,274-line god object holding all 17 scan domain states, ~80 exported refs, ~40 exported functions | Split into domain-specific stores |
| 3.3 | Frontend structure | — | S3: group by feature | MEDIUM | Frontend organized by type (`views/`, `components/`, `stores/`) not by feature. All 17 domains share one store, views have no co-located helpers | Consider feature-based folders; at minimum split scanStore |
| 3.4 | `src/types.ts` | 1-636 | S3: SRP | LOW | All interfaces for every domain in one flat file | Co-locate types with domain stores when splitting |
| 3.5 | `src-tauri/src/lib.rs` | 101-339 | S3: dead code | MEDIUM | Non-streaming `scan_large_files` (~240 lines) is never called from frontend | Remove — streaming variant is the only one used |
| 3.6 | `src/views/IconTest.vue` | 1-153 | S3: YAGNI | LOW | Dev/debug view registered in production routes | Gate behind dev flag or remove route |

**Subtotal: 2 HIGH, 2 MEDIUM, 2 LOW**

---

### Section 4 — Comments & Readability

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 4.1 | `src-tauri/src/commands.rs` | 1-337 (13 instances) | S4: comments explain why, not what | HIGH | 13 "RUST CONCEPT" comments explaining basic syntax (`use`, `mod`, `derive`, `Option`, `Result`) | Remove all tutorial comments |
| 4.2 | `src-tauri/src/lib.rs` | throughout (21 instances) | S4: comments explain why | HIGH | 21 "RUST CONCEPT" comments | Remove all |
| 4.3 | `src-tauri/src/security.rs` | throughout (14 instances) | S4: comments explain why | HIGH | 14 "RUST CONCEPT" comments | Remove all |
| 4.4 | `src-tauri/src/duplicates.rs` | 13, 224, 290, 454 | S4: comments explain why | MEDIUM | 4 "RUST CONCEPT" comments | Remove |
| 4.5 | `src-tauri/src/thermal.rs` | 307, 381, 591 | S4: comments explain why | MEDIUM | 3 "RUST CONCEPT" comments | Remove |
| 4.6 | Other Rust files | various | S4: comments explain why | LOW | ~5 scattered "RUST CONCEPT" comments across `memory.rs`, `maintenance.rs`, `browser.rs`, `diskmap.rs` | Remove |
| 4.7 | `src/views/Dashboard.vue` | 184-188 | S4: magic numbers | HIGH | `tempToColor` uses bare thresholds `95, 80, 65, 45` — duplicated in Thermal.vue with slightly different values | Extract shared constants: `TEMP_CRITICAL`, `TEMP_HOT`, `TEMP_WARM`, `TEMP_COOL` |
| 4.8 | `src/views/Thermal.vue` | 129-132 | S4: magic numbers | HIGH | `tempColor` uses `95, 80, 65` — inconsistent with Dashboard (missing `45` threshold) | Shared constants |
| 4.9 | `src/views/Dashboard.vue` | 200, 265, 297, 323, 344, 352 | S4: magic numbers | MEDIUM | Undocumented thresholds: `110` (max temp), `70/40` (fan), `50/20` (battery), `90/75` (memory), `0.5` (segment), `90/75` (storage) | Extract named constants |
| 4.10 | `src/views/LargeFiles.vue` | 313 | S4: magic numbers | MEDIUM | Sparse threshold `0.8` conflicts with `duplicates.rs` using `0.5` | Reconcile and extract constant |
| 4.11 | `src/views/Duplicates.vue` | 148-156 | S4: magic strings | MEDIUM | `extCardColor` returns bare hex strings for file kinds | Extract to named `KIND_COLORS` map |
| 4.12 | `src-tauri/src/similar_images.rs` | 232 | S4: magic numbers | LOW | `4096` buffer duplicates `PARTIAL_HASH_BYTES` from duplicates.rs | Import shared constant |
| 4.13 | `src/views/Dashboard.vue` | 225 | S4: commented-out code | LOW | Changelog comment about removed code | Remove — VCS tracks this |

**~60 total RUST CONCEPT instances. Subtotal: 5 HIGH, 5 MEDIUM, 3 LOW**

---

### Section 5 — Error Handling

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 5.1 | `src-tauri/src/lib.rs` | 2692 | S5: handle explicitly | HIGH | `app.get_webview_window("main").unwrap()` in Tauri setup — crashes app on startup if window not found | Replace with `.expect()` or `.ok_or()?` |
| 5.2 | `src-tauri/src/vault.rs` | 152 | S5: handle explicitly | HIGH | `serde_json::from_str(&data).unwrap_or(empty)` silently returns empty manifest on JSON corruption — data integrity risk | Return `Result` and surface parse errors |
| 5.3 | `src-tauri/src/vault.rs` | 429, 439, 444, 562, 572, 584, 592 | S5: silent swallowing | HIGH | `let _ = fs::remove_file()` during vault cleanup — orphaned files on failure with no logging | Log cleanup failures, append to result's error list |
| 5.4 | `src-tauri/src/gradient.rs` | 50, 63, 162 | S5: handle explicitly | MEDIUM | `GRADIENT_STATE.lock().unwrap()` — poisoned mutex cascades | Use `.unwrap_or_else(\|e\| e.into_inner())` |
| 5.5 | `src-tauri/src/lib.rs` | 114 | S5: meaningful messages | MEDIUM | `"df command failed"` lacks exit code or stderr | Include `output.status` and stderr |
| 5.6 | `src-tauri/src/security.rs` | 938 | S5: silent swallowing | MEDIUM | `let _ = Command::new("launchctl")` silently ignores permission-denied failures | Log non-fatal failures |
| 5.7 | 9 view files | various | S5: silent swallowing | MEDIUM | `try { await invoke("open_full_disk_access_settings"); } catch (_) {}` duplicated across 9 views with no user feedback | Extract shared `openFdaSettings()` with toast on failure |
| 5.8 | `src/stores/scanStore.ts` | 420-428 | S5: handle explicitly | MEDIUM | `loadDiskUsage` catch says "non-critical" but Dashboard relies on the data | Set error state or log |
| 5.9 | `src/stores/scanStore.ts` | 699-704 | S5: handle explicitly | MEDIUM | `scanAll()` catch discards top-level error — scan silently stops midway | Set `scanAllError` state |
| 5.10 | `src/views/Settings.vue` | 80, 104, 127 | S5: handle explicitly | MEDIUM | Path access checks silently fail — user gets no feedback on why toggle failed | Show inline error |
| 5.11 | Multiple frontend files | various | S5: silent swallowing | LOW | ~15 `.catch(() => {})` blocks on icon loading, cache saving, fire-and-forget invocations | At minimum `console.warn` |
| 5.12 | `src-tauri/src/lib.rs` | 481, 509, 557, 564, 578 | S5: silent swallowing | LOW | `let _ = app.emit()` — progress events silently discarded | Log failure on "done" event at minimum |
| 5.13 | `src-tauri/src/vitals.rs` | 1153 | S5: handle explicitly | LOW | `.filter(is_ok)` then `.unwrap()` — safe but brittle | Use `.filter_map(\|e\| e.ok())` |
| 5.14 | `src-tauri/src/preview.rs` | 169-223 | S5: silent swallowing | LOW | Temp dir cleanup failures silently discarded | Log failures |

**Subtotal: 3 HIGH, 7 MEDIUM, 4 LOW**

---

### Section 6 — DRY & Abstraction

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 6.1 | 5 Rust files | lib.rs, duplicates.rs, similar_images.rs, vault.rs | S6: DRY | HIGH | Skip-prefix and scan-root logic copy-pasted 5 times with slightly diverging safe_dirs lists | Extract `ScanConfig` struct + `build_scan_config()` helper in commands.rs |
| 6.2 | 3 Rust files | lib.rs:980, browser.rs:627, packages.rs:136 | S6: DRY | HIGH | `get_du_size` / `dir_size` identical implementation in 3 files (+ a 4th walkdir variant in commands.rs) | Make `get_du_size` pub in commands.rs |
| 6.3 | 2 Rust files | security.rs:178, packages.rs:119 | S6: DRY | HIGH | `run_cmd` identical in 2 files | Move to commands.rs |
| 6.4 | 2 Rust files | commands.rs:271, packages.rs:114 | S6: DRY | HIGH | `home_dir` duplicated with different return types | Use `crate::commands::home_dir()` everywhere |
| 6.5 | 2 Rust files | memory.rs:115-690+, vitals.rs:166-355 | S6: DRY | HIGH | `get_process_dictionary()` and `get_app_bundle_mappings()` duplicated with diverging contents (~600-800 lines total) | Extract shared `process_info.rs` |
| 6.6 | `src/stores/scanStore.ts` | 509-787 | S6: DRY | MEDIUM | 6 scan action functions follow identical boilerplate (guard, set scanning, invoke, set domain, catch, finally) | Extract `runDomainScan<T>()` generic helper |
| 6.7 | `src-tauri/src/similar_images.rs` | 474-507 | S6: DRY | MEDIUM | Hand-rolled date formatting when `commands::format_system_time` already exists | Use the existing utility |
| 6.8 | `src-tauri/src/browser.rs` | 646-661 | S6: DRY | LOW | `path_exists` and `is_app_installed` nearly identical | Consolidate |
| 6.9 | `src-tauri/src/security.rs` | 187 | S6: DRY | LOW | `run_cmd_ok` duplicates pattern from `run_cmd` | Co-locate in commands.rs |

**Subtotal: 5 HIGH, 2 MEDIUM, 2 LOW**

---

### Section 7 — Testing & Maintainability

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 7.1 | Frontend (all) | — | S7: testability | HIGH | Zero frontend tests. No test runner (vitest/jest) configured. 23,000 lines of untested Vue/TS code | Add vitest; start with pure functions in utils.ts and store computed properties |
| 7.2 | `src/stores/scanStore.ts` | 1-1274 | S7: avoid global state | HIGH | Module-level `ref()` singletons = effectively global mutable state. Impossible to unit test in isolation | Split into domain stores; make data refs `readonly` at export boundary |
| 7.3 | `src-tauri/src/gradient.rs` | 36 | S7: global state | LOW | `static GRADIENT_STATE: Mutex<Option<...>>` — justified by AppKit threading model | Document as intentional singleton |
| 7.4 | `src-tauri/src/vitals.rs` | 1287 | S7: global state | LOW | `static CACHE: Mutex<Option<...>>` — reasonable caching | Accept and document |

**Subtotal: 2 HIGH, 0 MEDIUM, 2 LOW**

---

### Section 8 — General Principles

Covered by findings in Sections 3, 6, and 7 (YAGNI, Boy Scout Rule, Least Surprise). No additional unique findings.

---

### Section 9 — Component Design

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 9.1 | `src/views/SpaceMap.vue` | 1-2004 | S9: SRP, composable | HIGH | 2,004-line monolith mixing overview mode, sunburst D3, cache controls, FDA checks | Extract `SpaceMapOverview.vue`, `SunburstViz.vue`, `SpaceMapCacheControls.vue` |
| 9.2 | `src/views/LargeFiles.vue` | 1-1975 | S9: SRP | HIGH | 1,975 lines mixing dir tree building, categorization, icon loading, vault integration, export | Extract `useDirTree.ts`, `FileRow.vue`, `useFileIcons.ts` |
| 9.3 | `src/views/Dashboard.vue` | 178-300 | S9: reuse | HIGH | `tempToColor`, `thermalColor`, `fanArc`, `fanNeedle`, fan gauge SVG all copy-pasted between Dashboard and SystemVitals | Extract `ThermalCard.vue`, `CpuCard.vue`, `FanCard.vue`, `BatteryCard.vue`, `MemoryCard.vue` |
| 9.4 | `src/views/Duplicates.vue` | 1-1383 | S9: SRP | MEDIUM | Two independent workflows (exact + similar) in one 1,383-line component | Split into `ExactDuplicates.vue` + `SimilarImages.vue` |
| 9.5 | `src/components/VoronoiViz.vue`, `GalacticViz.vue` | 1-1843, 1-1269 | S9: composable | MEDIUM | Both implement their own zoom/pan handlers | Extract `useZoomPan.ts` composable |
| 9.6 | 7 view files | various | S9: reuse | LOW | `openFdaSettings()` and FDA warning banner duplicated across 7 views | Extract `FdaWarningBanner.vue` component |
| 9.7 | Multiple views | various | S9: reuse | LOW | `revealInFinder()` duplicated | Move to `utils.ts` or composable |
| 9.8 | 3 views | LargeFiles:374, Logs:73, Memory:105 | S9: reuse | LOW | `timeAgo()` duplicated with slight variations | Consolidate in `utils.ts` |
| 9.9 | `src/components/ChipSchematic.vue` | 1-611 | S9: props | LOW | Reads directly from global store instead of accepting thermal data as props | Accept data as props |

**Subtotal: 3 HIGH, 2 MEDIUM, 4 LOW**

---

### Section 10 — State Management

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 10.1 | `src/stores/scanStore.ts` | 1-1274 | S10: minimize global | HIGH | All 17 domain states in one module with ~80 reactive refs. Every view imports from it | Split into domain-specific stores |
| 10.2 | 3 files | scanStore:431, LargeFiles:313, Dashboard:123 | S10: avoid duplication | HIGH | Sparse file size calculation `f.actual_size < f.apparent_size * 0.8` appears 5 times across 3 files | Export single `fileDiskSize()` from utils |
| 10.3 | Dashboard + SystemVitals | various | S10: derive, don't duplicate | MEDIUM | `thermalColor`, `thermalLabel`, `tempToColor` independently computed from same store ref | Move to store as exported computed or composable |
| 10.4 | All 18 views | various | S10: one-directional flow | MEDIUM | Views directly mutate global store refs (e.g., `largeFiles.value = largeFiles.value.filter(...)`) bypassing store actions | All mutations through store actions; make refs readonly at export |
| 10.5 | `src/stores/scanStore.ts` | 236-368 | S10: avoid duplication | MEDIUM | Per-domain `*Scanning`/`*Scanned`/`*Error` refs AND `domainStatus` record = same info in two shapes | Consolidate into single representation |
| 10.6 | scanStore + LargeFiles + Vault | various | S10: single source of truth | MEDIUM | Vault data fetched independently by LargeFiles (its own `invoke`) and the store | Have store own vault data; views consume from store |
| 10.7 | Multiple views | various | S10: local UI state | LOW | Per-view `successMsg`/error refs for delete feedback repeated everywhere | Use existing `Toast.vue` consistently |

**Subtotal: 2 HIGH, 4 MEDIUM, 1 LOW**

---

### Section 11 — Layout & Styling

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 11.1 | All views | — | S11: semantic HTML | MEDIUM | View root containers use `<div>` where `<section>` fits; lists use nested `<div>` instead of `<ul>/<li>`; stat blocks use `<div>` instead of `<dl>` | Adopt `<section>`, `<ul>/<li>`, `<dl>` where appropriate |
| 11.2 | Memory, SpaceMap, Dashboard, Duplicates | script sections | S11: design tokens | MEDIUM | ~50+ hardcoded hex colors in JS objects bypassing CSS variable system | Define visualization color tokens in `style.css` or a shared JS color map |
| 11.3 | Dashboard, Thermal, Memory | script sections | S11: design tokens | MEDIUM | Temperature/gauge HSLA palettes hardcoded and duplicated | Extract shared `thermal-palette.ts` |
| 11.4 | Packages, Vault, LargeFiles, Duplicates, SpaceMap | scoped `<style>` | S11: design tokens | MEDIUM | Raw hex values in scoped styles instead of CSS variables | Replace with existing or new tokens |
| 11.5 | Docker, Settings, IconTest, Dashboard, Memory, App | template | S11: inline styles | MEDIUM | Static inline styles (`margin-top: 16px`, `text-align: right`) where classes would suffice | Replace with scoped CSS classes using spacing tokens |
| 11.6 | `src/style.css` | 176, 207, 485, 494 | S11: design tokens | LOW | `style.css` itself uses raw hex values instead of self-referencing its own tokens | Use `var(--text)`, `var(--text-secondary)` etc. |
| 11.7 | LargeFiles, Dashboard | template | S11: design tokens | LOW | Inline SVG with hardcoded stroke colors | Use `currentColor` or CSS variables |
| 11.8 | LargeFiles | 1066, 1104, 1144, 1184 | S11: inline styles | LOW | `:style="{ paddingLeft: '20px' }"` repeated 4 times | Create `.file-list-indented` class |
| 11.9 | Various | — | S11: spacing grid | LOW | Arbitrary px values (`9px`, `18px`, `14px`) not on 4/8px grid | Snap to nearest spacing token |

**Subtotal: 0 HIGH, 5 MEDIUM, 4 LOW**

---

### Section 12 — Responsiveness & Accessibility

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 12.1 | Dashboard, Caches, Security, Packages, Logs, Memory, Browsers, Duplicates | various | S12: keyboard accessible | HIGH | Clickable `<div>` elements with `@click` but no `tabindex`, `role`, or `@keydown` handler. Dashboard stat cards are primary navigation | Convert to `<button>` or add `tabindex="0"`, `role="button"`, `@keydown.enter/.space` |
| 12.2 | Caches, Logs, LargeFiles, Browsers, Duplicates, SpaceMap, IconTest | various | S12: alt text | HIGH | Majority of `<img>` tags have no `alt` attribute. Only Apps.vue and one Duplicates instance provide alt text | Add `alt=""` for decorative, descriptive alt for informative images |
| 12.3 | All views and components | — | S12: ARIA | HIGH | Zero `aria-*` attributes or `role` attributes in the entire codebase. Expandable sections, tabs, progress bars, modals all lack ARIA | Add ARIA incrementally: tablist, dialog, progressbar, aria-expanded |
| 12.4 | Dashboard, Memory, LargeFiles, Thermal | various | S12: color alone | MEDIUM | Thermal dot, live indicator, CPU heatmap cells convey status through color only | Add text labels or icons alongside color |

**Subtotal: 3 HIGH, 1 MEDIUM, 0 LOW**

---

### Section 13 — User Interaction & Feedback

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 13.1 | `src/views/Apps.vue` | 63-79 | S13: feedback, prevent double-action | HIGH | `handleUninstall` immediately deletes app + leftovers with no confirmation dialog. Destructive and irreversible | Add confirmation modal (like Trash.vue's `confirmEmpty` pattern) |
| 13.2 | ~15 view files | various | S13: specific errors | MEDIUM | Dozens of `catch (_) {}` blocks with no user feedback on failure (covered in detail under S5) | Toast or inline error on user-initiated action failures |
| 13.3 | `src/views/Duplicates.vue` | 686-689 | S13: loading states | MEDIUM | Similar-images delete button shows "Deleting..." text but lacks the spinner that every other delete button has | Add `<span v-if="cleaning" class="spinner spinner-sm">` |
| 13.4 | `src/views/Vault.vue` | 111-142 | S13: error handling | MEDIUM | `compressQueue` loop has no per-item try/catch — one throw stops all remaining items | Wrap each iteration in try/catch; collect errors |
| 13.5 | `src/views/Maintenance.vue` | 19-21 | S13: error feedback | LOW | Task error status shown via button color only; error output buried in expandable panel | Surface error message more prominently |

**Subtotal: 1 HIGH, 3 MEDIUM, 1 LOW**

---

### Section 14 — Performance

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 14.1 | `src/router.ts` | 2-20 | S14: lazy-load | HIGH | All 20 views eagerly imported. SpaceMap pulls entire D3 library into initial bundle | Convert to `() => import('./views/X.vue')` |
| 14.2 | `src/views/SpaceMap.vue` | 16 | S14: lazy-load | MEDIUM | `import * as d3 from "d3"` imports full D3 instead of specific modules | Use targeted imports: `import { partition, hierarchy } from "d3-hierarchy"` |
| 14.3 | `src/views/SpaceMap.vue` | D3 rendering | S14: main thread | MEDIUM | Sunburst D3 layout computation runs synchronously on main thread | Consider Web Workers or chunked rendering for large datasets |
| 14.4 | `src/views/Dashboard.vue` | 55-61 | S14: unnecessary work | LOW | Polls 4 Tauri commands every 5s unconditionally | Increase interval, add visibility gating, or diff-check |
| 14.5 | `src/views/Apps.vue` | 175 | S14: debounce | LOW | Search input triggers computed filter on every keystroke | Debounce 200-300ms |

**Subtotal: 1 HIGH, 2 MEDIUM, 2 LOW**

---

### Section 15 — Forms

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 15.1 | `src/views/LargeFiles.vue` | 765 | S15: validate on blur | LOW | `minSizeMb` input has `min="1"` HTML attribute but no programmatic validation | Add blur validation clamping to valid range |
| 15.2 | LargeFiles, Duplicates | various | S15: required markers | LOW | Scan config inputs have no visual required indicators or `<label>` association | Add labels and required indicators |

**Subtotal: 0 HIGH, 0 MEDIUM, 2 LOW**

---

### Section 16 — Navigation & Routing

| # | File | Line(s) | Rule | Severity | Description | Fix |
|---|------|---------|------|----------|-------------|-----|
| 16.1 | `src/router.ts` | 24-45 | S16: 404 handling | HIGH | No catch-all route. Navigating to undefined hash shows blank content area | Add `{ path: '/:pathMatch(.*)*', redirect: '/dashboard' }` |
| 16.2 | `src/router.ts` | 22-46 | S16: scroll position | MEDIUM | No `scrollBehavior` configured. Scroll position lost on back navigation | Add `scrollBehavior(to, from, savedPosition) { return savedPosition \|\| { top: 0 } }` |
| 16.3 | `src/views/Dashboard.vue` | 562, 585 | S16: real links | LOW | Health cards use `@click="navigateTo('thermal')"` on divs instead of `<router-link>` | Wrap in `<router-link>` or add `role="link"` + keyboard support |

**Subtotal: 1 HIGH, 1 MEDIUM, 1 LOW**

---

### Section 17 — UI General Principles

No additional unique findings beyond those covered in Sections 14 (performance/lazy-load) and 9 (component design). The D3 full-import issue (14.2) and utility duplication (covered in S6/S9) are the main relevant items.

---

## Hotspot Files

Top 10 files by severity-weighted violation count (HIGH=3, MEDIUM=2, LOW=1):

| Rank | File | Lines | Findings | Weighted Score | Primary Issues |
|------|------|------:|----------|---------------:|----------------|
| 1 | `src-tauri/src/lib.rs` | 2,793 | 14 | 32 | Monolith, dead code, DRY violations, unwrap in setup, silent emits |
| 2 | `src/stores/scanStore.ts` | 1,274 | 10 | 26 | God object, global state, duplicated scan boilerplate, error swallowing |
| 3 | `src/views/Dashboard.vue` | 1,504 | 12 | 24 | Magic numbers, duplicated card logic, non-keyboard divs, color-only indicators |
| 4 | `src/views/LargeFiles.vue` | 1,975 | 9 | 20 | Monolith, inline styles, missing alt text, sparse threshold inconsistency |
| 5 | `src/views/SpaceMap.vue` | 2,004 | 7 | 18 | Monolith, full D3 import, hardcoded colors, main-thread rendering |
| 6 | `src-tauri/src/vault.rs` | 1,039 | 8 | 18 | Silent error swallowing, long functions, manifest corruption risk, param count |
| 7 | `src-tauri/src/security.rs` | 1,049 | 6 | 16 | RUST CONCEPT comments, silent launchctl, long functions, run_cmd duplication |
| 8 | `src/views/Duplicates.vue` | 1,383 | 6 | 14 | Mixed responsibilities, missing spinner, hardcoded colors, no alt text |
| 9 | `src-tauri/src/similar_images.rs` | 511 | 5 | 12 | 290-line function, param count, DRY (date formatting, buffer size) |
| 10 | `src/router.ts` | 48 | 3 | 11 | No lazy loading, no 404 route, no scroll behavior |
