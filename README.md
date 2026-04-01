# Negativ_

![Status](https://img.shields.io/badge/status-early%20alpha-orange)

> ⚠️ **Early alpha** — expect rough edges and missing features. Not recommended for critical use.

A lightweight macOS system cleaner and monitor built with Tauri v2.

## Features

- **Disk cleanup** — scan and remove caches, logs, large files, duplicates, trash, and browser data
- **Package manager cleanup** — Homebrew, npm, pip, Cargo
- **Docker cleanup** — unused images, containers, and reclaimable space
- **App management** — detect installed apps and leftover files
- **System monitoring** — CPU thermals, memory usage, disk space mapping
- **Apple Silicon visualization** — live thermal overlay on a chip die schematic
- **Security auditing** — launch agent inspection, app code signing verification
- **Maintenance tasks** — flush DNS, rebuild Spotlight, purge purgeable space

## Installation

### Homebrew (recommended)

```bash
brew tap conradfeyt/negativespace
brew install --cask negativ_
```

> **First launch:** macOS will block the app as it's unsigned. Right-click `Negativ_.app` in `/Applications` and select **Open** to bypass Gatekeeper.

### Uninstall

```bash
brew uninstall --cask negativ_
```

To also remove all app data and your vault:

```bash
brew uninstall --cask --zap negativ_
```

### Direct download

Download the latest `.dmg` from [Releases](https://github.com/conradfeyt/negative_space/releases).

---

## Stack

- **Frontend:** Vue 3 + TypeScript + Vite + D3.js
- **Backend:** Rust (Tauri v2)
- **Native integration:** AppKit via objc2 (custom gradient layers, SMC sensor reading)

## Requirements

- macOS (Apple Silicon or Intel)
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- Tauri v2 CLI: `cargo install tauri-cli --version "^2"`

Full Disk Access is recommended for thorough scanning (the app will prompt on first launch).

## Development

```bash
# Install frontend dependencies
npm install

# Run in dev mode (hot-reload frontend + Rust backend)
npm run tauri dev
```

## Building

```bash
npm run tauri build
```

The `.dmg` / `.app` output lands in `src-tauri/target/release/bundle/`.

## Project Structure

```
src/                    # Vue 3 frontend
  views/                # 17 view components (Dashboard, Caches, Logs, etc.)
  components/           # Visualizations (ChipSchematic, VoronoiViz, GalacticViz)
  stores/               # Global scan state
  composables/          # Scan settings
src-tauri/src/          # Rust backend
  lib.rs                # Tauri app + 40+ commands
  commands.rs           # Shared data structures
  browser.rs            # Browser data detection & cleanup
  duplicates.rs         # 3-stage duplicate detection (BLAKE3)
  gradient.rs           # Native NSImageView gradient layer
  maintenance.rs        # System maintenance tasks
  memory.rs             # Process analysis
  packages.rs           # Package manager detection
  security.rs           # Launch agent / code signing auditing
  thermal.rs            # SMC sensor reading
  vitals.rs             # Thermal monitoring + remediation
  diskmap.rs            # Directory tree for treemap viz
  preview.rs            # File preview generation
```

## IDE Setup

[VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
