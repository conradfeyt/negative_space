# Component Cohesion Report
> Generated: 2026-04-06
> Standards: CODING_STANDARDS.md Section 9

## Executive Summary
- Total UI patterns identified: **28**
- Patterns with consistent implementation: **8**
- Patterns with divergent implementations: **20**
- Total consolidation opportunities: **16**
- Estimated shared components to extract: **12**

The app has strong foundations — global `.card`, `.card-flush`, `.btn-*`, `.badge-*`, `.empty-state`, and `.loading-state` classes provide good consistency where used. The main problems are: (1) patterns that _should_ use these globals but don't (one-off cards, custom buttons), (2) patterns that have no global class at all (stat cards, section headers, tab bars, toggles, segmented controls), and (3) threshold/color-mapping logic duplicated with inconsistent values across views.

## Resolution Status
> Updated: 2026-04-10

All critical and high-value items resolved. Summary:

| Item | Status | Component |
|---|---|---|
| StatusDot | Resolved | Global `.status-dot` CSS class |
| SectionHeader | Resolved | Global `.section-title` CSS class |
| StatCard | Resolved | `StatCard.vue` component |
| SegmentedControl | Resolved | Merged into `TabBar.vue` |
| EmptyState | Resolved | `EmptyState.vue` with icon slot |
| HealthThresholds | Resolved | Centralized in `utils.ts` |
| LiveIndicator | Resolved | `LiveIndicator.vue` component |
| TabBar | Resolved | `TabBar.vue` (unified) |
| ToggleSwitch | Resolved | `ToggleSwitch.vue` component |
| SourceBadge | Resolved | Global CSS `.source` modifier |
| CloseButton | Resolved | Global `.btn-close` CSS class |
| RevealButton | Resolved | Global `.btn-reveal` CSS class |
| Spinner fix | Resolved | Removed conflicting double class |
| Card opacity | Resolved | Unified to `var(--glass)` 0.45 |
| Badge system | Resolved | Border-based with modifiers |
| Button system | Resolved | Standardized across app |
| Checkbox | Resolved | `Checkbox.vue` component |
| FileRow | Resolved | `FileRow.vue` component |
| ScanBar | Resolved | `ScanBar.vue` component |
| Modal | Resolved | `Modal.vue` component |

### Remaining tech debt
- DirTreeNode: LargeFiles directory tree has 4 nested depth levels. FileRow extraction covered the file rows, but the directory node nesting is still manual.
- Toast redesign: Current Toast.vue needs bottom-right fixed positioning and app-wide adoption.
- FDA warning → sidebar: Design decision deferred to future work.

---

## Pattern Inventory

### 1. Cards & Containers

#### 1.1 Health Stat Cards (Dashboard + SystemVitals)

| Property | Dashboard `.stat-card` | SystemVitals `.stat-card` |
|---|---|---|
| Background | `rgba(255,255,255,0.45)` + `blur(20px)` | `rgba(255,255,255,0.45)` + `blur(20px)` |
| Border | `0.5px solid rgba(255,255,255,0.55)` | `0.5px solid rgba(255,255,255,0.55)` |
| Border-radius | `var(--radius-sm)` = 12px | `var(--radius-sm)` = 12px |
| Box-shadow | inset highlight + layered | inset highlight + layered |
| Padding | `12px 16px` | `12px 16px` |
| Grid | 3-column, 10px gap | 3-column, 10px gap |

**Verdict:** Identical — copy-pasted CSS blocks. Should be a single shared class.

#### 1.2 Small Metric/Stat Cards

Five different implementations of the same visual role:

| Property | Apps/Packages `.stat-card.card-flush` | Duplicates `.stat` | Security `.summary-card` | Vault `.summary-stat` |
|---|---|---|---|---|
| Background | `var(--glass)` (via card-flush) | `var(--glass)` | `var(--surface)` | `var(--surface)` |
| Border-radius | 16px | 16px | 16px | 16px |
| Box-shadow | `var(--shadow-sm)` | `var(--shadow-sm)` | `var(--shadow-sm)` | `var(--shadow-sm)` |
| Padding | `12px 16px` | `14px 16px` | `16px` | `14px 12px` |
| Value font-size | 18px | 18px | **24px** | 18px |
| Value font-weight | 700 | 700 | 700 | 700 |
| Label font-size | 11px | 11px | (unset) | 10px |
| Container gap | 8px | 16px | 12px | 8px |

**Recommendation:** Extract `<StatCard>` with props for `value`, `label`, and optional `highlight` variant.

**Design decisions needed:**
- Value font-size: 18px (majority) or 24px (Security)?
- Background: `var(--glass)` or `var(--surface)`?
- Container gap: 8px, 12px, or 16px?

#### 1.3 Category/Group Containers

| Property | Caches `.cache-category` | Logs `.log-category` | Duplicates `.group-card` |
|---|---|---|---|
| Background | `rgba(255,255,255,0.4)` | `rgba(255,255,255,0.4)` | `rgba(255,255,255,0.45)` |
| Border | `1px solid rgba(0,0,0,0.05)` | `1px solid rgba(0,0,0,0.05)` | `1px solid rgba(0,0,0,0.06)` |
| Border-radius | `var(--radius-md)` = 16px | `var(--radius-md)` = 16px | `var(--radius-lg)` = 20px |
| Padding | 0 (overflow: hidden) | 0 (overflow: hidden) | 0 (overflow: hidden) |

**Verdict:** Caches and Logs are identical. Duplicates is trivially different (5% more opacity, 20px vs 16px radius). All three should use a shared `.group-container` class.

**Design decision needed:** Border-radius 16px or 20px?

#### 1.4 CPU Process Cards (Custom one-offs)

| Property | CPU `.hog-card` | CPU `.idle-list` |
|---|---|---|
| Background | `rgba(255,255,255,0.32)` | `rgba(255,255,255,0.28)` |
| Border | `0.5px solid rgba(255,255,255,0.45)` | `0.5px solid rgba(255,255,255,0.4)` |
| Border-radius | 12px | 12px |
| Box-shadow | `0 1px 2px rgba(0,0,0,0.03)` | `0 1px 2px rgba(0,0,0,0.03)` |

**Issue:** Neither uses global `.card` or `.card-flush`. These use lower-opacity backgrounds (0.28–0.32 vs standard 0.72) for a subtler look. Could potentially use `.card` with opacity overrides, or a new `.card-subtle` variant.

#### 1.5 Dashboard One-off Cards

| Card | Class | Uses global `.card`? |
|---|---|---|
| AI summary | `.ai-summary-card` | No — custom `rgba(255,255,255,0.35)`, 12px radius |
| Info strip | `.info-strip` | No — custom `rgba(255,255,255,0.3)`, 12px radius |
| Side cards (files, reclaim) | `.card.side-card` | Yes — with scoped padding override |

**Issue:** AI summary and info strip are one-off styles that could use a `.card-subtle` variant.

#### 1.6 Thermal Cards

| Property | `.summary-card` | `.fan-card` | `.assessment-bar` |
|---|---|---|---|
| Background | `rgba(255,255,255,0.12)` + `blur(16px)` | `rgba(255,255,255,0.12)` + `blur(16px)` | `var(--glass)` |
| Border | `1px solid rgba(255,255,255,0.25)` | `1px solid rgba(255,255,255,0.25)` | `1px solid var(--glass-border)` |
| Border-radius | 12px | 12px | 12px |

**Issue:** Thermal `.summary-card` uses `rgba(255,255,255,0.12)` while Security `.summary-card` uses `var(--surface)` = `rgba(255,255,255,0.75)`. Same class name, completely different visual weight across views.

#### 1.7 List Item Cards (card-flush users)

All consistent via global `.card-flush`:

| View | Class modifier | Custom padding |
|---|---|---|
| Apps | `.app-item` | via internal `.app-row` |
| Browsers | `.browser-card` | via internal rows |
| Security | `.finding-item` | via internal rows |
| Packages | `.manager-card` / `.runtime-card` | via internal rows |
| Maintenance | `.task-card` | `sp-5` top, `sp-3` bottom |
| Memory | `.group-card` | via internal rows |
| Vault | `.entry-card` / `.candidate-card` / queue items | `sp-3` to `sp-5` |

**Verdict:** Best-standardized group. All correctly use the global `.card-flush` base.

#### 1.8 Content Cards (global `.card` users)

Consistent via global `.card` (glass bg, 16px radius, 24px padding):

Docker (3 cards), Trash (2 cards), Memory (stats-card), Settings (3 sections), Dashboard (scan-section, side-cards), all empty states.

**Verdict:** Fully consistent.

#### 1.9 Empty State Variants

| Pattern | Views | Styling |
|---|---|---|
| `.card.empty-state` | Caches, Logs, Apps, LargeFiles, Browsers, Duplicates, Memory, Packages, Security, Docker, Trash | Global: centered, 48px padding, muted 15px |
| `.empty-state-card` | Vault (2 instances) | Custom: SVG icon + title (16px/600) + description (13px) |
| `.side-empty` | Dashboard | Custom: `.text-muted` only |

**Recommendation:** Vault's richer empty state (icon + title + description) is better UX. Promote it as the standard and retrofit simpler instances.

---

### 2. Buttons & Actions

#### 2.1 Primary Scan Buttons

All use `btn-primary scan-btn`. Consistent base, but spinner markup diverges:

| Spinner pattern | Views |
|---|---|
| `class="spinner-sm"` (correct) | Caches, Logs, LargeFiles, Dashboard |
| `class="spinner spinner-sm"` (double class) | Apps, Browsers, Duplicates, Packages, Security, Vault, SpaceMap, Maintenance |
| No spinner | Dashboard reclaim card |

**Issue:** The double-class `spinner spinner-sm` applies conflicting border colors (dark track from `.spinner` + white track from `.spinner-sm`). 8 views have this bug.

**Recommendation:** Fix all to use just `class="spinner-sm"` inside buttons.

#### 2.2 Danger/Destructive Buttons

All use `btn-danger`. Same spinner inconsistency as above. Additionally:

| View | Loading feedback |
|---|---|
| Caches, Logs, Browsers, Duplicates, LargeFiles, Apps, Trash (confirm) | Spinner + text change |
| Trash (initial), Docker, Security | Text change only or no feedback |

**Recommendation:** Standardize: all danger actions should show spinner during operation.

#### 2.3 Toggle/Filter Button Groups (7 independent implementations)

| View | Class | Active style | Shape |
|---|---|---|---|
| LargeFiles | `.sort-btn` / `.sort-btn--active` | accent fill, white text | rounded container |
| Packages | `.filter-btn.active` | accent tint, accent-deep text | individual bordered pills |
| Duplicates (tabs) | `.tab-btn.active` | white bg + shadow | underline-style tabs |
| Vault (tabs) | `.tab-btn.active` | white bg + shadow | underline-style tabs |
| Duplicates (kind) | `.kind-pill.active` | accent glow | full pills (20px radius) |
| SpaceMap (viz) | `.viz-btn.active` | white bg, font-weight 600 | icon buttons |
| SpaceMap (color/expand) | `.color-toggle-btn` / `.expand-toggle-btn` | accent bg | custom |

**Recommendation:** Extract two shared components:
- `<SegmentedControl>` — for mutually exclusive options (sort, filter, viz mode)
- `<TabBar>` — for page-section tabs (Duplicates, Vault)

**Design decisions needed:**
- Tab active indicator: underline (Duplicates/Vault style) or filled background (SpaceMap style)?
- Segmented control shape: pill (Duplicates kind) or rectangular (LargeFiles sort)?

#### 2.4 Close/Dismiss Buttons (4 implementations)

| View | Class | Icon size |
|---|---|---|
| Vault error banner | `.dismiss-btn` | 14x14 SVG |
| Vault queue item | `.btn-remove` | 14x14 SVG |
| Toast | `.toast-close` | 14x14 SVG |
| Duplicates preview | `.btn-preview-close` | 14x14 SVG |

**Verdict:** All are 14x14 X icons with slightly different container styling. Should be a single `.btn-close` class.

#### 2.5 Pause/Resume Buttons

| View | Implementation |
|---|---|
| Thermal, Cpu, SystemVitals | `btn-ghost btn-sm` with text "Pause"/"Resume" |
| Memory | Custom `.btn-pause` — 32x32 icon-only with play/pause SVG |

**Design decision needed:** Text-based or icon-only? Memory's icon-only approach is more compact but the others are more explicit.

#### 2.6 FDA Banner Buttons (separate button system)

`FdaWarningBanner.vue` defines its own `.btn-fda-primary` / `.btn-fda-secondary` with `border-radius: 10px` and `font-size: 12px`. These do NOT use the global `.btn-primary` / `.btn-secondary` classes.

**Recommendation:** Replace with global `btn-primary btn-sm` and `btn-secondary btn-sm`.

#### 2.7 Reveal-in-Finder Buttons

| View | Class | Icon |
|---|---|---|
| LargeFiles | `.reveal-btn` | Native folder icon + SVG fallback |
| SpaceMap | `.dir-reveal-btn` | SVG open-external |
| Dashboard | No button — click on entire row | N/A |

**Recommendation:** Extract a shared `<RevealButton>` or standardize the icon approach.

---

### 3. Form Controls & Inputs

#### 3.1 Text/Number Inputs

| Instance | File | Type | Width | Padding | Border | Focus shadow | Label | Placeholder |
|---|---|---|---|---|---|---|---|---|
| Min size | LargeFiles | number | 60px | `8px 16px` (global) | `var(--border)` | 3px accent-light | sr-only | none |
| Scan path | LargeFiles | text | 120px | `8px 16px` (global) | `var(--border)` | 3px accent-light | sr-only | none |
| App search | Apps | text | flex:1 | `8px 14px` (scoped) | `var(--glass-border)` | **none** | **none** | "Filter apps..." |

**Issues:**
- Apps search has **no label and no aria-label** (accessibility violation)
- Apps search uses `var(--glass-border)` instead of `var(--border)` for its border
- Apps search has **no focus box-shadow** (the only input missing it)

#### 3.2 Select Elements

Only 2 instances (both in Duplicates.vue). They override global select styles:
- Scoped padding `6px var(--sp-2)` vs global `7px 10px`
- Scoped `var(--radius-sm)` vs global `var(--radius-xs)`
- Scoped `var(--surface)` bg vs global `rgba(255,255,255,0.5)`
- No focus box-shadow (unlike text inputs)

#### 3.3 Checkboxes

~20+ instances across 6 views. All use the global `input[type="checkbox"]` (16x16, accent-color). **Consistent.**

Minor divergences in "Select all" wrappers:

| View | Gap in `.select-all-label` | Has text? |
|---|---|---|
| Caches | 8px | "Select all" |
| Logs | 6px | "Select all" |
| Vault | 6px | "Select all" |
| LargeFiles | (default) | **No text** (bare checkbox) |

#### 3.4 Toggle Switches

Only exist in Settings.vue (2 instances: window drag + scan area toggles). Implementation is in scoped CSS — not reusable elsewhere without duplication.

**Recommendation:** Move toggle styles to global CSS or extract a `<ToggleSwitch>` component.

#### 3.5 Segmented/Pill Controls

See Section 2.3 above — 5 different implementations of the same pattern.

---

### 4. Status Indicators & Feedback

#### 4.1 Spinner Class Bug

| Pattern | Views | Issue |
|---|---|---|
| `class="spinner-sm"` (correct) | Caches, Logs, LargeFiles, Dashboard | White track, designed for dark button backgrounds |
| `class="spinner spinner-sm"` (buggy) | Apps, Browsers, Duplicates, Packages, Security, Maintenance, Vault | `.spinner` sets dark track, `.spinner-sm` overrides to white — conflicting rules |

#### 4.2 Loading States

| Pattern | Views |
|---|---|
| `.loading-state` + `<span class="spinner">` (standard) | Caches, Logs, Apps, Browsers, Duplicates, Packages, Security, Maintenance, Docker, Trash, SystemVitals, Cpu |
| `.scanning-state.card` + `<div class="spinner">` (card-wrapped) | Memory |
| `.loading-state` + `<div class="spinner-sm">` | Thermal |
| Custom 3-dot pulse animation | Vault |

**Recommendation:** Memory should use the standard `.loading-state` pattern. Vault's 3-dot animation is unique but unnecessary — use the standard spinner.

#### 4.3 Live Refresh Indicators (3 different patterns)

| View | Pattern | Dot size | Pulse opacity |
|---|---|---|---|
| Memory | `.live-dot` + `.live-label` + `.live-updated` | 8px | 1 → 0.4 |
| Cpu, SystemVitals | `.live-badge` pill with `.live-dot` | 5px | 1 → 0.3 |
| Thermal | Plain text "Updated HH:MM:SS" | N/A | none |
| Dashboard | "Last scan: X min ago" text | N/A | none |

**Recommendation:** Extract `<LiveIndicator>` component with dot + optional timestamp.

**Design decision needed:** Dot size 5px or 8px? Pill badge (Cpu/SystemVitals) or standalone dot (Memory)?

#### 4.4 Health Threshold Inconsistencies

**CPU load color thresholds:**
| View | Danger | Warning | Normal |
|---|---|---|---|
| SystemVitals | >80% | >50% | ≤50% |
| Cpu.vue | >50% | >20% | ≤20% |

**Memory pressure thresholds:**
| View | Critical/High | Moderate | Low |
|---|---|---|---|
| Memory.vue | >85% | >65% | ≤65% |
| MemoryCard.vue | ≥90% / ≥75% | ≥50% | <50% |

**Fan color thresholds:**
| View | Critical | Warm | Normal |
|---|---|---|---|
| Thermal.vue | ≥80% | ≥50% | <50% |
| FanCard.vue | >70% | >40% | ≤40% |

**Recommendation:** Centralize all health thresholds in `utils.ts` as shared functions (like `tempToColor` already is). The current duplication guarantees dashboard cards show different colors than detail views for the same data.

#### 4.5 Status Dots (6+ independent implementations)

| Context | Class | Size | Defined in |
|---|---|---|---|
| Thermal state | `.thermal-dot` | (varies) | Dashboard, SystemVitals (scoped) |
| Memory pressure | `.thermal-dot` (misnamed) + `.dot-success/warning/danger` | 5px | MemoryCard (scoped) |
| Battery condition | `.thermal-dot` (misnamed) + `.dot-success/warning/danger` | 5px | BatteryCard (scoped) |
| Live indicator | `.live-dot` | 5–8px | Memory, Cpu, SystemVitals (scoped) |
| FDA status | `.status-dot` | 10px | Settings (scoped) |
| FDA warning | `.fda-warning-dot` | 8px | FdaWarningBanner (global) |
| Info banner | `.info-dot` | 8px | Maintenance (scoped) |
| Domain idle | `.domain-idle-dot` | (small) | Dashboard (scoped) |
| Category color | `.group-color-dot` | (small) | Memory (scoped) |

**Issues:**
- `.thermal-dot` name used for memory and battery contexts (misleading)
- `.dot-success/.dot-warning/.dot-danger` defined independently in MemoryCard and BatteryCard scoped CSS (identical but duplicated)
- No global status dot class exists

**Recommendation:** Extract global `.status-dot` class with `.status-dot--success`, `.status-dot--warning`, `.status-dot--danger` variants. Size controlled by modifier (`.status-dot--sm`, `.status-dot--lg`).

#### 4.6 Source Badges (duplicated across views)

| View | Badge class | Variants |
|---|---|---|
| Apps.vue | `.source-badge` (scoped) | `.source-homebrew`, `.source-app-store` |
| Packages.vue | `.source-badge` (scoped) | `.source-homebrew`, `.source-nvm`, `.source-rustup`, `.source-manual` |

**Verdict:** Identical base styling, independently defined. `.source-homebrew` appears in both. Should be a shared global class.

#### 4.7 Feedback Messages

| Pattern | Used in | Mechanism |
|---|---|---|
| `.success-message` (inline, persistent) | Caches, Logs, Apps, Browsers, Duplicates, Docker, Trash, LargeFiles, Security, Cpu | Global CSS class |
| `.error-message` (inline, persistent) | Same views + Memory, SystemVitals | Global CSS class |
| Toast component (floating, auto-dismiss) | Vault only | `Toast.vue` component |

**Recommendation:** Toast is better UX (auto-dismiss, animated, non-intrusive). Consider promoting it app-wide rather than having inline success/error messages that the user must scroll to see.

#### 4.8 Warning Banners

| Banner | Background | Border color | Used in |
|---|---|---|---|
| FDA Warning | `rgba(255,252,240,0.85)` | `rgba(229,163,15,0.15)` | 7 views via `FdaWarningBanner.vue` |
| Dashboard FDA | Inline `.fda-notice` | Different styling | Dashboard only |
| Settings Denied | `var(--warning-tint)` | `rgba(194,122,18,0.1)` | Settings only |
| Maintenance Info | `var(--accent-light)` | `rgba(0,180,216,0.1)` | Maintenance only |
| Vault Candidate Warning | `var(--warning-tint)` | N/A | Vault only |

**Design decision needed:** Dashboard FDA notice looks different from the shared `FdaWarningBanner` — should it use the same component?

---

### 5. Layout Patterns & Navigation

#### 5.1 Page Header

All 19 views use `.view-header` with `<h2>` title and `<p class="text-muted">` subtitle. Most use `.view-header-top` for side-by-side title+actions layout. **Mostly consistent**, with exceptions:

| Divergence | Views |
|---|---|
| No `.view-header-top` wrapper (custom flex on `.view-header`) | Thermal, Memory |
| No `.view-header-top` (no right-side actions) | Maintenance, Settings |
| Custom `lf-header` class overriding layout | LargeFiles |
| Scoped `align-items: baseline` override | Dashboard |

**Recommendation:** Minor — the scoped overrides are functional but could use a `.view-header--inline` modifier instead.

#### 5.2 Page Content Max-Width

All 19 views use `max-width: 1440px`. **Fully consistent.**

#### 5.3 Section Headers

| View | Font size | Font weight | Spacing below |
|---|---|---|---|
| Cpu | 14px | 600 | via `.section-header` |
| Thermal | 15px | 600 | above sections |
| Security | 15px | 600 | clickable, with chevron |
| Vault | 15px | 600 | `var(--sp-4)` |
| Memory | 15px | 600 | inside card |
| Packages | 16px | 600 | `var(--sp-4)` |
| Settings | 16px | 600 | `var(--sp-3)` |

**Issue:** No global `.section-header` class. Font sizes range from 14px to 16px. Each view independently styles its `<h3>` elements.

**Recommendation:** Add global `.section-title` class at 15px/600 with consistent spacing.

#### 5.4 Summary/Results Bars

| View | Class | margin-bottom |
|---|---|---|
| Caches | `.summary-bar` (global) | `var(--sp-4)` |
| Logs | `.results-bar` (scoped) | `var(--sp-3)` |
| LargeFiles | `.results-summary` (scoped) | varies |
| Browsers | `.summary-bar` (global) | `var(--sp-4)` |
| Security | `.summary-row` (scoped) | varies |

**Issue:** 4 different class names for the same flex-row pattern. Only Caches and Browsers use the global `.summary-bar`.

**Recommendation:** All should use the global `.summary-bar`.

#### 5.5 Expand Chevrons

| Pattern | Views |
|---|---|
| Global `.expand-chevron` (correct) | LargeFiles, Apps, Memory, Packages, Security, Maintenance, Browsers |
| `.category-chevron` (scoped duplicate) | Caches, Logs |
| `.cat-chevron` (scoped, different SVG viewBox) | Thermal |

**Recommendation:** Replace scoped chevron classes with the global `.expand-chevron`.

#### 5.6 Tab Bars

| View | Container | Active style |
|---|---|---|
| Vault | `.tab-bar` | underline + accent color |
| Duplicates | `.tab-switcher` (role="tablist") | underline + accent color |
| SpaceMap | `.viz-switcher` | filled background |

**Recommendation:** Extract `<TabBar>` component. Vault and Duplicates are nearly identical and should share one implementation.

#### 5.7 List Item Layout

| View | Grid columns | Vertical padding | Divider |
|---|---|---|---|
| Caches | `44px 1fr 90px 28px` | 10px | `1px solid rgba(0,0,0,0.04)` |
| Logs | `36px 1fr 90px 28px` | 8px | `1px solid rgba(0,0,0,0.04)` |
| LargeFiles | `36px 1fr 28px 100px 28px` | 8px | custom |

**Issue:** Caches and Logs are nearly identical but diverge on icon column width (44px vs 36px) and vertical padding (10px vs 8px).

---

## Proposed Component Library

| # | Component | Consolidates | Instances | Effort | Design Decisions Needed |
|---|---|---|---|---|---|
| 1 | `StatCard` | 5 naming variants (`.stat-card`, `.stat`, `.summary-card`, `.summary-stat`) | 14 usages | MEDIUM | Value font-size (18 vs 24px), background token, gap |
| 2 | `SegmentedControl` | 5 toggle/filter implementations | 12 usages | MEDIUM | Shape (pill vs rect), active style (fill vs tint) |
| 3 | `TabBar` | 3 tab implementations | 6 usages | SMALL | Already similar — just merge Vault + Duplicates |
| 4 | `LiveIndicator` | 3 live-refresh patterns | 5 usages | SMALL | Dot size, pill vs standalone |
| 5 | `StatusDot` | 6+ dot implementations | ~15 usages | SMALL | None — straightforward color variants |
| 6 | `CloseButton` | 4 dismiss button variants | 4 usages | SMALL | None — all are 14x14 X icons |
| 7 | `SectionHeader` | 7 independent h3 styles | 14 usages | SMALL | Font-size 15px (majority) |
| 8 | `EmptyState` | 3 empty state variants | 13 usages | SMALL | Promote Vault's richer pattern |
| 9 | `ToggleSwitch` | Settings-only scoped toggle | 9 rendered | SMALL | None — move to global |
| 10 | `SourceBadge` | 2 duplicated scoped badge sets | 6 usages | SMALL | Merge variant lists |
| 11 | `HealthThresholds` (util) | 3 mismatched threshold sets (CPU, memory, fan) | 6 usages | MEDIUM | Pick canonical thresholds |
| 12 | `RevealButton` | 2 reveal-in-Finder implementations | 3 usages | SMALL | Pick one icon style |

---

## Recommended Extraction Order

Priority: highest instance count × lowest effort = biggest impact.

1. **`StatusDot`** — ~15 usages, SMALL effort. Fixes `.thermal-dot` misname, eliminates 6 scoped duplicates.
2. **`SectionHeader`** — 14 usages, SMALL effort. One global class replaces 7 scoped h3 styles.
3. **`StatCard`** — 14 usages, MEDIUM effort. Needs design decisions but eliminates 5 naming variants.
4. **`SegmentedControl`** — 12 usages, MEDIUM effort. Replaces 5 independent implementations.
5. **`EmptyState` (enhanced)** — 13 usages, SMALL effort. Promote Vault's icon+title+description pattern.
6. **`HealthThresholds` utility** — 6 usages, MEDIUM effort. Critical for correctness — dashboard and detail views currently show different colors for the same data.
7. **`LiveIndicator`** — 5 usages, SMALL effort.
8. **`TabBar`** — 6 usages, SMALL effort.
9. **`ToggleSwitch`** — 9 rendered instances, SMALL effort. Move from scoped to global/component.
10. **`SourceBadge`** — 6 usages, SMALL effort. Merge two scoped definitions.
11. **`CloseButton`** — 4 usages, SMALL effort.
12. **`RevealButton`** — 3 usages, SMALL effort.

**Also fix (no new component needed):**
- Spinner double-class bug (`spinner spinner-sm` → `spinner-sm`) — 8 views
- Summary bar class names → use global `.summary-bar` — 3 views
- Expand chevron → use global `.expand-chevron` — 2 views (Caches, Logs)
- FDA banner buttons → use global `btn-primary btn-sm` / `btn-secondary btn-sm`
- Apps search input: add `aria-label`, fix focus shadow, use `var(--border)`
- Group container: extract `.group-container` for Caches/Logs/Duplicates

---

## Design Decisions for Human Review

1. **Stat card value font-size:** 18px (Apps, Packages, Duplicates, Vault) vs 24px (Security). Majority says 18px, but Security may want emphasis.

2. **Stat card background:** `var(--glass)` (Apps/Duplicates) vs `var(--surface)` (Security/Vault). Both are white-ish with different opacities.

3. **Segmented control active state:** Accent fill (LargeFiles) vs accent tint (Packages) vs white+shadow (Duplicates/Vault) vs accent glow (Duplicates kind pills).

4. **Tab active indicator:** Underline (Duplicates, Vault) vs filled background (SpaceMap). These may be intentionally different patterns (tabs vs mode switcher).

5. **Live indicator style:** 8px standalone dot (Memory) vs 5px dot in pill badge (Cpu, SystemVitals). Pill badge is more polished.

6. **Pause button:** Text-based `btn-ghost btn-sm` "Pause"/"Resume" (Thermal, Cpu, SystemVitals) vs icon-only 32x32 play/pause (Memory).

7. **CPU load danger threshold:** >80% (SystemVitals) vs >50% (Cpu). This is a correctness bug — both views show the same CPU data.

8. **Memory pressure labels and thresholds:** "High/Moderate/Low" at 85/65% (Memory.vue) vs "Critical/High/Moderate/Low" at 90/75/50% (MemoryCard.vue). Again, same data, different presentation.

9. **Fan color thresholds:** ≥80/50% (Thermal.vue) vs >70/40% (FanCard.vue). Same fans, different colors.

10. **Category container radius:** `var(--radius-md)` = 16px (Caches, Logs) vs `var(--radius-lg)` = 20px (Duplicates). Majority says 16px.

11. **Empty state richness:** Simple text-only (most views) vs icon + title + description (Vault). Vault's approach is better UX but requires adding icons to every empty state.

12. **Feedback mechanism:** Inline persistent `.success-message`/`.error-message` (most views) vs auto-dismissing Toast (Vault only). Toast is better UX but requires rethinking feedback flow in views that show success inline.

13. **Dashboard FDA notice:** Uses custom `.fda-notice` inline styling. Should it use the shared `FdaWarningBanner` component instead?
