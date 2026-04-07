# Visual Consistency Audit Report

> Generated: 2026-04-05
> Standards: CODING_STANDARDS.md (Sections 11, 12)
> Scope: All 30 `.vue` files + `src/style.css` + `src/utils.ts`

---

## Executive Summary

- **Total inconsistencies found: 127**
- **Critical (user-visible, jarring): 18**
- **Moderate (noticeable on comparison): 52**
- **Minor (subtle, may not notice): 57**

| Area | Rating | Notes |
|------|--------|-------|
| Colour | Moderate | Good token system, ~40 hardcoded bypasses |
| Typography | Poor | 10+ font-size values for labels alone, no unified scale |
| Spacing | Poor | 28+ uses of off-grid `10px`, pervasive token bypass |
| Component Variants | Moderate | Cards/badges diverge per-view, buttons mostly OK |
| Icons | Good | One consistent size issue (10×10 vs 12×12 chevrons) |
| Interactive States | Poor | `<button>` focus-visible broken globally, missing disabled states |
| Animations | Good | Minor duration drift (0.1s vs 0.12s vs 0.15s) |
| Responsive | N/A | Only 1 breakpoint in entire app (desktop-only Tauri app) |

**Best consistency:** Sidebar elements, modal pattern, spinner usage, expand chevron logic
**Worst consistency:** Uppercase label typography, stat-card padding, hardcoded colors in Toast/Maintenance, interactive state coverage

---

## 1. Colour Inconsistencies

### 1.1 Exact Token Duplicates (should use `var()` directly)

These are hardcoded values that **exactly match** an existing design token.

| Value | Token | File | Line(s) | Severity |
|-------|-------|------|---------|----------|
| `#E5700F` | `--thermal-serious` | SystemVitals.vue | 93 | Critical |
| `#c2410c` | `--source-homebrew` | Apps.vue | 478 | Critical |
| `#d94b4b` | `--danger` | LargeFiles.vue | 488, 808 | Critical |
| `#ffffff` | `--sidebar-text` | App.vue | 1057, 1100, 1139 | Moderate |

### 1.2 Near-Duplicate Colours (different values, same semantic purpose)

| Purpose | Values Found | Files | Proposed Fix |
|---------|-------------|-------|-------------|
| Success text on tint | `#1E8E5A` (token), `#1a8d36`, `#1a7a3a` | style.css, Maintenance.vue:509, Toast.vue:61 | All use `var(--success-text)` |
| Danger text | `#C94444` (token), `#c03030` | style.css, Toast.vue:67 | Use `var(--danger-text)` |
| Info text | `#148AA0` (token), `#1a6e80` | style.css, Toast.vue:73 | Use `var(--info-text)` |
| Danger base red | `#D94B4B` (token) vs `rgb(255,69,58)` (Apple systemRed) | style.css vs Toast.vue, Maintenance.vue, Memory.vue | Consolidate to `var(--danger-tint)` for backgrounds |
| Info/accent tint | `rgba(0,180,216,…)` vs `rgba(59,199,232,…)` | Maintenance.vue, Toast.vue, Dashboard.vue vs Duplicates.vue, Logs.vue | Use `var(--accent-light)` or `var(--info-tint)` |
| Warning tint | `rgba(255,159,10,…)` (Apple orange) vs `rgba(251,146,60,…)` (Tailwind orange) vs `#E5A30F` (token) | Memory.vue, Maintenance.vue, Apps.vue | Use `var(--warning-tint)` |

### 1.3 MemoryCard vs Utils.ts — Semantic Colour Mismatch

`MemoryCard.vue` defines its own `hsla()` palette for memory segments (lines 27-54) that does **not** match the `--mem-*` CSS tokens consumed by `utils.ts`:

| Segment | MemoryCard.vue | Token (utils.ts) | Visual Difference |
|---------|---------------|-------------------|-------------------|
| App | `hsla(195,55%,42%,0.75)` ≈ `rgb(49,130,153)` | `--mem-app: #00b4d8` = `rgb(0,180,216)` | **Much darker** |
| Wired | `hsla(35,55%,45%,0.75)` | `--mem-wired: #f97316` | Different hue entirely |
| Compressed | `hsla(280,35%,50%,0.75)` | `--mem-compressed: #a78bfa` | Similar purple, different shade |
| Free | `hsla(140,20%,70%,0.75)` | `--mem-free: #30d158` | Desaturated vs vivid green |

This means the Dashboard's MemoryCard ring chart and the Memory view's bar chart show the **same data in different colours**.

### 1.4 Hardcoded `rgba(59,199,232,…)` — Raw Accent Values

~25 instances of the raw `--accent` RGB value appear in scoped styles instead of using tokens:

| Pattern | Token Available | Files |
|---------|----------------|-------|
| `rgba(59,199,232,0.06–0.10)` | `--accent-light` (0.10) | Duplicates.vue, LargeFiles.vue, Logs.vue |
| `rgba(59,199,232,0.18)` | `--accent-glow` (0.18) | Duplicates.vue, App.vue |
| `rgba(59,199,232,0.25–0.30)` | (shadow values) | style.css (already in token file — these are OK) |

### 1.5 Colours Needing New Tokens

| Colour Family | Current Usage | Proposed Tokens |
|---------------|--------------|-----------------|
| Vault purple `hsl(280°)` | 15+ values in LargeFiles.vue | `--vault: hsla(280,40%,50%,1)`, `--vault-tint`, `--vault-text`, `--vault-muted` |
| Protect green `hsl(145°)` | 6 shield icons in LargeFiles.vue | `--protect-green: hsla(145,55%,45%,1)` |
| Thermal scale `hsl(0–195°)` | Thermal.vue, FanCard.vue, ThermalCard.vue | `--temp-cool`, `--temp-warm`, `--temp-hot`, `--temp-critical` |
| App Store blue | Apps.vue:483 | `--source-appstore: #2563eb` |
| Viz dark base `rgb(12–22,16–28,42–58)` | VoronoiViz.vue, SpaceMap.vue, GalacticViz.vue | `--viz-dark-base`, `--viz-dark-fill`, `--viz-stroke` |

### 1.6 Cleanest Files (zero hardcoded colours)

Trash.vue, Caches.vue, Vault.vue, Browsers.vue, Docker.vue, Security.vue, Packages.vue, FdaWarningBanner.vue

---

## 2. Typography Inconsistencies

### 2.1 Page Title / Hero Numbers — No Unified Scale

Elements that serve as the "biggest number on the page" use **six different sizes**:

| Size | Weight | Where | Semantic Role |
|------|--------|-------|---------------|
| 32px | 700 | Trash.vue `.stat-value-large` | Trash item count |
| 28px | 700 | style.css `.view-header h2`, Dashboard.vue `.summary-value` | View title, scan summary |
| 26px | 700 | App.vue `.fda-gate-content h1` | Onboarding gate title |
| 26px | 600 | Dashboard/SystemVitals/FanCard `.stat-hero` | Card hero number |
| 24px | 700 | Security.vue `.summary-count`, Duplicates.vue `.card-overflow-count` | Summary stat |
| 22px | 600 | BatteryCard.vue `.bat-ring-value` | Ring chart centre |
| 20px | 700 | Memory.vue `.stats-total-value` | Total memory |
| 18px | 700 | Apps/Packages/Duplicates/Vault `.stat-card-value` | Stat card number |
| 18px | 600 | MemoryCard.vue `.mem-ring-value` | Ring chart centre |
| 18px | 500 | Thermal.vue `.sc-value` | Summary card value |

**Proposed scale:** 28px (page hero), 22px (card hero), 18px (stat number), 14px (inline stat).

### 2.2 Uppercase Label Typography — Fragmented

All uppercase small labels share the pattern (small / uppercase / letter-spacing) but use **inconsistent values**:

| File | Selector | Size | Weight | Letter-spacing |
|------|----------|------|--------|---------------|
| Dashboard/SystemVitals | `.stat-label` | 10px | 600 | 0.8px |
| Apps/Packages | `.stat-card-label` | 11px | 500 | 0.4px |
| Cpu/Thermal/SystemVitals | `.section-title` | 11px | 600 | 0.8px |
| Dashboard/SystemVitals | `.info-strip-label` | 10px | 600 | 0.4px |
| Thermal | `.sc-label` | 10px | 600 | 0.6px |
| Vault | `.summary-stat-label` | 10px | 600 | 0.4px |
| Apps | `.expanded-label` | 11px | 600 | 0.5px |
| Packages | `.removal-label` | 11px | 600 | 0.3px |
| Dashboard | `.side-card-title` | 10px | 600 | 0.1em (~1.3px) |
| Maintenance | `.detail-section-label` | 11px | 700 | 0.5px |
| ThermalCard | `.tstrip-label` | 9px | 600 | 0.2px |
| BatteryCard | `.bat-meta-label` | 9px | 500 | 0.3px |
| Trash | `.stat-label` | 13px | 500 | 0.5px |

**Proposed standard:** `--label-sm: 10px/600/0.5px` for dense cards, `--label-md: 11px/600/0.5px` for section headers.

### 2.3 In-View h3 Weight Inconsistency

All in-view `h3` elements use 16px/600 **except**:
- Dashboard `.scan-header h3`: 16px/**700** (the only bold one)
- Modal `.modal-content h3`: 18px/700 (acceptable — different layer)

### 2.4 Button Font Size — System Default Rarely Used

The global `button` is 13px/500, but in practice:
- Most view buttons (filter, sort, toggle, expand, FAB): **12px/500** or **11px/500**
- Onboarding gate buttons: **14px/600** (the only buttons larger than system default)
- The 13px default is almost never directly applied

### 2.5 Hardcoded `font-family` (Bypassing `var(--font-sans)`)

| File | Selector | Line |
|------|----------|------|
| LargeFiles.vue | `.file-name` | 1251 |
| LargeFiles.vue | `.file-row-path` | 1434 |
| VoronoiViz.vue | `.voronoi-label` | 1521 |
| VoronoiViz.vue | `.cluster-label-name` | 1802 |
| VoronoiViz.vue | `.cluster-label-size` | 1810 |

All should use `var(--font-sans)` or `font-family: inherit`.

### 2.6 Non-Standard `font-weight` Values

| File | Selector | Weight | Line |
|------|----------|--------|------|
| Cpu.vue | `.idle-name` | **550** | 620 |
| VoronoiViz.vue | `.voronoi-location` | **550** | 1622 |
| VoronoiViz.vue | `.cluster-label-name` | **650** | 1804 |

These render correctly on macOS (SF Pro is a variable font) but would snap to 500/700 on non-Apple fallback fonts. Acceptable for Tauri-macOS-only but should be documented.

### 2.7 Ultra-Light Weights Used Only Once

| File | Selector | Weight | Notes |
|------|----------|--------|-------|
| Dashboard.vue | `.reclaim-value` | **200** | Only ultralight text in the entire app |
| Dashboard.vue | `.file-size`, `.file-path`, `.reclaim-breakdown` | **300** | Only light text in the app |

---

## 3. Spacing Inconsistencies

### 3.1 Off-Grid Values (Not on 4px/8px Scale)

The design system defines `--sp-1` through `--sp-12` on a 4px base grid. The following values violate this grid:

| Value | Occurrences | Files (examples) | Nearest Token |
|-------|-------------|-------------------|---------------|
| **10px** | ~28 | Dashboard gap, SystemVitals gap, Thermal gap, Packages gap, Cpu gap, App.vue sidebar nav | `--sp-3` (12px) or `--sp-2` (8px) |
| **14px** | ~16 | Dashboard/SystemVitals `.stat-card` padding-top, Thermal card padding, sidebar footer | `--sp-3` (12px) or `--sp-4` (16px) |
| **6px** | ~22 | FanCard gap, ThermalCard gap, Cpu action gap, Vault gap, Packages gap | `--sp-1` (4px) or `--sp-2` (8px) |
| **5px** | ~8 | `.live-badge` gap, `.fan-bars` margin, `.proc-row` padding | `--sp-1` (4px) |
| **9px** | ~4 | Global `button` padding, `input` padding | `--sp-2` (8px) |
| **7px** | ~4 | `.btn-ghost` padding, `.btn-fda` padding | `--sp-2` (8px) |
| **18px** | ~8 | Sidebar header padding, Dashboard `.side-card` padding | `--sp-5` (20px) |
| **22px** | ~4 | Sidebar header/footer horizontal padding | `--sp-5` (20px) or `--sp-6` (24px) |
| **3px** | ~6 | Cpu heatmap gap, tstrip border-radius, Vault tab-bar padding | `--sp-1` (4px) |
| **2px** | ~12 | Half-grid sub-element gaps | `--sp-1` (4px) |

### 3.2 On-Grid Values That Bypass Tokens

These use correct 4px-multiple values but hardcode them instead of using `var(--sp-*)`:

| Value | Token | Approx. Occurrences |
|-------|-------|---------------------|
| `12px` | `--sp-3` | ~26 |
| `16px` | `--sp-4` | ~24 |
| `8px` | `--sp-2` | ~20 |
| `4px` | `--sp-1` | ~14 |
| `24px` | `--sp-6` | ~12 |
| `48px` | `--sp-12` | ~6 |
| `40px` | `--sp-10` | ~2 |

### 3.3 Global Button/Input Padding — Off-Grid

The most impactful off-grid values are in `style.css` itself:

| Element | Current Padding | Proposed |
|---------|----------------|----------|
| `button` (L238) | `9px 18px` | `var(--sp-2) var(--sp-5)` (8px 20px) |
| `input` (L341) | `9px 14px` | `var(--sp-2) var(--sp-4)` (8px 16px) |
| `.btn-ghost` (L311) | `7px 14px` | `var(--sp-2) var(--sp-3)` (8px 12px) |
| `.btn-sm` (L320) | `6px 14px` | `var(--sp-1) var(--sp-3)` (4px 12px) |
| `.success-message` (L594) | `14px 18px` | `var(--sp-3) var(--sp-5)` (12px 20px) |

### 3.4 Card Internal Padding — Inconsistent Across Views

| Pattern | Files | Value | Status |
|---------|-------|-------|--------|
| `.card` (global) | style.css | `var(--sp-6)` (24px) | Correct |
| `.stat-card` | Dashboard, SystemVitals | `14px 16px 12px` | Off-grid, asymmetric |
| `.stat-card` | Apps, Packages | `12px 16px` | On-grid, hardcoded |
| `.side-card` | Dashboard | `16px 18px` | 18px off-grid |
| `.summary-card` / `.fan-card` | Thermal | `12px 14px` | 14px off-grid |
| `.ai-summary-card` | Dashboard | `14px 18px` | Both off-grid |
| Toast `.toast` | Toast.vue | `10px 14px` | Both off-grid |

### 3.5 Border-Radius Bypassing Tokens

| Value | Token Equivalent | Files |
|-------|-----------------|-------|
| `10px` | None (between --radius-sm=12 and 8) | `.btn-sm`, Vault `.tab-bar`, Caches `.cache-item`, Toast, Memory `.memory-bar` |
| `8px` | None | Vault `.tab-btn`, Thermal `.cat-header`, Apps `.app-icon`, Browsers `.browser-icon` |
| `6px` | None | Toast `.toast-close`, Cpu `.btn-quit`, Browsers `.cache-icon img` |
| `4px` | None | Apps `.source-badge`, Packages `.dep-badge`, `.btn-quit-xs`, `.sensor-row` |
| `14px` | None (between --radius-sm=12 and --radius-md=16) | Caches `.cache-category`, Logs `.log-category` |
| `12px` | `--radius-sm` (hardcoded) | App.vue `.content-panel` |
| `3px` | None | ThermalCard `.tstrip-track`, scrollbar thumb |

**Proposed additions:** `--radius-xs: 6px`, `--radius-2xs: 4px` (for badges/small elements).

---

## 4. Component Visual Variants

### 4.1 One-Off Button Styles (Not Using System Classes)

| File | Selector | Deviation from System |
|------|----------|-----------------------|
| App.vue | `.btn-gate-primary`, `.btn-gate-secondary` | 14px/600, `12px 28px` padding — larger than any system button |
| Maintenance.vue | `.btn-task-success`, `.btn-task-error` | Custom colours, no `:active` state, hover uses `opacity` not `translateY` |
| Packages.vue | `.filter-btn` | `padding: 4px 12px`, `border-radius: 6px` — doesn't use `.btn-sm` |
| Packages.vue | `.deps-toggle` | `padding: 2px 8px`, `border-radius: 4px` — outside system |
| Vault.vue | `.tab-btn` | `padding: 7px 16px`, `border-radius: 8px` |
| Duplicates.vue | `.tab-btn` | `padding: 5px 14px` — different from Vault's `.tab-btn` |
| Cpu.vue | `.btn-quit` | `padding: 4px 12px`, `border-radius: 6px` |
| Cpu.vue | `.btn-quit-xs` | `padding: 1px 7px`, `border-radius: 4px` |

### 4.2 Card Surface Inconsistencies

| File | Selector | Radius | Background | Border |
|------|----------|--------|------------|--------|
| style.css `.card` | — | `--radius-md` (16px) | `var(--glass)` | `1px solid var(--glass-border)` |
| Caches/Logs `.cache-category`/`.log-category` | — | `14px` (hardcoded) | `rgba(255,255,255,0.4)` | `0.5px solid rgba(255,255,255,0.5)` |
| LargeFiles `.file-group` | — | `12px` (hardcoded) | `rgba(255,255,255,0.3)` | `0.5px solid rgba(255,255,255,0.5)` |
| Duplicates `.group-card` | — | `--radius-lg` (20px) | `rgba(255,255,255,0.45)` | `1px solid rgba(0,0,0,0.06)` |
| Security `.summary-card` | — | `--radius-md` (16px) | `var(--surface)` | **Missing entirely** |
| Vault `.summary-stat` | — | `--radius-md` (16px) | `var(--surface)` | **Missing entirely** |

### 4.3 Badge/Pill Inconsistencies

The global `.badge` (11px/600, `padding: 3px 10px`, `--radius-pill`) is rarely used directly. Most badge-like elements are scoped one-offs:

| File | Selector | Size | Padding | Radius |
|------|----------|------|---------|--------|
| style.css `.badge` | 11px | `3px 10px` | pill (100px) |
| style.css `.badge-pill` | 10px | `2px 8px` | 8px |
| Packages `.dep-badge` | 9px | `1px 5px` | 3px |
| Packages `.source-badge` | 10px | `2px 7px` | 4px |
| Apps `.source-badge` | 10px | `2px 7px` | 4px |
| Duplicates `.badge-keep` | 9px | `1px 5px` | 4px |
| LargeFiles `.safety-pill` | 9px | inherits | inherits |
| LargeFiles `.vault-badge` | 9px | inherits | inherits |
| Dashboard `.ai-badge` | 9px | inherits | inherits |

---

## 5. Icon & Image Sizing

### 5.1 Expand Chevron Size Inconsistency

Standard is 12x12 (defined in `style.css` `.expand-chevron svg`). Three files deviate:

| File | Line | Size | Should Be |
|------|------|------|-----------|
| Caches.vue | 217 | **10x10** | 12x12 |
| Logs.vue | 188 | **10x10** | 12x12 |
| Thermal.vue | 306 | **10x10** | 12x12 |

### 5.2 Viz Action Button Size Fragmentation

SpaceMap, VoronoiViz, and GalacticViz use three different sizes for the same type of small action button:

| Size | Files |
|------|-------|
| 11x11 | SpaceMap.vue (reveal button) |
| 13x13 | SpaceMap.vue (expand), VoronoiViz.vue (expand), GalacticViz.vue (expand) |
| 14x14 | SpaceMap.vue (viz switcher), VoronoiViz.vue (back/reset), GalacticViz.vue (reset) |

**Proposed:** Standardise all to 14x14.

### 5.3 Native Icon Retina Alignment

| File | Render Size | Display Size | Ratio | Aligned? |
|------|-------------|--------------|-------|----------|
| Caches.vue, Logs.vue | 64px | 28x28 | 2.29x | No — use 32x32 display |
| Browsers.vue | 64px | 44x44 | 1.45x | No — use 32x32 display |
| SpaceMap.vue | 32px | 25x25 | 1.28x | No — use 16x16 display |
| LargeFiles.vue (file row) | 64px | 32x32 | 2.0x | Yes |
| LargeFiles.vue (folder) | 32px | 16x16 | 2.0x | Yes |

### 5.4 Apps.vue Icon Fallback Size Mismatch

The app icon `<img>` is 40x40 via CSS, but the SVG fallback placeholder draws at 24x24 inside the 40x40 container — visual weight differs between loaded and fallback states.

---

## 6. Interactive States

### 6.1 Critical: `<button>` Focus-Visible Broken Globally

`style.css` line 232 sets `outline: none` on all `<button>` elements. The focus-visible rule (L858) only covers `[tabindex="0"]` and `[role="button"]` — **not `<button>` elements**. This means every button in the app lacks a keyboard focus indicator.

**Fix:** Add `button:focus-visible` to the focus-visible rule.

### 6.2 Missing Disabled States

| Selector | Has `:disabled`? |
|----------|-----------------|
| `.btn-primary` | Yes |
| `.btn-danger` | Yes |
| `.btn-secondary` | **No** |
| `.btn-ghost` | **No** |
| `.btn-sm` | **No** |

### 6.3 Missing `:active` States

| Element | Has `:active`? |
|---------|---------------|
| `.btn-ghost` | **No** |
| `.btn-task-success`, `.btn-task-error` (Maintenance) | **No** |
| All `.filter-btn`, `.tab-btn`, `.sort-btn`, `.deps-toggle` | **No** |
| `.nav-item` (App.vue sidebar) | **No** |

### 6.4 Missing Hover Feedback on Clickable Elements

| File | Element | Issue |
|------|---------|-------|
| Security.vue | `.section-header` | Has `cursor: pointer` but **no hover background change** |
| Settings.vue | `.area-item` | Has `transition: filter 0.2s` but **no hover state defined** |
| Duplicates.vue | `.file-card` | Clickable but **missing `cursor: pointer`** |

### 6.5 Input Focus Ring Inconsistency

`Apps.vue` `.search-input:focus` only sets `border-color: var(--accent)` — it does **not** apply the `box-shadow: 0 0 0 3px var(--accent-light)` ring that the global `input:focus` provides.

---

## 7. Animation & Transitions

### 7.1 Row Hover Duration — Three Values for Same Action

| Duration | Files |
|----------|-------|
| `0.10s` | Browsers.vue `.category-row` |
| `0.12s` | Caches.vue `.cache-item` / `.category-header`, Logs.vue `.log-item` / `.category-header` |
| `0.15s` | Apps.vue `.app-row`, Vault.vue `.candidate-card`, Packages.vue `.manager-header`, Security.vue `.finding-row`, LargeFiles.vue `.group-header` / `.dir-header` |

**Proposed:** Standardise all row hover transitions to `0.15s ease`.

### 7.2 Chevron Rotate Duration — Two Values

| Duration | Source |
|----------|--------|
| `0.2s ease` | `style.css` `.expand-chevron` (global) |
| `0.15s` | Caches.vue `.category-chevron`, Logs.vue `.category-chevron` (scoped, different class name) |

Caches and Logs define `.category-chevron` separately from the global `.expand-chevron`.

### 7.3 Bar Fill Animation — Three Easing Approaches

| Pattern | Duration/Easing | Files |
|---------|----------------|-------|
| Gauge/progress bars | `0.6s cubic-bezier(0.25,0.46,0.45,0.94)` | ThermalCard, FanCard, SystemVitals |
| Heat cell | `0.4s ease` | CpuCard |
| Footprint/progress | `0.3s ease` | Apps.vue, Duplicates.vue |

### 7.4 `fadeIn` Animation Duration Inconsistency

- `0.3s ease` — style.css (`.success-message`, `.error-message`, `.fda-warning-banner`)
- `0.2s ease` — LargeFiles.vue `.scan-progress-bar`

### 7.5 Missing Transition on Vault `.dismiss-btn`

Hover changes `opacity` and `background` without any transition, unlike the identical `.toast-close` in Toast.vue which uses `transition: opacity 0.15s, background 0.15s`.

---

## 8. Responsive Breakpoints

### 8.1 Only One Breakpoint in Entire App

`Dashboard.vue` line 642:
```css
@media (max-width: 720px) {
  .bottom-row { grid-template-columns: 1fr; }
}
```

This is the **only** `@media` query in the codebase. No breakpoint system exists — no CSS custom properties, no defined set.

### 8.2 Assessment

As a Tauri macOS desktop app, the lack of responsive design is less critical. However, if the minimum window size is not enforced at the native level, the following views are most at risk on narrow windows: SpaceMap.vue (multi-panel), LargeFiles.vue (deep nesting), Duplicates.vue (card grid).

**Recommendation:** Verify `tauri.conf.json` enforces a minimum window width of ~900px. If not, add at minimum one breakpoint at ~800px for the most complex layouts.

---

## Recommended Design Tokens

Based on this audit, the following token additions/changes would resolve the majority of inconsistencies:

### Colours — New Tokens
```css
/* Vault feature */
--vault: hsla(280, 40%, 50%, 1);
--vault-tint: hsla(280, 30%, 70%, 0.08);
--vault-text: hsla(280, 35%, 35%, 0.9);
--vault-muted: hsla(280, 30%, 30%, 0.6);
--vault-border: hsla(280, 40%, 60%, 0.15);

/* File protection */
--protect-green: hsla(145, 55%, 45%, 1);

/* Thermal scale */
--temp-cool: hsla(195, 40%, 40%, 1);
--temp-warm: hsla(40, 50%, 40%, 1);
--temp-hot: hsla(25, 45%, 40%, 1);
--temp-critical: hsla(0, 45%, 42%, 1);

/* Source badges */
--source-appstore: #2563eb;
--source-appstore-tint: rgba(59, 130, 246, 0.12);

/* Visualisation dark canvas */
--viz-dark-base: rgba(15, 18, 45, 1);
--viz-dark-fill: rgba(22, 28, 58, 1);
--viz-stroke: rgba(12, 16, 42, 0.82);
```

### Typography Scale — Proposed
```css
/* Hero numbers */
--text-hero-lg: 28px;    /* page-level stat */
--text-hero-md: 22px;    /* card-level stat */
--text-hero-sm: 18px;    /* inline stat */

/* Labels */
--text-label-sm: 10px;   /* dense card labels */
--text-label-md: 11px;   /* section headers */
--text-label-weight: 600;
--text-label-tracking: 0.5px;
```

### Spacing — Proposed Fixes
```
Global button:    9px 18px  →  var(--sp-2) var(--sp-5)   (8px 20px)
Global input:     9px 14px  →  var(--sp-2) var(--sp-4)   (8px 16px)
Pervasive 10px:   10px      →  var(--sp-3)               (12px)
Pervasive 6px:    6px       →  var(--sp-2)               (8px)
Pervasive 14px:   14px      →  var(--sp-4)               (16px)
```

### Border-Radius — Proposed Additions
```css
--radius-2xs: 4px;   /* badges, tiny pills */
--radius-xs: 6px;    /* small buttons, close icons */
/* existing: --radius-sm: 12px, --radius-md: 16px, --radius-lg: 20px, --radius-xl: 24px */
```

### Transition Defaults — Proposed
```css
--transition-fast: 0.15s ease;    /* row hovers, button state changes */
--transition-base: 0.2s ease;     /* standard UI transitions */
--transition-slow: 0.3s ease;     /* entrance animations, progress bars */
--transition-gauge: 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94);  /* gauges, data bars */
```

---

## Remediation Priority

### Tier 1 — High Impact, Low Effort (fix today)

1. **Fix `<button>` focus-visible** — Add `button:focus-visible` to the global focus-visible rule in `style.css`. One line, fixes accessibility for every button. (`style.css:858`)

2. **Replace 4 exact token duplicates** — `SystemVitals.vue:93` (#E5700F→`var(--thermal-serious)`), `Apps.vue:478` (#c2410c→`var(--source-homebrew)`), `LargeFiles.vue:488,808` (#d94b4b→`var(--danger)`), `App.vue:1057,1100,1139` (#ffffff→`var(--sidebar-text)`).

3. **Fix Toast.vue colours** — Lines 61, 67, 73: replace `#1a7a3a`→`var(--success-text)`, `#c03030`→`var(--danger-text)`, `#1a6e80`→`var(--info-text)`.

4. **Fix expand chevron sizes** — Caches.vue:217, Logs.vue:188, Thermal.vue:306: change `width="10" height="10"` to `width="12" height="12"`.

5. **Add disabled states** — Add `:disabled { opacity: 0.4; cursor: not-allowed; transform: none; }` to `.btn-secondary` and `.btn-ghost` in `style.css`.

### Tier 2 — Moderate Impact, Moderate Effort (this sprint)

6. **Standardise row hover transitions** — Replace all `0.10s` and `0.12s` row hover durations with `0.15s ease` across Browsers.vue, Caches.vue, Logs.vue.

7. **MemoryCard colour alignment** — Migrate `MemoryCard.vue` from bespoke `hsla()` values to the `--mem-*` token family used by `utils.ts`.

8. **Replace ~25 raw accent rgba values** — Change `rgba(59,199,232,0.10)` to `var(--accent-light)` and `rgba(59,199,232,0.18)` to `var(--accent-glow)` across Duplicates, LargeFiles, Logs, App.vue.

9. **Unify stat-card padding** — Replace `14px 16px 12px` (Dashboard/SystemVitals) and `12px 16px` (Apps/Packages) with a single `var(--sp-3) var(--sp-4)`.

10. **Add missing hover feedback** — Security.vue `.section-header`, Settings.vue `.area-item`, Duplicates.vue `.file-card` cursor.

### Tier 3 — Consistency Polish (next sprint)

11. **Create vault/thermal/viz token families** — Add the ~15 new CSS custom properties proposed above.

12. **Replace hardcoded font-family** — 5 instances in LargeFiles.vue and VoronoiViz.vue → `var(--font-sans)`.

13. **Standardise uppercase label scale** — Pick 10px/600/0.5px and 11px/600/0.5px; migrate all ~13 variants.

14. **Fix off-grid global button/input padding** — Shift from 9px to 8px in `style.css`.

15. **Standardise viz action button sizes** — Unify SpaceMap/VoronoiViz/GalacticViz action icons to 14x14.

16. **Replace `14px` border-radius** — Caches `.cache-category` and Logs `.log-category` should use `--radius-md` (16px) or `--radius-sm` (12px).

17. **Add `--radius-xs: 6px` and `--radius-2xs: 4px`** tokens for small elements, then migrate the ~20 hardcoded small radii.

18. **Add `:active` states** to `.btn-ghost` and all one-off button types.
