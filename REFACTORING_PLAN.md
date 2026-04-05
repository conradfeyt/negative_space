# Refactoring Plan

> Based on: AUDIT_REPORT.md (V2 — adversarial review)
> Standards: CODING_STANDARDS.md
> Generated: 2026-04-05

---

## Prioritisation Strategy

Rank by:
1. **HIGH severity first** — correctness, data integrity, accessibility
2. **CONFIRMED confidence first** — both Auditor and Challenger agree
3. **Impact radius** — how many files/features benefit
4. **Effort** — quick wins before large rewrites
5. **Dependencies** — foundational changes before dependent ones

UNCERTAIN items go into "Pending Human Review" and are not scheduled.

---

## Phase 1 — Critical Fixes (HIGH severity, CONFIRMED)

### 1.1 — Fix keyboard accessibility regression in LargeFiles.vue [S12]
- [ ] Add `tabindex="0"` `role="button"` `:aria-expanded` `@keydown.enter` `@keydown.space.prevent` to 6 clickable div headers:
  - Line 801: vault-header (`toggleGroup('vaulted')`)
  - Line 833: group-header (`toggleGroup(group.id)`)
  - Lines 908, 939, 969, 998: dir-header at depths 0-3 (`toggleDir()`)
- [ ] Add same to Cpu.vue line 203: hog-header (`toggleGroup(group.name)`)
- [ ] Add `:focus-visible` CSS for `.group-header`, `.dir-header`, `.vault-header`, `.hog-header`
- [ ] Use Logs.vue:186 as reference implementation (correct pattern exists there)
- Files: `src/views/LargeFiles.vue`, `src/views/Cpu.vue`
- Complexity: **SMALL**
- Confidence: **CONFIRMED** — both Auditor and Challenger verified missing handlers
- Risk: None — additive change, no behavior modification
- Dependencies: None

### 1.2 — Fix direct store mutations in LargeFiles.vue [S10]
- [ ] Create `removeDeletedFiles(paths: string[])` action in `largeFilesStore.ts`
- [ ] Create `setVaultEntries(entries: VaultEntry[])` action in `vaultStore.ts`
- [ ] Replace `largeFiles.value = largeFiles.value.filter(...)` at line 169 with store action call
- [ ] Replace `vaultEntries.value = entries` at line 253 with store action call
- Files: `src/views/LargeFiles.vue`, `src/stores/largeFilesStore.ts`, `src/stores/vaultStore.ts`
- Complexity: **SMALL**
- Confidence: **CONFIRMED** — both Auditor and Challenger agreed this violates one-direction flow
- Risk: Low — mutation logic stays the same, just moves to store layer
- Dependencies: None

### 1.3 — Fix compress_directory duplicate state in vault.rs [S2]
- [ ] Extract `build_vault_entry_for_directory()` helper (lines 831-855) to consolidate metadata extraction
- [ ] Consolidate duplicate state assignments in success/error branches (lines 863-876)
- Files: `src-tauri/src/vault.rs`
- Complexity: **SMALL**
- Confidence: **CONFIRMED** — Auditor identified, Challenger agreed with reframing
- Risk: Low — internal refactor, no API change
- Dependencies: None

---

## Phase 2 — Structural Improvements (MEDIUM severity)

### 2.1 — Add focus-visible styles to keyboard-accessible elements [S12]
- [ ] Add `:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }` for:
  - Dashboard.vue stat-card elements (lines 380, 395, 424, 436)
  - Duplicates.vue kind-pill filter buttons
  - All expandable section headers with `tabindex="0"` across views
- Files: `src/views/Dashboard.vue`, `src/views/Duplicates.vue`, `src/style.css` (global rule for `[tabindex="0"]:focus-visible`)
- Complexity: **SMALL**
- Dependencies: 1.1 (new keyboard elements need focus styles too)

### 2.2 — Fix direct store mutation in Vault.vue [S10]
- [ ] Create `removeCandidates(paths: string[])` action in `vaultStore.ts`
- [ ] Replace `vaultCandidates.value = vaultCandidates.value.filter(...)` at line 185 with action call
- Files: `src/views/Vault.vue`, `src/stores/vaultStore.ts`
- Complexity: **SMALL**
- Dependencies: None

### 2.3 — Replace D3 wildcard import with targeted imports [S14]
- [ ] Replace `import * as d3 from "d3"` with named imports from D3 submodules
- [ ] Identify all d3.* usages in SpaceMap.vue and map to subpackages:
  - `d3-hierarchy`: partition, hierarchy
  - `d3-scale`: scaleLinear
  - `d3-shape`: arc
  - `d3-selection`: select
  - `d3-zoom`: zoom, zoomIdentity
  - `d3-transition`: transition
  - `d3-ease`: easeCubicInOut
  - `d3-interpolate`: interpolateRgb
- [ ] Update type annotations if needed (d3.HierarchyRectangularNode etc.)
- Files: `src/views/SpaceMap.vue`
- Complexity: **MEDIUM** — requires updating ~15+ type annotations per V1 deferral note
- Dependencies: None
- Risk note: Challenger confirmed this is still present. V1 deferred due to type annotation complexity.

### 2.4 — Reconcile temperature thresholds [S4]
- [ ] Import TEMP_CRITICAL/HOT/WARM/COOL from utils.ts into ChipSchematic.vue
- [ ] Optionally add 2 intermediate levels (TEMP_VERY_HOT=90, TEMP_MILD=55) to utils.ts if finer granularity is needed for die visualization
- [ ] Replace hardcoded thresholds (100/90/80/70/55/40) in `tempHSL()` with shared constants
- Files: `src/components/ChipSchematic.vue`, `src/utils.ts`
- Complexity: **SMALL**
- Confidence: Confirmed as oversight — reconcile to shared constants

### 2.5 — Consolidate SpaceMap overviewColors with SPACEMAP_CATEGORY_FILLS [S4/S11]
- [ ] Merge `overviewColors` (SpaceMap.vue:61-83) into the existing `SPACEMAP_CATEGORY_FILLS` map in utils.ts, or create a shared `OVERVIEW_CATEGORY_COLORS` alongside it
- [ ] Replace hardcoded HSLA strings with `cssVar()` pattern from utils.ts
- [ ] Define corresponding `--overview-*` CSS custom properties in `style.css`
- Files: `src/views/SpaceMap.vue`, `src/utils.ts`, `src/style.css`
- Complexity: **SMALL**
- Confidence: Confirmed as oversight — consolidate into design token system

### 2.6 — Break down long Rust functions [S2]
- [ ] `scan_large_files_stream` (185 lines) in large_files.rs: extract `process_file_entry()` helper
- [ ] `scan_candidates` (132 lines) in vault.rs: extract `should_skip_candidate()` filter helper
- [ ] `run_browser_scan` (160 lines) in browser.rs: extract `scan_browser_category()` helper
- [ ] `scan_vitals` (112 lines) in vitals.rs: extract `build_vitals_group()` helper
- Files: `src-tauri/src/large_files.rs`, `vault.rs`, `browser.rs`, `vitals.rs`
- Complexity: **MEDIUM**
- Dependencies: 1.3 (vault.rs compress_directory first)

### 2.7 — Consolidate safe_dirs across scan modules [S6]
- [ ] Create `pub fn base_scan_safe_dirs(home: &str) -> Vec<String>` in commands.rs with the ~20 shared entries from large_files/duplicates
- [ ] Add `/var/tmp` to the shared list (currently only in large_files — accidental omission from duplicates)
- [ ] Add `~/node_modules` to the shared list (currently only in large_files — likely should also exclude from duplicates)
- [ ] Have large_files.rs and duplicates.rs call the shared function instead of building inline vectors
- [ ] Keep similar_images.rs separate — its 4-entry media-only list (Pictures/Downloads/Documents/Desktop) is intentionally different
- Files: `src-tauri/src/commands.rs`, `large_files.rs`, `duplicates.rs`
- Complexity: **MEDIUM**

### 2.8 — Split boolean flag functions [S2]
- [ ] `uninstall_app(path, remove_leftovers: bool)` in apps.rs -> `uninstall_app()` + `uninstall_app_with_cleanup()`
- [ ] `clean_docker(prune_all: bool)` in docker.rs -> `clean_docker_dangling()` + `clean_docker_all()`
- [ ] Update frontend callers to use the appropriate variant
- Files: `src-tauri/src/apps.rs`, `docker.rs`, `lib.rs`, frontend callers
- Complexity: **MEDIUM**
- Dependencies: None
- Risk note: Challenger flagged that `fda: bool` in diskmap.rs is a capability toggle (NOT a flag violation). Only split the two behavior switches.

### 2.9 — Expand test coverage [S7]
- [ ] Add Pinia store tests for at least: largeFilesStore, vaultStore, domainStatusStore (mutation patterns)
- [ ] Add Vue component test for at least one complex component (e.g., Dashboard computed properties)
- [ ] Target: 50+ tests covering critical paths
- Files: New test files in `src/__tests__/`
- Complexity: **MEDIUM-LARGE**
- Dependencies: 1.2 (store mutations fixed first makes testing cleaner)

### 2.10 — Extract composables from oversized views [S9]
- [ ] LargeFiles.vue: extract `useFileGrouping()` composable (sorting, grouping, categorization logic)
- [ ] SpaceMap.vue: extract `useSunburstViz()` composable (D3 sunburst setup, zoom, arc rendering)
- [ ] Duplicates.vue: extract `useDuplicateFilters()` composable (kind filtering, selection state)
- [ ] Vault.vue: extract `useCompressionQueue()` composable (queue state, size calculation, progress)
- Files: New composables + corresponding view files
- Complexity: **LARGE**
- Dependencies: 2.3 (D3 imports resolved before extracting sunburst composable)

---

## Phase 3 — Polish & Consistency (LOW severity)

### 3.1 — Extract display limit constants [S4]
- [ ] CpuCard.vue: `const MAX_DISPLAYED_CORES = 24`
- [ ] VoronoiViz.vue: `const MAX_TOP_CHILDREN = 18`, `const MAX_CLUSTER_CHILDREN = 20`
- [ ] GalacticViz.vue: `const MAX_PLANETS = 15`, `const MAX_MOONS = 5`
- [ ] Duplicates.vue: `const PREVIEW_FILES_PER_GROUP = 10`
- [ ] Dashboard.vue: `const TOP_FILES_COUNT = 5`, `const TOP_MEMORY_COUNT = 3`
- Files: 5 component/view files
- Complexity: **SMALL**
- Dependencies: None

### 3.2 — Extract getFileExtension utility [S6]
- [ ] Add `export function getFileExtension(name: string): string` to utils.ts
- [ ] Replace 8+ inline `.split(".").pop()?.toLowerCase()` occurrences
- Files: `src/utils.ts`, Dashboard.vue, Duplicates.vue, LargeFiles.vue
- Complexity: **SMALL**
- Dependencies: None

### 3.3 — Wrap store exports in readonly() [S10]
- [ ] Add `readonly()` wrapper to exported refs in all domain stores
- [ ] Expose mutation functions as the only way to modify state
- Files: All files in `src/stores/`
- Complexity: **SMALL-MEDIUM**
- Dependencies: 1.2, 2.2 (store actions created first)

### 3.4 — Add debug logging to remaining silent catches [S5]
- [ ] Add `console.debug()` to FDA settings catches in App.vue and Dashboard.vue
- [ ] Add `console.debug()` to cache save failure in domainStatusStore.ts
- Files: `src/App.vue`, `src/views/Dashboard.vue`, `src/stores/domainStatusStore.ts`
- Complexity: **SMALL**
- Dependencies: None

### 3.5 — Hardcoded fallback colors to design tokens [S11]
- [ ] Memory.vue: replace `"#94a3b8"` fallback with token constant
- [ ] MemoryCard.vue: replace inline HSLA values with token references
- Files: `src/views/Memory.vue`, `src/components/MemoryCard.vue`
- Complexity: **SMALL**
- Dependencies: None

### 3.6 — Add aria-labels to gauge components [S12]
- [ ] ThermalCard.vue: add `aria-label` to thermal strip (e.g., "Temperature: 78 degrees")
- [ ] FanCard.vue: add `aria-label` to fan gauge (e.g., "Fan speed: 2100 RPM")
- Files: `src/components/ThermalCard.vue`, `src/components/FanCard.vue`
- Complexity: **SMALL**
- Dependencies: None

### 3.7 — Snap non-standard pixel values to spacing tokens [S11]
- [ ] Review style.css button/badge padding values (9px, 7px, 6px, 3px)
- [ ] Snap to nearest token or document as intentional visual refinement
- Files: `src/style.css`, `src/components/Toast.vue`
- Complexity: **SMALL**
- Dependencies: None

### 3.8 — Extract scan_thermal assessment helper [S2]
- [ ] Extract `generate_assessment(hottest_temp)` from scan_thermal (102 lines) in thermal.rs
- Files: `src-tauri/src/thermal.rs`
- Complexity: **SMALL**
- Dependencies: None

### 3.9 — Add debounce to Apps search [S14]
- [ ] Add 200ms debounce to `searchQuery` in Apps.vue
- Files: `src/views/Apps.vue`
- Complexity: **SMALL**
- Dependencies: None

---

## Pending Human Review

All three uncertain items have been resolved:

| # | Item | Decision | Action |
|---|------|----------|--------|
| ~~U1~~ | ChipSchematic vs utils.ts temperature thresholds | **Reconcile** — oversight, not intentional | See item 2.4 |
| ~~U2~~ | safe_dirs differences between scan modules | **Consolidate** — `/var/tmp` and `~/node_modules` omissions are accidental. similar_images stays separate (intentionally media-only) | See item 2.7 |
| ~~U3~~ | SpaceMap overviewColors vs SPACEMAP_CATEGORY_FILLS | **Consolidate** — confirmed oversight | See item 2.5 |

---

## Recommended Order of Execution

**Quick wins (do first, ~1-2 hours):**
1. **1.1** — Fix keyboard accessibility regression (7 divs)
2. **1.2** — Fix store mutations in LargeFiles.vue (create 2 store actions)
3. **2.1** — Add focus-visible styles
4. **2.2** — Fix store mutation in Vault.vue
5. **3.4** — Add debug logging to silent catches
6. **3.6** — Add aria-labels to gauges

**Rust backend cleanup (~2 hours):**
7. **1.3** — Fix compress_directory duplicate state
8. **2.6** — Break down 4 long Rust functions
9. **2.8** — Split boolean flag functions (2 instances)
10. **3.8** — Extract scan_thermal helper

**Frontend utilities and tokens (~1-2 hours):**
11. **3.1** — Extract display limit constants
12. **3.2** — Extract getFileExtension utility
13. **3.5** — Hardcoded fallback colors to tokens
14. **3.7** — Snap non-standard pixels
15. **2.5** — SpaceMap overviewColors to tokens (after human decision)

**Structural refactoring (~3-4 hours):**
16. **2.3** — D3 targeted imports
17. **2.10** — Extract composables from oversized views
18. **2.7** — Consolidate safe_dirs (after human decision)

**Testing (~2-3 hours):**
19. **3.3** — Wrap store exports in readonly()
20. **2.9** — Expand test coverage

**After human decisions:**
21. **2.4** — Reconcile temperature thresholds

---

## Rules of Engagement

- Each fix should be a single, reviewable commit
- Run existing tests after every change (`npm run test`)
- Run `npm run tauri build` after structural Rust changes
- Do not change behaviour while refactoring
- If a fix requires behaviour changes, flag it separately
- Where Challenger proposed an alternative fix, prefer the simpler option unless the risk note justifies complexity
- Open the built app from `src-tauri/target/release/bundle/macos/Negativ_.app` to verify after major changes
- Use `./rebuild.sh` which auto-increments build number for verification

---

## Comparison with V1 Refactoring Plan

| Metric | V1 Plan | V2 Plan |
|--------|---------|---------|
| Total items | 35 | 21 (+ 3 pending human review) |
| Phase 1 (Critical) | 14 items | 3 items |
| Phase 2 (Structural) | 10 items | 10 items |
| Phase 3 (Polish) | 12 items | 9 items |
| Estimated effort | ~16-20 hours | ~10-14 hours |

The V2 plan is significantly smaller because the V1 refactoring resolved the majority of issues. Remaining work focuses on:
- **Accessibility regression** from new code (Phase 1)
- **State management discipline** introduced by the store split (Phase 1-2)
- **Continuing size reduction** of large components via composables (Phase 2)
- **Testing foundation** to prevent future regressions (Phase 2)
