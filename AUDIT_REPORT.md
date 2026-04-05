# Codebase Audit Report

> Generated: 2026-04-05
> Codebase: ~/projects/negativ_ (~37,000 lines across ~60 source files)
> Standards: CODING_STANDARDS.md (17 sections)
> Method: Adversarial review (7 Auditors + 7 Challengers + Lead Auditor adjudication)

---

## Executive Summary

- **Total confirmed violations: 33**
- **CONFIRMED: 14 | ADJUDICATED: 19 | UNCERTAIN: 0** (3 formerly uncertain items resolved via human input)
- **HIGH: 3 | MEDIUM: 17 | LOW: 13**
- **Top 3 problem areas:**
  1. **Section 9 (Component Design)** — 6 Vue components exceed 1000 lines: LargeFiles (1947), SpaceMap (1960), VoronoiViz (1827), Duplicates (1349), GalacticViz (1286), Vault (997). Mixed presentation and business logic.
  2. **Section 2 (Functions)** — 7 Rust functions exceed 100 lines: scan_large_files_stream (185), run_browser_scan (160), scan_candidates (132), scan_vitals (112), compress_directory (112), run_duplicate_scan (102), scan_thermal (102).
  3. **Section 10 (State Management)** — Views directly mutate store state in LargeFiles.vue and Vault.vue, bypassing store actions. Store exports lack readonly() guard.
- **Overall health rating: GOOD**
- **Delta from V1: FAIR (146 findings) -> GOOD (33 findings) — significant improvement**
- **False positive rate: 35 findings removed during challenge phase (44% of auditor findings)**
- **Missed finding rate: 6 new findings added during challenge phase**
- **Regressions introduced by prior refactoring: 1 (keyboard accessibility missing on new LargeFiles directory tree headers)**

---

## Findings by Section

### Section 1 — Naming

No confirmed violations. V1's 10 single-letter variable findings were all resolved. Challengers confirmed renames are in place.

---

### Section 2 — Functions

| # | File | Line(s) | Rule | Severity | Confidence | Fix |
|---|------|---------|------|----------|------------|-----|
| 2.1 | `src-tauri/src/large_files.rs` | 24-208 | S2: function length | MEDIUM | ADJUDICATED | Extract `process_file_entry()` and `build_safe_dirs()` helpers; 185 lines is long even for an orchestrator |
| 2.2 | `src-tauri/src/vault.rs` | 231-362 | S2: function length | MEDIUM | ADJUDICATED | Extract `should_skip_candidate()` filter helper from scan_candidates (132 lines) |
| 2.3 | `src-tauri/src/vault.rs` | 774-885 | S2: function length + DRY | HIGH | CONFIRMED | compress_directory (112 lines) has duplicated state assignments; extract `build_vault_entry_for_directory()` |
| 2.4 | `src-tauri/src/browser.rs` | 653-812 | S2: function length | MEDIUM | CONFIRMED | run_browser_scan (160 lines) — extract `scan_browser_category()` helper. *New: found by Challenger* |
| 2.5 | `src-tauri/src/vitals.rs` | 171-282 | S2: function length | MEDIUM | CONFIRMED | scan_vitals (112 lines) — extract `build_vitals_group()`. *New: found by Challenger* |
| 2.6 | `src-tauri/src/thermal.rs` | 295-396 | S2: function length | LOW | CONFIRMED | scan_thermal (102 lines) — extract `generate_assessment()`. *New: found by Challenger* |
| 2.7 | `src-tauri/src/apps.rs` | 118 | S2: boolean flag | MEDIUM | ADJUDICATED | `remove_leftovers: bool` is a behavior switch — split into `uninstall_app()` + `uninstall_app_with_cleanup()` |
| 2.8 | `src-tauri/src/docker.rs` | 135 | S2: boolean flag | MEDIUM | ADJUDICATED | `prune_all: bool` is a behavior switch — split into `clean_docker()` + `clean_docker_all()` |
| 2.9 | `src/views/LargeFiles.vue` | 97-151 | S2: function length | MEDIUM | CONFIRMED | exportSelected (55 lines) — extract `buildExportPayload()` helper |

**Note:** Auditor flagged `fda: bool` across 4 diskmap.rs functions as boolean flag violations. Challenger correctly identified these as **capability toggles** (system permission state), not behavior switches. Removed as false positives.

**Note:** Auditor flagged run_similar_scan (87 lines) and run_duplicate_scan (102 lines) as violations. Challenger demonstrated both are well-structured orchestrators with already-extracted phase helpers. Removed as false positives.

**Subtotal: 1 HIGH, 7 MEDIUM, 1 LOW**

---

### Section 3 — Structure & Organisation

| # | File | Line(s) | Rule | Severity | Confidence | Fix |
|---|------|---------|------|----------|------------|-----|
| 3.1 | `src-tauri/src/` (4 modules) | 1000+ each | S3: file length | MEDIUM | CONFIRMED | vitals.rs (1124), vault.rs (1067), packages.rs (1039), security.rs (1003) — consider splitting security.rs into launch_audit + app_trust + shell_security. *New: found by Challenger* |

**Note:** Auditor flagged lib.rs (1436 lines) as MEDIUM. Challenger demonstrated it is a Tauri command registration layer — thin wrappers delegating to modules. This is the standard Tauri architecture. Removed as false positive.

**Note:** Auditor flagged types.ts (636 lines) as MEDIUM. Challenger demonstrated splitting would triple import lines across 60+ files for a type-only module with no business logic. Cost exceeds benefit at current project scale. Removed as false positive.

**Subtotal: 0 HIGH, 1 MEDIUM, 0 LOW**

---

### Section 4 — Comments & Readability

| # | File | Line(s) | Rule | Severity | Confidence | Fix |
|---|------|---------|------|----------|------------|-----|
| 4.1 | `src/components/ChipSchematic.vue` | 97-104 | S4: magic numbers | MEDIUM | CONFIRMED | Temperature thresholds (100/90/80/70/55/40) diverge from shared constants in utils.ts (95/80/65/45). Either reconcile or document why different |
| 4.2 | `src/views/SpaceMap.vue` | 61-83 | S4: magic strings + S11: design tokens | MEDIUM | CONFIRMED | 20+ hardcoded HSLA color strings in overviewColors map. Should use cssVar() pattern from utils.ts |
| 4.3 | Multiple files | various | S4: magic numbers | LOW | CONFIRMED | Display limits (.slice(0, N)) used 10+ times without named constants — e.g., CpuCard(24), VoronoiViz(18/20), GalacticViz(15/5), Duplicates(10), Dashboard(5/3). Extract per-component constants |
| 4.4 | `src/components/ChipSchematic.vue` | 120-122 | S4: magic numbers | LOW | ADJUDICATED | ASCII code ranges (48-57, 65-90, 97-122) in charToOrd() — comments exist but named constants would improve clarity |
| 4.5 | `src/components/ChipSchematic.vue` | 140 | S4: magic numbers | LOW | ADJUDICATED | Trimming threshold `0.35` and minimum `2` are undocumented tuning parameters |

**Note:** Auditor flagged 10+ additional magic number instances (regex [0], .split(".")[0], padStart(3), .slice(0,10) for dates, padding 32px). Challenger correctly identified these as idiomatic patterns where extracting constants would hurt readability. Removed as over-pedantic.

**Note:** V1's ~60 "RUST CONCEPT" tutorial comments were fully removed. No instances remain. Confirmed resolved.

**Subtotal: 0 HIGH, 2 MEDIUM, 3 LOW**

---

### Section 5 — Error Handling

| # | File | Line(s) | Rule | Severity | Confidence | Fix |
|---|------|---------|------|----------|------------|-----|
| 5.1 | `src/App.vue`, `src/views/Dashboard.vue` | 146, 47 | S5: meaningful errors | LOW | ADJUDICATED | open_full_disk_access_settings empty catches — no recoverable action exists, but adding console.debug would aid diagnostics |
| 5.2 | `src/stores/domainStatusStore.ts` | 82 | S5: silent swallowing | LOW | CONFIRMED | Cache save failures silently caught — add console.debug for observability |

**Note:** Auditor flagged 8 additional empty catch blocks (revealInFinder, native background sync, monitor polling, localStorage, FDA dev-mode check, FdaWarningBanner). Challenger demonstrated all are intentional best-effort operations in a desktop app context with appropriate comments. High-frequency catches (drag sync at 60fps, monitor polling at 1.5s) would harm performance if logged. Removed as defensible patterns.

**Note:** V1's HIGH findings (vault manifest corruption, cleanup swallowing, unwrap in setup, mutex handling) were all confirmed resolved. Rust backend error handling is now generally good.

**Subtotal: 0 HIGH, 0 MEDIUM, 2 LOW**

---

### Section 6 — DRY & Abstraction

| # | File | Line(s) | Rule | Severity | Confidence | Fix |
|---|------|---------|------|----------|------------|-----|
| 6.1 | `large_files.rs`, `duplicates.rs`, `similar_images.rs` | various | S6: DRY (Rule of Three) | MEDIUM | CONFIRMED | safe_dirs vectors constructed independently in 3 files with near-identical entries. Consolidate shared entries into a base set in commands.rs, with per-domain additions |
| 6.2 | Multiple frontend files | various | S6: DRY | LOW | CONFIRMED | `.split(".").pop()?.toLowerCase()` file extension pattern repeated 8+ times across Dashboard, Duplicates, LargeFiles. Extract `getFileExtension()` to utils.ts. *New: found by Challenger* |

**Note:** V1's 5 HIGH DRY findings (skip prefixes, get_du_size, run_cmd, home_dir, process dictionaries) were all confirmed resolved. Shared utilities consolidated in commands.rs and process_info.rs.

**Subtotal: 0 HIGH, 1 MEDIUM, 1 LOW**

---

### Section 7 — Testing & Maintainability

| # | File | Line(s) | Rule | Severity | Confidence | Fix |
|---|------|---------|------|----------|------------|-----|
| 7.1 | Frontend + Backend | — | S7: testability | MEDIUM | ADJUDICATED | 21 frontend utility tests (utils.test.ts) + ~20 Rust unit tests (security, duplicates, commands, image_utils). Zero Vue component tests, zero Pinia store tests, zero integration tests. Large untested components: SpaceMap (1960 lines), LargeFiles (1947), VoronoiViz (1827) |

**Note:** V1's testing finding was LOW-MEDIUM. Upgraded to MEDIUM because: (a) 37k lines with ~40 tests is minimal, (b) zero store tests means mutation patterns (see S10 findings) have no safety net, (c) Challenger found Rust tests exist (not "zero" as originally reported), but coverage is still narrow.

**Subtotal: 0 HIGH, 1 MEDIUM, 0 LOW**

---

### Section 8 — General Principles

No unique findings beyond those covered in Sections 3, 6, and 7. YAGNI compliance is good (dead code removed, IconTest dev-gated). Boy Scout Rule being followed.

---

### Section 9 — Component Design

| # | File | Line(s) | Rule | Severity | Confidence | Fix |
|---|------|---------|------|----------|------------|-----|
| 9.1 | `src/views/LargeFiles.vue` | 1-1947 | S9: SRP | MEDIUM | ADJUDICATED | 1947 lines mixing file scanning, deletion, AI classification, vault integration, export. Extract composables for grouping/sorting logic and file actions |
| 9.2 | `src/views/SpaceMap.vue` | 1-1960 | S9: SRP | MEDIUM | ADJUDICATED | 1960 lines with 4 visualization modes, cache management, enrichment. Extract sunburst rendering and cache management to composables |
| 9.3 | `src/components/VoronoiViz.vue` | 1-1827 | S9: component size | MEDIUM | ADJUDICATED | 1827 lines — legitimate domain complexity for D3 treemap rendering. Splitting would create tightly-coupled subcomponents. Consider extracting utility functions only |
| 9.4 | `src/views/Duplicates.vue` | 1-1349 | S9: SRP | MEDIUM | ADJUDICATED | 1349 lines with exact duplicates + similar images workflows. Extract DuplicateGroupCard and SimilarImageGroup subcomponents |
| 9.5 | `src/components/GalacticViz.vue` | 1-1286 | S9: component size | MEDIUM | ADJUDICATED | 1286 lines — same domain-complexity justification as VoronoiViz. Extract utility functions only |
| 9.6 | `src/views/Vault.vue` | 1-997 | S9: component size | MEDIUM | ADJUDICATED | 997 lines with clear internal separation. Extract CompressionQueue and VaultEntryCard subcomponents |

**Note:** Auditor rated all 7 oversized components as HIGH. Challenger correctly argued that "lines of code" alone does not constitute HIGH severity ("causes bugs, data loss, security risk, or blocks maintainability"). Visualization components (VoronoiViz, GalacticViz) are inherently complex and well-structured internally. Splitting would create tightly-coupled files with no reuse benefit. All downgraded to MEDIUM.

**Note:** Auditor flagged Security.vue (779 lines) as HIGH. Challenger demonstrated it's a read-only view with clear accordion structure. Removed — below threshold for actionable finding.

**Note:** V1's health card extraction (ThermalCard, CpuCard, FanCard, BatteryCard, MemoryCard) confirmed successful. These components are well-designed, small, and properly accept props.

**Subtotal: 0 HIGH, 6 MEDIUM, 0 LOW**

---

### Section 10 — State Management

| # | File | Line(s) | Rule | Severity | Confidence | Fix |
|---|------|---------|------|----------|------------|-----|
| 10.1 | `src/views/LargeFiles.vue` | 169, 253 | S10: one-direction flow | HIGH | CONFIRMED | View directly mutates store refs: `largeFiles.value = largeFiles.value.filter(...)` and `vaultEntries.value = entries`. Create store actions `removeDeletedFiles()` and `setVaultEntries()` |
| 10.2 | `src/views/Vault.vue` | 185 | S10: one-direction flow | MEDIUM | ADJUDICATED | View mutates store ref: `vaultCandidates.value = vaultCandidates.value.filter(...)`. Create store action `removeCandidates()` |
| 10.3 | All stores | — | S10: minimize global state | LOW | ADJUDICATED | Store refs exported as mutable `ref()` without `readonly()` wrapper. Any view can accidentally mutate. Wrap exports in `readonly()` at store boundary |

**Note:** V1's scanStore monolith (1274 lines) confirmed split into 17 domain stores. Domain status consolidated. fileDiskSize shared. These are confirmed resolved.

**Subtotal: 1 HIGH, 1 MEDIUM, 1 LOW**

---

### Section 11 — Layout & Styling

| # | File | Line(s) | Rule | Severity | Confidence | Fix |
|---|------|---------|------|----------|------------|-----|
| 11.1 | `src/style.css`, `Toast.vue` | various | S11: spacing grid | LOW | ADJUDICATED | ~20 pixel values not on 4/8px grid (9px, 7px, 3px, 5px, 22px). Likely intentional visual refinements but inconsistent with defined spacing tokens |
| 11.2 | `src/views/Memory.vue`, `src/components/MemoryCard.vue` | 85, 27-54 | S11: design tokens | LOW | CONFIRMED | Hardcoded fallback colors (#94a3b8, HSLA values) should use design token constants |

**Note:** Auditor flagged ~60 dynamic `:style` bindings as MEDIUM violations. Challenger demonstrated all are for truly dynamic data-driven values (temperature-mapped colors, computed widths, data-driven backgrounds) — explicitly permitted by the standard. Removed as false positives.

**Note:** V1's inline style and design token findings were largely resolved. ~60 CSS tokens now defined in style.css.

**Subtotal: 0 HIGH, 0 MEDIUM, 2 LOW**

---

### Section 12 — Responsiveness & Accessibility

| # | File | Line(s) | Rule | Severity | Confidence | Fix |
|---|------|---------|------|----------|------------|-----|
| 12.1 | `src/views/LargeFiles.vue` | 801, 833, 908, 939, 969, 998 | S12: keyboard accessible | HIGH | CONFIRMED | 6 collapsible directory/group headers have `@click` but no `tabindex`, `role`, or `@keydown` handlers. **Regression**: these are new file-tree headers not present in V1. Other views (Logs, Security, Packages, Caches) have correct keyboard support |
| 12.2 | `src/views/Cpu.vue` | 203 | S12: keyboard accessible | HIGH | CONFIRMED | hog-header clickable div missing keyboard support. Same regression pattern |
| 12.3 | `src/views/LargeFiles.vue`, `src/views/Cpu.vue`, `src/views/Dashboard.vue` | various | S12: focus indicators | MEDIUM | CONFIRMED | Keyboard-accessible elements (including stat cards with tabindex) lack `:focus-visible` CSS. Add outline styling |

**Note:** Auditor also flagged ThermalCard/FanCard as missing ARIA progressbar roles. Challenger correctly identified these as **gauges/meters**, not progress bars — `role="progressbar"` would be semantically incorrect. Recommend `aria-label` instead. Removed as false positive.

**Note:** V1's 3 HIGH accessibility findings (keyboard, alt text, ARIA) were largely resolved. Alt text added to all images. ARIA attributes added across 7 views. Current finding 12.1 is a **regression** from new code introduced during the refactoring.

**Subtotal: 1 HIGH, 1 MEDIUM, 0 LOW** (12.1 and 12.2 consolidated as one HIGH)

---

### Section 13 — User Interaction & Feedback

No confirmed violations. V1's findings (uninstall confirmation, missing spinners, per-item error handling) all confirmed resolved. Loading states, disabled buttons, and error feedback are properly implemented across views.

---

### Section 14 — Performance

| # | File | Line(s) | Rule | Severity | Confidence | Fix |
|---|------|---------|------|----------|------------|-----|
| 14.1 | `src/views/SpaceMap.vue` | 15 | S14: lazy-load | MEDIUM | CONFIRMED | `import * as d3 from "d3"` imports entire D3 library. Use targeted imports: `import { partition, hierarchy } from "d3-hierarchy"`. *Deferred from V1, still present. New: re-confirmed by Challenger* |
| 14.2 | `src/views/Apps.vue` | 167 | S14: debounce | LOW | ADJUDICATED | Search input filters on every keystroke. List is small (50-300 apps, <1ms filter), so LOW priority. Add debounce for future-proofing |

**Note:** V1's HIGH finding (route lazy-loading) confirmed resolved. All 20 routes use dynamic imports. Dashboard 5s polling noted but deferred to performance optimization phase.

**Subtotal: 0 HIGH, 1 MEDIUM, 1 LOW**

---

### Section 15 — Forms

No confirmed violations. V1's findings (blur validation, label association) confirmed resolved. LargeFiles.vue has blur validation on minSize input.

---

### Section 16 — Navigation & Routing

No confirmed violations. V1's findings (404 route, scroll behavior) confirmed resolved. Catch-all route redirects to dashboard. Scroll position properly restored.

**Note:** Auditor flagged sidebar `<button>` navigation as non-semantic. Challenger correctly identified that buttons are semantically appropriate for interactive elements that do more than navigate (show scan status badges, domain counts). Removed as false positive.

---

### Section 17 — UI General Principles

No unique findings beyond Section 14 (D3 import, debounce). All items covered elsewhere.

---

## Disputed Findings Log

| Dispute | Auditor | Challenger | Lead Ruling | Reasoning |
|---------|---------|------------|-------------|-----------|
| Dynamic inline `:style` bindings (60+) | MEDIUM violation (A6.1) | FALSE POSITIVE — data-driven values | **Removed** | Standard explicitly permits inline styles for "truly dynamic values." Temperature colors, data widths are dynamic. |
| lib.rs 1436 lines (A3.1) | MEDIUM — split command handlers | FALSE POSITIVE — Tauri routing layer | **Removed** | Thin wrappers delegating to modules. Standard Tauri architecture. Splitting would degrade DX. |
| types.ts 636 lines (A3.2) | MEDIUM — split by feature | Cost-negative — triple imports | **Removed** | Type-only module with no logic. Import friction exceeds benefit at project scale. |
| VoronoiViz/GalacticViz 1800+ lines (A5.1/A5.2) | HIGH — monolithic | MEDIUM — domain complexity | **Downgraded to MEDIUM** | D3/Canvas visualization inherently requires significant code. Well-structured internally. Splitting creates coupled subcomponents. |
| `fda: bool` in diskmap.rs (A2.7 partial) | MEDIUM — boolean flag | NOT a violation — capability toggle | **Removed** | System capability flag, not behavior switch. Different from `prune_all`/`remove_leftovers`. |
| run_similar_scan 87 lines (A2.4) | MED-HIGH — too long | NOT a violation — well-orchestrated | **Removed** | 5 extracted phase helpers already exist. Function is a clean orchestrator. |
| ARIA progressbar on ThermalCard (A6.5) | MEDIUM — missing role | FALSE POSITIVE — semantic mismatch | **Removed** | Gauges are not progress bars. `role="progressbar"` would be incorrect. Use `aria-label`. |
| Frontend empty catches (8 items) | MEDIUM/LOW violations | DEFENSIBLE — desktop app patterns | **Mostly removed** | Best-effort operations with intentional comments. High-frequency logging harms perf. Kept 2 as LOW. |
| Apps search debounce (A7.1) | MEDIUM | LOW — list is small | **Downgraded to LOW** | 50-300 items with .includes() filter is <1ms. Debounce adds complexity for negligible gain. |
| Security.vue 779 lines (A5.7) | HIGH — oversized | LOW — read-only accordion | **Removed** | Clear internal structure, read-only from store. Size driven by template, not logic complexity. |

---

## Hotspot Files

Top 10 files ranked by severity-weighted violation count (HIGH=3, MEDIUM=2, LOW=1):

| Rank | File | Lines | Findings | Weighted Score | Primary Issues |
|------|------|------:|----------|---------------:|----------------|
| 1 | `src/views/LargeFiles.vue` | 1,947 | 5 | 12 | Direct store mutation, missing keyboard a11y (regression), monolithic, long function |
| 2 | `src-tauri/src/vault.rs` | 1,067 | 3 | 8 | compress_directory duplicate state (HIGH), scan_candidates length, store mutation in view |
| 3 | `src/views/SpaceMap.vue` | 1,960 | 3 | 6 | D3 full import, hardcoded overview colors, monolithic |
| 4 | `src/components/ChipSchematic.vue` | 611 | 3 | 5 | Temperature threshold divergence, magic numbers |
| 5 | `src-tauri/src/browser.rs` | 867 | 1 | 4 | run_browser_scan 160 lines |
| 6 | `src-tauri/src/large_files.rs` | 208 | 1 | 4 | scan_large_files_stream 185 lines |
| 7 | `src/views/Duplicates.vue` | 1,349 | 1 | 2 | Monolithic (mixed exact + similar workflows) |
| 8 | `src/views/Vault.vue` | 997 | 2 | 4 | Monolithic, direct store mutation |
| 9 | `src-tauri/src/vitals.rs` | 1,124 | 2 | 4 | Module size, scan_vitals 112 lines |
| 10 | `src/views/Cpu.vue` | 214 | 1 | 3 | Missing keyboard accessibility |

---

## Comparison with Prior Audit (V1)

| Metric | V1 (2026-04-04) | V2 (2026-04-05) | Delta |
|--------|-----------------|-----------------|-------|
| Total findings | 146 | 33 | -113 (77% reduction) |
| HIGH severity | 29 | 3 | -26 |
| MEDIUM severity | 65 | 17 | -48 |
| LOW severity | 52 | 13 | -39 |
| Health rating | FAIR -> GOOD | GOOD | Maintained |

**Issues fully resolved since V1: ~120**
- All 60 RUST CONCEPT comments removed
- All 10 single-letter variable findings resolved
- All 5 HIGH DRY violations resolved (shared utilities, scan config, process dictionaries)
- scanStore.ts monolith split into 17 domain stores (1274 -> 368 lines)
- lib.rs extracted into 4+ domain modules (2793 -> 1436 lines)
- 5 health card components extracted and reused
- FDA warning banner extracted to shared component
- Zoom/pan composable extracted from visualizations
- All V1 accessibility findings addressed (alt text, keyboard, ARIA)
- Router: 404 route, scroll behavior, lazy-loading all added
- Vault manifest corruption, cleanup logging, mutex recovery all fixed
- Uninstall confirmation added
- 21 vitest utility tests + ~20 Rust unit tests added

**Issues partially resolved since V1: 3**
- Large Vue components: health cards extracted but main views still 1000+ lines
- Design tokens: ~60 CSS tokens added but some hardcoded colors remain
- Testing: tests added but coverage is still minimal

**Issues unchanged since V1: 4**
- D3 full import (deferred in V1, still present)
- Boolean flag parameters in apps.rs/docker.rs (noted LOW in V1)
- Dashboard polling without visibility gating (deferred to performance phase)
- Frontend organized by type not feature (accepted as Vue convention)

**New issues not present in V1 (including regressions): 7**
- LargeFiles.vue directory tree headers missing keyboard accessibility (**regression** from new code)
- Direct store mutation in LargeFiles.vue and Vault.vue (arose from store split pattern)
- Missing focus-visible CSS on keyboard-accessible elements
- Large Rust modules approaching 1000+ lines (vitals, vault, packages, security)
- run_browser_scan long function (not in V1 scope)
- File extension pattern repeated without utility

**Summary:** The V1 refactoring was highly effective. 77% of findings were resolved. The codebase moved from FAIR to GOOD. Remaining work is primarily structural (large components, long functions) and polish (design tokens, testing coverage). One accessibility regression was introduced in the new LargeFiles directory tree.

---

## Items Flagged for Human Review — RESOLVED

All three items resolved via human input (2026-04-05):

| # | Finding | Decision | Action |
|---|---------|----------|--------|
| ~~U1~~ | ChipSchematic temperature thresholds vs utils.ts | **Reconcile** — not intentionally different | Import shared constants; add intermediate levels if finer granularity needed |
| ~~U2~~ | safe_dirs differences between scan modules | **Consolidate** — `/var/tmp` and `~/node_modules` omissions accidental; similar_images intentionally different (media-only) | Extract shared base list; keep similar_images separate |
| ~~U3~~ | SpaceMap overviewColors vs SPACEMAP_CATEGORY_FILLS | **Consolidate** — confirmed oversight | Merge into design token system |
