# Negativ_ — Claude Context

## Project
macOS system cleaner and disk visualiser built with Tauri v2 (Rust backend + Vue 3/TypeScript frontend).
~37,000 lines of code across ~60 source files. Early alpha, distributed via Homebrew tap.

## Key facts
- **Correct project root:** `~/projects/negativ_` — always build from here, never from `~/projects/macsweep` (old name)
- **Binary name:** `negative-space` (Rust crate name), installed as `Negativ_.app`
- **Bundle ID:** `com.conradfe.negativespace`
- **Git identity:** `conrad.feyt@gmail.com` / Conrad Feyt (local config only, not global)
- **GitHub:** `github.com/conradfeyt/negative_space`
- **Homebrew tap:** `conradfeyt/homebrew-negativespace` → `brew tap conradfeyt/negativespace`

## Build workflow
```bash
# PREFERRED: Use the rebuild script (bumps build number, kills app, full build, opens from dist)
./rebuild.sh

# Manual full bundle:
cd ~/projects/negativ_ && npm run tauri build
open src-tauri/target/release/bundle/macos/Negativ_.app
```

**IMPORTANT:** Always open the built app from `src-tauri/target/release/bundle/macos/Negativ_.app`, never from `/Applications`. The `/Applications` copy can be stale.

**Build number:** `src/buildNumber.ts` contains an incrementing build number displayed in the sidebar footer (`v0.1.0 (build N)`). The `rebuild.sh` script auto-increments this. Use it to verify you're running the correct build.

**Kill before rebuilding:** `pkill -x "Negativ_"` — macOS won't replace a running binary.

## Stack
- **Frontend:** Vue 3 + TypeScript + Vite + D3.js + Vitest (`src/`)
- **Backend:** Rust + Tauri v2 (`src-tauri/src/`)
- **Native:** AppKit via objc2 — custom gradient layer, SMC sensor reading
- **Image processing:** `image` crate (0.23) + `img_hash` (3.2) for perceptual hashing
- **Testing:** Vitest (37 unit tests for pure utilities), Rust `#[cfg(test)]` modules
- **Key views:** Dashboard, LargeFiles, Caches, Logs, Docker, Apps, Trash, Browsers, Duplicates, Vault, Archive, SpaceMap, Thermal, Memory, Vitals, Packages, Security, Maintenance, SensitiveContent

## Architecture notes

### Rust backend module structure
`src-tauri/src/` is organized into domain modules:
- **`lib.rs`** — module declarations, Tauri app builder, command registration, shared commands (disk usage, trash, FDA)
- **`commands.rs`** — shared types (`FileInfo`, `LargeFileScanResult`), utility functions (`home_dir`, `get_du_size`, `run_cmd`, `build_skip_prefixes`, `build_scan_roots`, `format_system_time`)
- **`large_files.rs`** — streaming large file scanner, supports multi-directory via `scan_paths`
- **`caches_logs.rs`** — cache and log scanning
- **`docker.rs`** — Docker detection, info, and cleanup
- **`apps.rs`** — app scanning, uninstall, leftover detection
- **`duplicates.rs`** — 3-stage duplicate file detection (size → partial hash → full hash), supports multi-directory via `scan_paths`
- **`similar_images.rs`** — perceptual hash-based similar image clustering, supports multi-directory via `scan_paths`
- **`vault.rs`** — compressed file vault (zstd)
- **`security.rs`** — launch item auditing
- **`browser.rs`** — multi-browser cache/data scanning
- **`diskmap.rs`** — disk usage tree builder
- **`thermal.rs`** — SMC temperature/fan reading
- **`vitals.rs`** — CPU, memory, system vitals
- **`memory.rs`** — detailed memory process analysis
- **`packages.rs`** — package manager inventory
- **`maintenance.rs`** — system maintenance tasks
- **`process_info.rs`** — shared process dictionary and app bundle mappings (canonical source for memory.rs and vitals.rs)
- **`gradient.rs`** — native NSImageView gradient layer
- **`image_utils.rs`** — shared image loading, HEIC fallback, thumbnails
- **`intelligence.rs`** — Apple Intelligence integration
- **`spotlight.rs`** — vault Spotlight indexing
- **`preview.rs`** — Quick Look thumbnail generation
- **`nsfw.rs`** — dual-model NSFW image detection (OpenNSFW2 + NudeNet), EXIF date extraction, multi-phase progress events

### Frontend store structure
`src/stores/` uses domain-specific stores with a re-export facade:
- **`scanStore.ts`** — thin facade re-exporting all domain stores + `scanAll()` orchestrator
- **`domainStatusStore.ts`** — shared: `domainStatus`, `setDomain()`, `hasFullDiskAccess`, `deleteFiles`, cache persistence
- **`largeFilesStore.ts`**, **`cachesStore.ts`**, **`logsStore.ts`**, **`appsStore.ts`**, **`browserStore.ts`**, **`dockerStore.ts`**, **`trashStore.ts`**, **`securityStore.ts`**, **`maintenanceStore.ts`**, **`packagesStore.ts`** — per-domain scan state + actions
- **`duplicatesStore.ts`** — exact duplicates + similar images
- **`diskMapStore.ts`** — disk map + caching + enrichment
- **`systemStore.ts`** — vitals, thermal, memory (live polling)
- **`vaultStore.ts`** — vault operations
- **`diskUsageStore.ts`** — disk usage stats
- **`intelligenceStore.ts`** — AI classification
- **`nsfwStore.ts`** — NSFW scan state, label exclusions, per-label weights, exposed label constants
- **`archiveStore.ts`** — compressed file archive operations (zstd compression, restoration, inventory)

All views import from `scanStore.ts` (the facade) — no view changes needed when stores are reorganized internally.

### Shared Vue components

**Shared UI components (extracted during component cohesion audit):**
- **`Checkbox.vue`** — Custom styled checkbox with animated check mark, indeterminate support, v-model
- **`StatCard.vue`** — Small metric/stat card (value + label), used by Apps, Packages, Duplicates, Security, Vault
- **`EmptyState.vue`** — Rich empty state with customizable icon slot, title, description
- **`TabBar.vue`** — Unified segmented control / tab bar with pill styling, disabled items, badges, scoped slot
- **`LiveIndicator.vue`** — Pulsing pill badge showing live/paused state (Cpu, SystemVitals, Memory, Thermal)
- **`ToggleSwitch.vue`** — macOS-style toggle switch with disabled + focus-visible support
- **`ScanBar.vue`** — Pill-shaped scan controls container with slot for inputs + integrated scan button
- **`Modal.vue`** — Reusable modal dialog with overlay, ESC dismiss, icon/default/actions slots
- **`FileRow.vue`** — File list row (icon, name, path, safety badge, size, reveal button, checkbox)
- **`AppSelect.vue`** — Custom div-based dropdown replacing native `<select>`, with icon support, glassmorphism styling, compact mode for ScanBar
- **`FilterPill.vue`** — Filter icon button for toggling filter states (used in Duplicates + Sensitive Content)
- **`ScanHeader.vue`** — Shared scan view header with title, stat cards, scan button, folder picker, and settings popover slot. Used by Duplicates, LargeFiles, SensitiveContent
- **`KindFilterBar.vue`** — File kind filter pills (All, Images, Documents, Audio, Video, Archives, Code, Other)
- **`StickyBar.vue`** — Sticky toolbar that pins to the top of scrollable content, with selection controls and action buttons
- **`LoadingState.vue`** — Lightweight loading placeholder with spinner and message
- **`ProgressBar.vue`** — Animated progress bar with percentage display
- **`InlineAlert.vue`** — Contextual alert banner (info, warning, success, error variants) with icon and dismiss
- **`CollapsibleSection.vue`** — Expandable/collapsible section with animated chevron and header slot
- **`ConfirmPanel.vue`** — Confirmation panel with summary, action buttons, and cancel — used for destructive batch operations
- **`DiskUsageBar.vue`** — Segmented horizontal bar showing disk usage breakdown by category
- **`MetricStrip.vue`** — Horizontal strip of key-value metric pairs, used in scan result summaries
- **`ViewHeader.vue`** — Simple view title header with optional subtitle and action slot

**Existing components (unchanged):**
- **`ThermalCard.vue`**, **`FanCard.vue`**, **`BatteryCard.vue`**, **`CpuCard.vue`**, **`MemoryCard.vue`** — health card components
- **`Toast.vue`** — notification toasts
- **`ChipSchematic.vue`** — Apple Silicon thermal overlay
- **`VoronoiViz.vue`**, **`GalacticViz.vue`** — SpaceMap visualizations
- **`NsfwImageCard.vue`** — sensitive content card with blurred preview, confidence badge, NudeNet label tags, info popover trigger
- **`TimelineRail.vue`** — hierarchical date scrubber (years → months) with dock-style fish-eye scaling, proportional scroll tracking

### Shared composables
- **`useZoomPan.ts`** — drag-state-machine + wheel-zoom (used by VoronoiViz and GalacticViz)
- **`useScanSettings.ts`** — scan area configuration with localStorage persistence
- **`useScreenGradient.ts`** — Screen-anchored gradient rendering (blob geometry, bitmap painting, position polling, monitor topology, custom JS drag). Extracted from App.vue (~370 lines).
- **`useScanFolder.ts`** — multi-directory scan path management. Exports `useScanFolder` (single path picker), `useScanLocations` (multi-path picker with up to 10 locations), `iconForPath` (native SF Symbol icons for known directories), `displayForPath` (friendly names). localStorage persistence. Used by LargeFiles, Duplicates, SensitiveContent.
- **`useDuplicateFilters.ts`** — file kind filtering for duplicate groups (All, Images, Documents, Audio, Video, Archives, Code, Other). Extracted from Duplicates.vue.
- **`useFileGrouping.ts`** — groups, sorts, and categorizes FileInfo[] by size, directory, safety, or type. Extracted from LargeFiles.vue.
- **`useCompressionQueue.ts`** — compression queue management for the Archive view. Handles staging, size calculation, and sequential zstd compression.

### Shared utilities (`src/utils.ts`)
- `formatSize`, `fileDiskSize`, `timeAgo`, `tempToColor`, `revealInFinder`
- Temperature threshold constants (`TEMP_CRITICAL/HOT/WARM/COOL`)
- Shared color maps (`KIND_COLORS`, `MEMORY_CATEGORY_COLORS`, `SPACEMAP_CATEGORY_FILLS`, `DASHBOARD_CATEGORY_COLORS`)
- `cssVar()` helper for reading CSS custom properties
- `cpuLoadColor`, `cpuLoadClass`, `memoryPressureLevel`, `memoryPressureDotClass`, `fanSpeedColor`, `fanSpeedZone`, `storageColor` — centralized health thresholds (single source of truth for dashboard cards and detail views)

### Background gradient
Two-layer system, managed by `src/composables/useScreenGradient.ts`:
1. **CSS layer** (content panel) — warm palette JPEG generated on frontend via canvas, set as `--gradient-bg` CSS var on `#app`. Uses `RENDER_SCALE=0.20`, `BLOB_HOLD=0.35`, `saturate(6.0)`.
2. **Native layer** (sidebar) — cool palette JPEG sent to Rust via `invoke('set_native_background', ...)`, rendered as `NSImageView` behind WKWebView.

The content panel has a 70% white overlay for readability. The sidebar + gutters have a 5% white wash overlay.

### Visualisations (SpaceMap)
Three modes switchable in-app:
- **Voronoi treemap** (`src/components/VoronoiViz.vue`) — d3-voronoi-treemap, white-opacity fills, dark gap mask via SVG
- **Sunburst** (`src/views/SpaceMap.vue`) — D3 partition, white-opacity fills matching Voronoi aesthetic
- **Galactic** (`src/components/GalacticViz.vue`) — star-field visualization

### Vault
Compressed file storage at `~/Documents/MyNegativeSpaceVault/`. Previously was at `~/Library/Application Support/MacSweep/vault` and `NegativeSpace/vault` — migration code exists in `vault.rs`.

### Similar image detection
`src-tauri/src/similar_images.rs` — perceptual hash-based (dHash 16x16 via `img_hash` crate). Deduplicates exact copies (BLAKE3 partial hash) before clustering by Hamming distance. Thumbnails generated during scan via `sips`. Results cached to disk.

### Image utilities
`src-tauri/src/image_utils.rs` — shared image loading (`image` crate), HEIC fallback via `sips`, thumbnail generation via `sips --resampleHeightWidthMax`.

### Duplicate finder thumbnails
Thumbnails are generated during the duplicate scan (one per group, since all files are byte-identical) and embedded as base64 JPEG in the scan result. This is a known performance issue — see `_private/PERFORMANCE_ROADMAP.md` item 4.3 for the planned file-based cache approach.

### Native icon system (Swift bridge)
`src-tauri/swift/Sources/Bridge.swift` `render_sf_symbol` supports multiple modes:
- `mode: "sf"` — SF Symbols by name (e.g., `"folder"`)
- `mode: "uttype"` — NSWorkspace icon for UTType identifier or file extension
- `mode: "app"` — NSWorkspace icon for app bundle path
- `mode: "file"` — NSImage from file path
- `mode: "system"` — NSImage named system image (e.g., `"NSApplicationIcon"`)

Styles: `plain` (preserves aspect ratio), `grayBadge` (white glyph on grey rounded rect), `grayBadgeHier` (hierarchical), `blueBadge` (blue symbol on white), `blueGradientBadge` (#47A8FF→#0690FF gradient, white glyph), `grayscaleApp`.

### Sensitive content detection (NSFW)
Dual-model on-device pipeline — fully private, no network calls.

**Models:**
- **OpenNSFW2** — CoreML classification model. Returns a single NSFW probability score (0.0–1.0). Fast, good recall.
- **NudeNet v3 (320n)** — CoreML object detection model (YOLO-based). Returns per-region labels with confidence (e.g., `FEMALE_BREAST_EXPOSED 0.87`). Provides richer information but slower.

**Pipeline (`nsfw.rs`):**
1. **Discovery** — `walkdir` traversal, filters by image extensions + minimum size.
2. **Classification** — Both models always run on all images. OpenNSFW2 detects, NudeNet enriches.
3. **Merge** — Union of positives from both models. Every flagged image gets NudeNet labels for enrichment.
4. **Synthetic label** — OpenNSFW2 score injected as `NSFW_SCORE` (displayed as "General") into `detected_labels`, making it a filterable dimension alongside NudeNet body-part labels.
5. **Thumbnails** — Generated via `sips` during scan, embedded as base64 in results.
6. **EXIF dates** — Extracted via `kamadak-exif` (JPEG/TIFF) or `mdls` (HEIC), used for timeline grouping.

**Swift bridge (`Bridge.swift`):**
- `msw_classify_nsfw` — OpenNSFW2 inference via `VNCoreMLRequest`.
- `msw_detect_nsfw` — NudeNet inference. Parses raw YOLO tensor `[1, 22, 2100]`, applies score threshold + greedy NMS. Returns JSON with per-detection labels + confidence. Uses safe `MLMultiArray` subscript access (not raw pointers) to handle Float16/Float32.

**Frontend (`SensitiveContent.vue`):**
- Date-grouped results with adaptive granularity (sparse years collapse, dense months expand to days).
- Hierarchical `TimelineRail` scrubber with fish-eye scaling.
- Shift-click multi-select, batch delete/vault/move operations with progress bars.
- Blurred image previews with "Show content" toggle.
- Label filter popover (draggable, clamped to window bounds) with per-label toggle + weight slider (0–200%).
- Filter dims non-passing images (never hides — all flagged images remain visible and interactive).
- Info popover shows all NudeNet labels with raw and weighted confidence bars.

**Exposed labels (filterable):** `NSFW_SCORE` (General), `FEMALE_BREAST_EXPOSED`, `BUTTOCKS_EXPOSED`, `FEMALE_GENITALIA_EXPOSED`, `MALE_GENITALIA_EXPOSED`, `ANUS_EXPOSED`, `MALE_BREAST_EXPOSED`, `BELLY_EXPOSED`, `ARMPITS_EXPOSED`.

**Persistence (localStorage):** `negativ_nsfw_sensitivity`, `negativ_nsfw_min_size`, `negativ_nsfw_excluded_labels`, `negativ_nsfw_label_weights`, `negativ_sensitive_move_target`.

**Bundled model:** `NudeNet320n.mlmodelc` in `tauri.conf.json` resources. OpenNSFW2 model at `src-tauri/resources/OpenNSFW2.mlmodelc`.

### Native gradient layer
`src-tauri/src/gradient.rs` — receives JPEG from frontend, creates `NSImageView` behind WKWebView. Do NOT replace with CSS — it's there for zero-lag window drag tracking.

## Design system
- Glassmorphism — translucent cards (`var(--glass)` = `rgba(255,255,255,0.45)`) over gradient background
- Dark text on light frosted glass content panel (70% white overlay)
- Sidebar: white text on native gradient + 5% white wash overlay
- **Accent:** `#0088FF` blue (was aqua `#3BC7E8`)
- **Color tokens:** Named-color-first system — `--blue`, `--green`, `--yellow`, `--red`, `--orange`, `--purple`, `--teal`, `--cyan`, `--slate`, `--grey`, `--pink` with semantic aliases (`--accent`, `--success`, `--warning`, `--danger`, `--info`)
- **Badges:** Border-based with `text-transform: uppercase`. Modifiers: `.pill` (rounded), `.source` (no border, tinted bg). Classes: `.badge-accent/.badge-success/.badge-warning/.badge-danger/.badge-info/.badge-neutral`
- **Buttons:** `.btn-primary` (solid blue), `.btn-secondary` (grey fill, no border), `.btn-danger` (solid red), `.btn-ghost` (text only), `.btn-ghost.danger` (red text)
- **Form controls:** Custom Checkbox, ToggleSwitch, styled select (appearance:none + custom chevron), slider, radio buttons
- **Design tokens:** `src/style.css` `:root` block with 100+ CSS custom properties. JS color maps in `utils.ts` read tokens via `cssVar()`.
- **Component showcase:** `src/views/Showcase.vue` + `showcase.html` — all components and tokens visible for tuning (dev-only route at `/showcase`)
- **Accessibility:** All `<img>` have alt text, clickable divs have keyboard access + ARIA, tabs/expandable sections/modals have proper roles

## Distribution
- Homebrew cask at `~/projects/homebrew-negativespace/Casks/negativ_.rb`
- Cask handles quarantine removal and auto-launch in `postflight`
- Current release: v0.1.0-alpha on GitHub Releases

## Private docs
`_private/` (gitignored):
- `HANDOVER.md` — comprehensive project handover with architecture, design decisions, caveats
- `ROADMAP.md` — feature roadmap with completed and planned work
- `PERFORMANCE_ROADMAP.md` — 15-item performance optimisation plan with metrics and priorities

## Git policy
- **Never** run `git commit`, `git push`, `git merge`, `git rebase`, or any command that modifies git history
- May run `git add` to stage files and read-only commands (`status`, `diff`, `log`, `branch`, `show`)
- When asked to commit: stage files and suggest a commit message — do not execute the commit

## What NOT to do
- Don't use git worktrees — causes complications with orphan branches and identity
- Don't build from `~/projects/macsweep` — old project, WebKit cache there is stale
- Don't change `gradient.rs` native layer to CSS
- Don't skip `touch lib.rs` before `--no-bundle` builds
- Don't use `--global` for git config — personal identity is local only
- Don't open app from `/Applications` — always open from `src-tauri/target/release/bundle/macos/`
- Don't generate thumbnails on the frontend — generate during Rust scan and include in results
- Don't render all cards in duplicate groups — cap at 10 with overflow indicator (DOM performance)
