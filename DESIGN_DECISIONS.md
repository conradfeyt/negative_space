# Design Decisions
> Resolved: 2026-04-06
> Reference: COMPONENT_COHESION_REPORT.md

---

## StatCard
- **Value font-size:** 18px (Security shrinks to match majority)
- **Background:** `var(--surface)` — slightly more opaque, better readability
- **Container gap:** Standardize to 12px (`var(--sp-3)`)
- **Padding:** Standardize to `14px 16px`

## SegmentedControl
- **Active state:** Accent fill — solid `var(--accent)` background with white text
- **Applies to:** LargeFiles sort, Packages filter, Duplicates kind pills, SpaceMap viz/color/expand toggles

## TabBar
- **Active style:** Filled background — active tab gets white bg + subtle shadow (not underline)
- **Applies to:** Duplicates tab-switcher, Vault tab-bar, SpaceMap viz-switcher

## LiveIndicator
- **Style:** Pill badge — 5px pulsing dot inside a pill with "Live" text
- **Applies to:** Memory, Cpu, SystemVitals, Thermal (replace plain timestamp)

## PauseButton
- **Style:** Text-based ghost button (`btn-ghost btn-sm`) with "Pause"/"Resume" labels
- **Memory outlier:** Replace 32x32 icon-only button with text ghost button to match Thermal/Cpu/SystemVitals

## StatusDot
- **Canonical:** 5px circular dot with color variants (`--success`, `--warning`, `--danger`)
- **Global class:** `.status-dot` + `.status-dot--success` / `--warning` / `--danger`
- **Rename:** `.thermal-dot` → `.status-dot` everywhere (including memory and battery contexts)

## SectionHeader
- **Font-size:** 15px (majority across Thermal, Security, Vault, Memory)
- **Font-weight:** 600
- **Global class:** `.section-title` with consistent `margin-bottom: var(--sp-4)`

## EmptyState
- **Style:** Rich — SVG icon (40x40, 0.4 opacity) + title (16px/600) + description (13px)
- **Retrofit:** All current text-only empty states get promoted to the Vault pattern

## CategoryContainer
- **Border-radius:** `var(--radius-md)` = 16px (Duplicates group-card shrinks from 20px)
- **Background:** `rgba(255,255,255,0.4)` with `1px solid rgba(0,0,0,0.05)` border

## Health Thresholds (centralize in utils.ts)

### CPU Load
- **Danger:** >80%
- **Warning:** >50%
- **Normal:** ≤50%
- Source: SystemVitals thresholds. Cpu.vue's 50/20 values are wrong — too aggressive.

### Memory Pressure (4-tier)
- **Critical:** ≥90%
- **High:** ≥75%
- **Moderate:** ≥50%
- **Low:** <50%
- Source: MemoryCard thresholds. Memory.vue's 85/65 values are wrong.

### Fan Speed
- **Critical:** ≥80%
- **Warm:** ≥50%
- **Normal:** <50%
- Source: Thermal.vue thresholds. FanCard's 70/40 values are wrong — too aggressive.

## FDA Warning
- **Remove from all 7 view pages** — no more `FdaWarningBanner` component in content areas
- **Replace with:** Subtle global indicator in the sidebar (e.g., small warning icon or dot on affected nav items, or a persistent but minimal notice at the bottom of the sidebar)
- **Design needed:** Exact sidebar placement and visual treatment TBD

## Feedback / Toast System
- **Style:** Auto-dismissing toast, bottom-right of screen
- **Auto-dismiss:** ~60 seconds
- **Close button:** Always present for manual dismiss
- **Variants:** Success (green), error (red), info (blue)
- **Behaviour:** Toasts stack vertically, newest on bottom. Don't shift page content.
- **Replaces:** All inline `.success-message` and `.error-message` divs across every view
- **Current Toast.vue needs redesign:** Restyle, reposition to bottom-right fixed, improve animations

## Spinner Fix
- **Correct pattern:** `class="spinner-sm"` inside buttons (not `class="spinner spinner-sm"`)
- **Fix in:** Apps, Browsers, Duplicates, Packages, Security, Maintenance, Vault, SpaceMap (8 views)

## Source Badges
- **Merge:** Apps.vue and Packages.vue scoped `.source-badge` definitions into one global class
- **Combined variants:** `.source-homebrew`, `.source-app-store`, `.source-nvm`, `.source-rustup`, `.source-manual`

## Expand Chevron
- **Use global `.expand-chevron` everywhere**
- **Fix:** Caches `.category-chevron` → `.expand-chevron`, Logs `.category-chevron` → `.expand-chevron`, Thermal `.cat-chevron` → `.expand-chevron` (with standard 12x12 viewBox)

## Summary Bar
- **Use global `.summary-bar` everywhere**
- **Fix:** Logs `.results-bar` → `.summary-bar`, LargeFiles `.results-summary` → `.summary-bar`, Security `.summary-row` → `.summary-bar`

## Toggle Switch
- **Move from Settings.vue scoped CSS to global stylesheet or extract `<ToggleSwitch>` component**
- **Current implementation is correct** — just needs to be reusable

## Close/Dismiss Button
- **Global class:** `.btn-close` — 14x14 X SVG icon with consistent hover state
- **Replaces:** `.dismiss-btn`, `.btn-remove`, `.toast-close`, `.btn-preview-close`

## Reveal-in-Finder Button
- **Standardize icon:** Use the native folder icon approach (LargeFiles pattern) with SVG fallback
- **Single class:** `.btn-reveal`
