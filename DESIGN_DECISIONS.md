# Design Decisions
> Resolved: 2026-04-10 (updated after showcase tuning)
> Reference: COMPONENT_COHESION_REPORT.md

---

## Accent Color
- **Final:** `#0088FF` blue
- **Previous:** `#3BC7E8` aqua/teal (original), `#0275F4` (interim)
- Semantic aliases: `--accent` = `--blue` = `#0088FF`

## Color Tokens
Named-color-first system with semantic aliases:
- **Success:** `--green` (`#34C759`)
- **Warning:** `--yellow` (`#FF9F0A`)
- **Danger:** `--red` (`#FF393C`)
- **Info:** `--blue` (same as accent)
- **Neutral:** `--grey`

## StatCard
- **Value font-size:** 18px (Security shrinks to match majority)
- **Background:** `var(--surface)` — slightly more opaque, better readability
- **Container gap:** Standardize to 12px (`var(--sp-3)`)
- **Padding:** Standardize to `14px 16px`
- **Component:** `StatCard.vue` with `value`, `label` props

## TabBar (unified — includes SegmentedControl)
- **Merged:** SegmentedControl and TabBar into a single `TabBar.vue` component
- **Active style:** Pill styling with solid fill
- **Features:** Disabled items, badges, scoped slot support
- **Applies to:** LargeFiles sort, Packages filter, Duplicates tabs + kind pills, Vault tabs, SpaceMap viz/color/expand toggles

## LiveIndicator
- **Style:** Pill badge — 5px pulsing dot inside a pill with "Live" text
- **Component:** `LiveIndicator.vue`
- **Applies to:** Memory, Cpu, SystemVitals, Thermal (replaces plain timestamp)

## PauseButton
- **Style:** Text-based ghost button (`btn-ghost btn-sm`) with "Pause"/"Resume" labels
- **Memory outlier:** Replaced 32x32 icon-only button with text ghost button to match Thermal/Cpu/SystemVitals

## StatusDot
- **Resolved as:** Global `.status-dot` CSS class with `.status-dot--success` / `--warning` / `--danger` variants
- **Not a component** — CSS-only solution was sufficient
- **Renamed:** `.thermal-dot` to `.status-dot` everywhere

## SectionHeader
- **Font-size:** 15px (majority across Thermal, Security, Vault, Memory)
- **Font-weight:** 600
- **Resolved as:** Global `.section-title` CSS class with consistent `margin-bottom: var(--sp-4)`

## EmptyState
- **Style:** Rich — customizable icon slot + title (16px/600) + description (13px)
- **Component:** `EmptyState.vue` with icon slot, title prop, description prop
- **All text-only empty states promoted** to the rich pattern

## CategoryContainer
- **Border-radius:** `var(--radius-md)` = 16px (Duplicates group-card shrunk from 20px)
- **Background:** `rgba(255,255,255,0.4)` with `1px solid rgba(0,0,0,0.05)` border

## Card Opacity
- **Unified:** All glass cards use `var(--glass)` = `rgba(255,255,255,0.45)`
- **Previous:** Various values from 0.28 to 0.75 across views

## Health Thresholds (centralized in utils.ts)

### CPU Load
- **Danger:** >80%
- **Warning:** >50%
- **Normal:** <=50%
- Functions: `cpuLoadColor()`, `cpuLoadClass()`

### Memory Pressure (4-tier)
- **Critical:** >=90%
- **High:** >=75%
- **Moderate:** >=50%
- **Low:** <50%
- Functions: `memoryPressureLevel()`, `memoryPressureDotClass()`

### Fan Speed
- **Critical:** >=80%
- **Warm:** >=50%
- **Normal:** <50%
- Functions: `fanSpeedColor()`, `fanSpeedZone()`

### Storage
- Function: `storageColor()`

## Badge System
- **Style:** Border-based with `text-transform: uppercase`
- **Modifiers:** `.pill` (fully rounded), `.source` (no border, tinted background)
- **Classes:** `.badge-accent`, `.badge-success`, `.badge-warning`, `.badge-danger`, `.badge-info`, `.badge-neutral`
- **Source badges:** Merged Apps.vue and Packages.vue scoped definitions into global `.source` modifier

## Button System
- **Primary:** `.btn-primary` — solid `#0088FF` blue, white text
- **Secondary:** `.btn-secondary` — grey fill, no border
- **Danger:** `.btn-danger` — solid `#FF393C` red, white text (not tinted)
- **Ghost:** `.btn-ghost` — `var(--text)` color, no background (not accent-colored)
- **Ghost danger:** `.btn-ghost.danger` — red text variant
- **Close:** `.btn-close` — 14x14 X SVG icon with consistent hover state
- **Reveal:** `.btn-reveal` — standardized reveal-in-Finder button

## Checkbox
- **Component:** `Checkbox.vue`
- **Prop:** `modelValue` only (not `isOn` or `checked`)
- **Features:** Animated check mark, indeterminate support, v-model binding

## FileRow
- **Component:** `FileRow.vue`
- **Features:** Icon, name, path, safety badge, size, reveal button, checkbox

## ScanBar
- **Component:** `ScanBar.vue`
- **Style:** Pill-shaped container with slot for inputs + integrated scan button

## Modal
- **Component:** `Modal.vue`
- **Features:** Overlay, ESC dismiss, icon/default/actions slots

## ToggleSwitch
- **Component:** `ToggleSwitch.vue`
- **Style:** macOS-style toggle with disabled + focus-visible support
- **Moved from:** Settings.vue scoped CSS

## Spinner Fix
- **Correct pattern:** `class="spinner-sm"` inside buttons (not `class="spinner spinner-sm"`)
- **Fixed in:** All 8 affected views

## Expand Chevron
- **Use global `.expand-chevron` everywhere**
- **Fixed:** Caches `.category-chevron`, Logs `.category-chevron`, Thermal `.cat-chevron` all replaced

## Summary Bar
- **Use global `.summary-bar` everywhere**
- **Fixed:** Logs `.results-bar`, LargeFiles `.results-summary`, Security `.summary-row` all replaced

## FDA Warning
- **Current:** Still using `FdaWarningBanner.vue` in 7 views
- **Deferred:** Sidebar migration is future work (not part of cohesion audit scope)

## Feedback / Toast System
- **Current:** Toast.vue exists but only used by Vault
- **Deferred:** Bottom-right fixed positioning and app-wide adoption is future work
