# Negativ_ — Claude Context

## Project
macOS system cleaner and disk visualiser built with Tauri v2 (Rust backend + Vue 3/TypeScript frontend).
~25,000 lines of code. Early alpha, distributed via Homebrew tap.

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
- **Frontend:** Vue 3 + TypeScript + Vite + D3.js (`src/`)
- **Backend:** Rust + Tauri v2 (`src-tauri/src/`)
- **Native:** AppKit via objc2 — custom gradient layer, SMC sensor reading
- **Image processing:** `image` crate (0.23) + `img_hash` (3.2) for perceptual hashing
- **Key views:** Dashboard, LargeFiles, Caches, Logs, Docker, Apps, Trash, Browsers, Duplicates, Vault, SpaceMap, Thermal, Memory, Vitals, Packages, Security, Maintenance

## Architecture notes

### Background gradient
Two-layer system:
1. **CSS layer** (content panel) — warm palette JPEG generated on frontend via canvas, set as `--gradient-bg` CSS var on `#app`. Uses `RENDER_SCALE=0.20` (render at 20% then upscale for smooth blending), `BLOB_HOLD=0.35`, `saturate(6.0)`. Lives in `src/App.vue`.
2. **Native layer** (sidebar) — cool palette JPEG sent to Rust via `invoke('set_native_background', ...)`, rendered as `NSImageView` behind WKWebView. Cannot be changed via CSS.

The content panel has a 70% white overlay (`rgba(255,255,255,0.70)`) for readability — this is intentional and should not be changed.

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
- `mode: "uttype"` — NSWorkspace icon for UTType identifier (e.g., `"public.folder"`) or file extension (e.g., `"png"`)
- `mode: "app"` — NSWorkspace icon for app bundle path (e.g., `"/Applications/Google Chrome.app"`)
- `mode: "file"` — NSImage from file path
- `mode: "system"` — NSImage named system image

### Native gradient layer
`src-tauri/src/gradient.rs` — receives JPEG from frontend, creates `NSImageView` behind WKWebView. Do NOT replace with CSS — it's there for zero-lag window drag tracking.

## Design system
- Glassmorphism — translucent cards over gradient background
- Dark text on light frosted glass content panel
- Sidebar: white text on native gradient (no backdrop-filter)
- Accent: `rgba(59, 199, 232, ...)` aqua/teal
- All opacity-based — white-on-dark for sidebar, dark-on-light for content

## Distribution
- Homebrew cask at `~/projects/homebrew-negativespace/Casks/negativ_.rb`
- Cask handles quarantine removal and auto-launch in `postflight`
- Current release: v0.1.0-alpha on GitHub Releases

## Private docs
`_private/` (gitignored):
- `HANDOVER.md` — comprehensive project handover with architecture, design decisions, caveats
- `ROADMAP.md` — feature roadmap with completed and planned work
- `PERFORMANCE_ROADMAP.md` — 15-item performance optimisation plan with metrics and priorities

## What NOT to do
- Don't use git worktrees — causes complications with orphan branches and identity
- Don't build from `~/projects/macsweep` — old project, WebKit cache there is stale
- Don't change `gradient.rs` native layer to CSS
- Don't skip `touch lib.rs` before `--no-bundle` builds
- Don't use `--global` for git config — personal identity is local only
- Don't open app from `/Applications` — always open from `src-tauri/target/release/bundle/macos/`
- Don't generate thumbnails on the frontend — generate during Rust scan and include in results
- Don't render all cards in duplicate groups — cap at 10 with overflow indicator (DOM performance)
