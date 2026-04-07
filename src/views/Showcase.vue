<script setup lang="ts">
/**
 * Showcase — Component widgetbook for visual tuning.
 * DEV only. Shows all shared components and style variants side by side.
 */
import StatCard from "../components/StatCard.vue";
import EmptyState from "../components/EmptyState.vue";
import TabBar from "../components/TabBar.vue";
import LiveIndicator from "../components/LiveIndicator.vue";
import ToggleSwitch from "../components/ToggleSwitch.vue";
import Checkbox from "../components/Checkbox.vue";
import type { TabOption } from "../components/TabBar.vue";
import { ref } from "vue";

const segValue = ref("size");
const segOptions: TabOption[] = [
  { value: "size", label: "Size" },
  { value: "dir", label: "Directory" },
  { value: "safety", label: "Safety" },
  { value: "type", label: "Type" },
];

const pillValue = ref("all");
const pillOptions: TabOption[] = [
  { value: "all", label: "All" },
  { value: "images", label: "Images" },
  { value: "docs", label: "Documents" },
  { value: "audio", label: "Audio" },
  { value: "disabled", label: "Disabled", disabled: true },
];

const tabValue = ref("tab1");
const tabOptions: TabOption[] = [
  { value: "tab1", label: "Exact Duplicates" },
  { value: "tab2", label: "Similar Images", badge: 13 },
];

const toggleValue = ref(true);

const sliderValue = ref(50);
const radioValue = ref("auto");
const radioScroll = ref("next-page");
const toggleOff = ref(false);
</script>

<template>
  <div class="showcase">
    <div class="view-header">
      <h2>Component Showcase</h2>
      <p class="text-muted">All shared components and style variants for visual tuning</p>
    </div>

    <!-- ================================================================= -->
    <!-- COLOR PALETTE -->
    <!-- ================================================================= -->
    <section class="showcase-section">
      <h3 class="section-title">Color Palette</h3>

      <h4 class="showcase-subtitle">Blue</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--blue)"></div><div class="swatch-info"><span class="swatch-name">--blue</span><span class="swatch-value">#0088FF</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--blue-hover)"></div><div class="swatch-info"><span class="swatch-name">--blue-hover</span><span class="swatch-value">#0391FF</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--blue-deep)"></div><div class="swatch-info"><span class="swatch-name">--blue-deep</span><span class="swatch-value">#1D6FF4</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--blue-light)"></div><div class="swatch-info"><span class="swatch-name">--blue-light</span><span class="swatch-value">rgba(2,117,244,0.10)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--blue-glow)"></div><div class="swatch-info"><span class="swatch-name">--blue-glow</span><span class="swatch-value">rgba(2,117,244,0.18)</span></div></div>
      </div>

      <h4 class="showcase-subtitle">Green</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--green)"></div><div class="swatch-info"><span class="swatch-name">--green</span><span class="swatch-value">#038933</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--green-light)"></div><div class="swatch-info"><span class="swatch-name">--green-light</span><span class="swatch-value">#34C759</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--green-tint)"></div><div class="swatch-info"><span class="swatch-name">--green-tint</span><span class="swatch-value">#4BD968</span></div></div>
      </div>

      <h4 class="showcase-subtitle">Yellow</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--yellow)"></div><div class="swatch-info"><span class="swatch-name">--yellow</span><span class="swatch-value">#FFCC02</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--yellow-tint)"></div><div class="swatch-info"><span class="swatch-name">--yellow-tint</span><span class="swatch-value">#FEDF43</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--yellow-text)"></div><div class="swatch-info"><span class="swatch-name">--yellow-text</span><span class="swatch-value">#A16B00</span></div></div>
      </div>

      <h4 class="showcase-subtitle">Red</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--red)"></div><div class="swatch-info"><span class="swatch-name">--red</span><span class="swatch-value">#FF393C</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--red-tint)"></div><div class="swatch-info"><span class="swatch-name">--red-tint</span><span class="swatch-value">#FF6165</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--red-hover)"></div><div class="swatch-info"><span class="swatch-name">--red-hover</span><span class="swatch-value">#E9152D</span></div></div>
      </div>

      <h4 class="showcase-subtitle">Orange</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--orange)"></div><div class="swatch-info"><span class="swatch-name">--orange</span><span class="swatch-value">#FF8D28</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--orange-dark)"></div><div class="swatch-info"><span class="swatch-name">--orange-dark</span><span class="swatch-value">#C55301</span></div></div>
      </div>

      <h4 class="showcase-subtitle">Purple</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--purple)"></div><div class="swatch-info"><span class="swatch-name">--purple</span><span class="swatch-value">#B02FC2</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--purple-light)"></div><div class="swatch-info"><span class="swatch-name">--purple-light</span><span class="swatch-value">#EA8EFF</span></div></div>
      </div>

      <h4 class="showcase-subtitle">Teal / Cyan</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--teal)"></div><div class="swatch-info"><span class="swatch-name">--teal</span><span class="swatch-value">#00C8B3</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--teal-deep)"></div><div class="swatch-info"><span class="swatch-name">--teal-deep</span><span class="swatch-value">#028199</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--teal-dark)"></div><div class="swatch-info"><span class="swatch-name">--teal-dark</span><span class="swatch-value">#007EAE</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--cyan)"></div><div class="swatch-info"><span class="swatch-name">--cyan</span><span class="swatch-value">#07C0E8</span></div></div>
      </div>

      <h4 class="showcase-subtitle">Neutral</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--slate)"></div><div class="swatch-info"><span class="swatch-name">--slate</span><span class="swatch-value">#94a3b8</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--grey)"></div><div class="swatch-info"><span class="swatch-name">--grey</span><span class="swatch-value">#78909c</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--pink)"></div><div class="swatch-info"><span class="swatch-name">--pink</span><span class="swatch-value">#ad1457</span></div></div>
      </div>

      <h4 class="showcase-subtitle">Semantic Aliases (reference color tokens above)</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--accent)"></div><div class="swatch-info"><span class="swatch-name">--accent</span><span class="swatch-value">= --blue</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--success)"></div><div class="swatch-info"><span class="swatch-name">--success</span><span class="swatch-value">= --green-light</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--warning)"></div><div class="swatch-info"><span class="swatch-name">--warning</span><span class="swatch-value">= --yellow</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--danger)"></div><div class="swatch-info"><span class="swatch-name">--danger</span><span class="swatch-value">= --red</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--info-tint)"></div><div class="swatch-info"><span class="swatch-name">--info-tint</span><span class="swatch-value">= --teal</span></div></div>
      </div>

      <h4 class="showcase-subtitle">Text &amp; Surface</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--text)"></div><div class="swatch-info"><span class="swatch-name">--text</span><span class="swatch-value">#1A1D2E</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--text-secondary)"></div><div class="swatch-info"><span class="swatch-name">--text-secondary</span><span class="swatch-value">#5C6070</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--muted)"></div><div class="swatch-info"><span class="swatch-name">--muted</span><span class="swatch-value">#8E92A4</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--glass)"></div><div class="swatch-info"><span class="swatch-name">--glass</span><span class="swatch-value">rgba(255,255,255,0.45)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--glass-border)"></div><div class="swatch-info"><span class="swatch-name">--glass-border</span><span class="swatch-value">rgba(255,255,255,0.55)</span></div></div>
      </div>

      <h4 class="showcase-subtitle">Thermal Scale</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--temp-cool)"></div><div class="swatch-info"><span class="swatch-name">--temp-cool</span><span class="swatch-value">#007EAE</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--temp-warm)"></div><div class="swatch-info"><span class="swatch-name">--temp-warm</span><span class="swatch-value">var(--purple)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--temp-serious)"></div><div class="swatch-info"><span class="swatch-name">--temp-serious</span><span class="swatch-value">var(--orange)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--temp-hot)"></div><div class="swatch-info"><span class="swatch-name">--temp-hot</span><span class="swatch-value">var(--danger-tint)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--temp-critical)"></div><div class="swatch-info"><span class="swatch-name">--temp-critical</span><span class="swatch-value">var(--danger-hover)</span></div></div>
      </div>

      <h4 class="showcase-subtitle">Memory Categories</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--cyan)"></div><div class="swatch-info"><span class="swatch-name">App</span><span class="swatch-value">var(--cyan)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--orange)"></div><div class="swatch-info"><span class="swatch-name">Wired</span><span class="swatch-value">var(--orange)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--purple-light)"></div><div class="swatch-info"><span class="swatch-name">Compressed</span><span class="swatch-value">var(--purple-light)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--slate)"></div><div class="swatch-info"><span class="swatch-name">Inactive</span><span class="swatch-value">var(--slate)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--success)"></div><div class="swatch-info"><span class="swatch-name">Free</span><span class="swatch-value">var(--success)</span></div></div>
      </div>

      <h4 class="showcase-subtitle">SpaceMap Categories</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--cyan)"></div><div class="swatch-info"><span class="swatch-name">Applications</span><span class="swatch-value">var(--cyan)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--success)"></div><div class="swatch-info"><span class="swatch-name">Documents</span><span class="swatch-value">var(--success)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--warning)"></div><div class="swatch-info"><span class="swatch-name">Developer</span><span class="swatch-value">var(--warning)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--orange)"></div><div class="swatch-info"><span class="swatch-name">Books</span><span class="swatch-value">var(--orange)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--accent-hover)"></div><div class="swatch-info"><span class="swatch-name">iCloud</span><span class="swatch-value">var(--accent-hover)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--temp-cool)"></div><div class="swatch-info"><span class="swatch-name">Mail</span><span class="swatch-value">var(--temp-cool)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--pink)"></div><div class="swatch-info"><span class="swatch-name">Photos</span><span class="swatch-value">var(--pink)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--purple-light)"></div><div class="swatch-info"><span class="swatch-name">Media</span><span class="swatch-value">var(--purple-light)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--purple)"></div><div class="swatch-info"><span class="swatch-name">Podcasts</span><span class="swatch-value">var(--purple)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--info-tint)"></div><div class="swatch-info"><span class="swatch-name">Docker</span><span class="swatch-value">var(--info-tint)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--warning-text)"></div><div class="swatch-info"><span class="swatch-name">Caches</span><span class="swatch-value">var(--warning-text)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--danger-tint)"></div><div class="swatch-info"><span class="swatch-name">macOS</span><span class="swatch-value">var(--danger-tint)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--grey)"></div><div class="swatch-info"><span class="swatch-name">Other</span><span class="swatch-value">var(--grey)</span></div></div>
      </div>

      <h4 class="showcase-subtitle">Source Badges</h4>
      <div class="palette-grid">
        <div class="swatch"><div class="swatch-color" style="background:var(--orange-dark)"></div><div class="swatch-info"><span class="swatch-name">Homebrew</span><span class="swatch-value">var(--orange-dark)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--blue-deep)"></div><div class="swatch-info"><span class="swatch-name">App Store</span><span class="swatch-value">var(--blue-deep)</span></div></div>
        <div class="swatch"><div class="swatch-color" style="background:var(--green)"></div><div class="swatch-info"><span class="swatch-name">NVM / Success</span><span class="swatch-value">var(--green)</span></div></div>
      </div>
    </section>

    <!-- ================================================================= -->
    <!-- BADGES -->
    <!-- ================================================================= -->
    <section class="showcase-section">
      <h3 class="section-title">Badges (.badge)</h3>
      <div class="showcase-row">
        <span class="badge badge-accent">Accent</span>
        <span class="badge badge-success">Success</span>
        <span class="badge badge-warning">Warning</span>
        <span class="badge badge-danger">Danger</span>
        <span class="badge badge-info">Info</span>
        <span class="badge badge-neutral">Neutral</span>
      </div>

      <h4 class="showcase-subtitle">In context: Security findings</h4>
      <div class="showcase-row">
        <span class="badge badge-neutral">10 Total</span>
        <span class="badge badge-danger">2 Malicious</span>
        <span class="badge badge-warning">3 Suspicious</span>
        <span class="badge badge-info">5 Informational</span>
      </div>

      <h4 class="showcase-subtitle">In context: Maintenance task</h4>
      <div class="showcase-row">
        <span class="badge pill badge-warning">Admin</span>
        <span class="badge pill badge-success">No data changed</span>
        <span class="badge pill badge-accent">Reversible</span>
      </div>
      <div class="showcase-row">
        <span class="badge pill badge-danger">Modifies data</span>
        <span class="badge pill badge-neutral">Not reversible</span>
      </div>

      <h4 class="showcase-subtitle">In context: File safety</h4>
      <div class="showcase-row">
        <span class="badge badge-success">Likely safe</span>
        <span class="badge badge-accent">Safe — recently modified</span>
        <span class="badge badge-warning">Caution</span>
        <span class="badge badge-danger">Risky</span>
        <span class="badge badge-info">Safe to delete</span>
        <span class="badge badge-neutral">Unknown</span>
      </div>

      <h4 class="showcase-subtitle">Small pills (.badge .pill)</h4>
      <div class="showcase-row">
        <span class="badge pill badge-accent">active</span>
        <span class="badge pill badge-neutral">dep</span>
        <span class="badge pill badge-neutral">49</span>
        <span class="badge pill badge-accent">3 with findings</span>
        <span class="badge pill badge-neutral">7</span>
      </div>

      <h4 class="showcase-subtitle">Source badges (.badge .source .pill)</h4>
      <div class="showcase-row">
        <span class="badge source pill homebrew">Homebrew</span>
        <span class="badge source pill app-store">App Store</span>
        <span class="badge source pill nvm">NVM</span>
        <span class="badge source pill rustup">Rustup</span>
        <span class="badge source pill manual">Manual</span>
      </div>
    </section>

    <!-- ================================================================= -->
    <!-- STAT CARDS -->
    <!-- ================================================================= -->
    <section class="showcase-section">
      <h3 class="section-title">StatCard</h3>
      <div class="showcase-row" style="gap: 8px">
        <StatCard value="54" label="Apps Found" />
        <StatCard value="65.3 GB" label="Total Footprint" />
        <StatCard value="26.9 GB" label="Leftover Data" value-color="var(--warning)" />
        <StatCard value="3" label="Via App Store" />
      </div>

      <h4 class="showcase-subtitle">With highlight</h4>
      <div class="showcase-row" style="gap: 8px">
        <StatCard value="57.0 GB" label="Wasted Space" highlight />
        <StatCard value="200" label="Duplicate Groups" />
        <StatCard value="6933" label="Duplicate Files" />
      </div>

      <h4 class="showcase-subtitle">Security severity (color only, no borders)</h4>
      <div class="showcase-row" style="gap: 8px">
        <StatCard value="10" label="Total Findings" />
        <StatCard value="2" label="Malicious" value-color="var(--danger-text)" label-color="var(--danger-text)" />
        <StatCard value="0" label="Likely Unwanted" value-color="var(--warning-text)" label-color="var(--warning-text)" />
        <StatCard value="6" label="Suspicious" value-color="var(--warning-text)" label-color="var(--warning-text)" />
        <StatCard value="2" label="Informational" value-color="var(--info-text)" label-color="var(--info-text)" />
      </div>
    </section>

    <!-- ================================================================= -->
    <!-- BUTTONS -->
    <!-- ================================================================= -->
    <section class="showcase-section">
      <h3 class="section-title">Buttons</h3>

      <h4 class="showcase-subtitle">Primary</h4>
      <div class="showcase-row">
        <button class="btn-primary scan-btn">Scan</button>
        <button class="btn-primary scan-btn" disabled>Disabled</button>
        <button class="btn-primary btn-sm">Small Primary</button>
        <button class="btn-primary scan-btn"><span class="spinner-sm"></span> Scanning...</button>
      </div>

      <h4 class="showcase-subtitle">Secondary</h4>
      <div class="showcase-row">
        <button class="btn-secondary">Refresh</button>
        <button class="btn-secondary btn-sm">Select all</button>
        <button class="btn-secondary btn-sm">Cancel</button>
        <button class="btn-secondary btn-sm">Disable</button>
        <button class="btn-secondary">Select Safe</button>
        <button class="btn-secondary">Select duplicates</button>
      </div>

      <h4 class="showcase-subtitle">Danger</h4>
      <div class="showcase-row">
        <button class="btn-danger">Delete Selected</button>
        <button class="btn-danger">Clean Selected</button>
        <button class="btn-danger btn-sm">Remove</button>
        <button class="btn-danger btn-sm">Uninstall</button>
        <button class="btn-danger">Empty Trash</button>
      </div>

      <h4 class="showcase-subtitle">Ghost</h4>
      <div class="showcase-row">
        <button class="btn-ghost btn-sm">Pause</button>
        <button class="btn-ghost btn-sm">Resume</button>
        <button class="btn-ghost btn-sm">Clear</button>
        <button class="btn-ghost danger btn-sm">Delete</button>
        <button class="btn-ghost danger btn-sm">Remove</button>
      </div>

      <h4 class="showcase-subtitle">Inline card actions</h4>
      <div class="card-flush">
        <div class="list-item">
          <span class="list-item-name">com.epicgames.launcher</span>
          <span class="badge badge-info">Informational</span>
          <button class="btn-secondary btn-sm">Disable</button>
          <button class="btn-danger btn-sm">Remove</button>
        </div>
        <div class="list-item">
          <span class="list-item-name">Google Chrome</span>
          <span class="list-item-meta">416.2 MB</span>
          <button class="btn-secondary btn-sm">Select Safe</button>
        </div>
        <div class="list-item">
          <span class="list-item-name">macOS System</span>
          <span class="badge pill badge-neutral">8</span>
          <span class="list-item-meta">48%</span>
          <button class="btn-secondary btn-sm">Quit</button>
        </div>
        <div class="list-item">
          <span class="list-item-name">archive_2025.zip</span>
          <span class="badge badge-success">Compressed</span>
          <button class="btn-primary btn-sm">Restore</button>
          <button class="btn-ghost btn-sm">Delete</button>
        </div>
        <div class="list-item">
          <span class="list-item-name">Claude</span>
          <span class="list-item-meta">684.9 MB</span>
          <button class="btn-danger btn-sm">Uninstall</button>
        </div>
      </div>
    </section>

    <!-- ================================================================= -->
    <!-- FORM INPUTS -->
    <!-- ================================================================= -->
    <section class="showcase-section">
      <h3 class="section-title">Form Inputs</h3>

      <h4 class="showcase-subtitle">Text inputs</h4>
      <div class="showcase-row" style="gap:16px">
        <div class="input-group">
          <label class="input-label">Scan path</label>
          <input type="text" placeholder="~/Documents" style="width:200px">
        </div>
        <div class="input-group">
          <label class="input-label">Min size (MB)</label>
          <input type="number" value="10" min="1" max="10000" style="width:80px">
        </div>
        <div class="input-group">
          <label class="input-label">Disabled</label>
          <input type="text" value="Not editable" disabled style="width:160px">
        </div>
      </div>

      <h4 class="showcase-subtitle">Search field</h4>
      <div class="search-wrapper" style="max-width:400px">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><circle cx="6" cy="6" r="4.5"/><path d="M9.5 9.5L13 13"/></svg>
        <input type="search" placeholder="Filter apps..." style="width:100%">
      </div>

      <h4 class="showcase-subtitle">Select / dropdown</h4>
      <div class="showcase-row" style="gap:16px">
        <div class="input-group">
          <label class="input-label">Min size</label>
          <select>
            <option>1 KB</option>
            <option selected>1 MB</option>
            <option>5 MB</option>
            <option>10 MB</option>
            <option>50 MB</option>
            <option>100 MB</option>
          </select>
        </div>
        <div class="input-group">
          <label class="input-label">Similarity threshold</label>
          <select>
            <option>Strict (5)</option>
            <option selected>Normal (10)</option>
            <option>Loose (15)</option>
            <option>Very loose (20)</option>
          </select>
        </div>
      </div>

      <h4 class="showcase-subtitle">Sliders</h4>
      <div class="showcase-row" style="gap: 24px;">
        <div class="input-group" style="flex: 1; max-width: 300px;">
          <label class="input-label">Size ({{ sliderValue }})</label>
          <div class="slider-wrapper">
            <input type="range" v-model.number="sliderValue" min="0" max="100" class="slider"
              :style="{ '--fill': sliderValue + '%' }" />
            <div class="slider-labels">
              <span>Small</span>
              <span>Large</span>
            </div>
          </div>
        </div>
        <div class="input-group" style="flex: 1; max-width: 300px;">
          <label class="input-label">Min file size (MB)</label>
          <div class="slider-wrapper">
            <input type="range" v-model.number="sliderValue" min="1" max="500" step="1" class="slider"
              :style="{ '--fill': ((sliderValue / 500) * 100) + '%' }" />
            <div class="slider-labels">
              <span>1 MB</span>
              <span>500 MB</span>
            </div>
          </div>
        </div>
      </div>

      <h4 class="showcase-subtitle">Radio buttons</h4>
      <div class="card" style="padding: 0;">
        <div class="radio-section">
          <div class="radio-section-label">Show scroll bars</div>
          <label class="radio-item">
            <input type="radio" v-model="radioValue" value="auto" />
            <span class="radio-dot"></span>
            <span>Automatically based on mouse or trackpad</span>
          </label>
          <label class="radio-item">
            <input type="radio" v-model="radioValue" value="scrolling" />
            <span class="radio-dot"></span>
            <span>When scrolling</span>
          </label>
          <label class="radio-item">
            <input type="radio" v-model="radioValue" value="always" />
            <span class="radio-dot"></span>
            <span>Always</span>
          </label>
        </div>
        <div class="radio-divider"></div>
        <div class="radio-section">
          <div class="radio-section-label">Click in the scroll bar to</div>
          <label class="radio-item">
            <input type="radio" v-model="radioScroll" value="next-page" />
            <span class="radio-dot"></span>
            <span>Jump to the next page</span>
          </label>
          <label class="radio-item">
            <input type="radio" v-model="radioScroll" value="spot" />
            <span class="radio-dot"></span>
            <span>Jump to the spot that's clicked</span>
          </label>
        </div>
      </div>

      <h4 class="showcase-subtitle">Checkboxes</h4>
      <div class="showcase-row" style="gap:8px">
        <Checkbox :model-value="true">Select all</Checkbox>
        <Checkbox :model-value="false">Unchecked</Checkbox>
        <Checkbox :model-value="false" disabled>Disabled</Checkbox>
      </div>

      <h4 class="showcase-subtitle">Checkboxes in list context</h4>
      <div class="card-flush">
        <div class="list-item">
          <Checkbox :model-value="true" />
          <span class="list-item-name">opencode-desktop_2026-03-14.log</span>
          <span class="list-item-meta">97.2 MB</span>
        </div>
        <div class="list-item">
          <Checkbox :model-value="false" />
          <span class="list-item-name">opencode-desktop_2026-03-18.log</span>
          <span class="list-item-meta">47.9 MB</span>
        </div>
        <div class="list-item">
          <Checkbox :model-value="false" disabled />
          <span class="list-item-name" style="opacity:0.5">Protected file (locked)</span>
          <span class="list-item-meta" style="opacity:0.5">10.1 MB</span>
        </div>
      </div>

      <h4 class="showcase-subtitle">Toggle switches (in settings context)</h4>
      <div class="card" style="padding:0">
        <div style="display:flex;align-items:center;justify-content:space-between;padding:12px 16px;border-bottom:1px solid rgba(0,0,0,0.04)">
          <div><div style="font-size:13px;font-weight:500">Enable window drag</div><div style="font-size:11px;color:var(--muted)">Use custom JavaScript drag instead of native</div></div>
          <ToggleSwitch v-model="toggleValue" />
        </div>
        <div style="display:flex;align-items:center;justify-content:space-between;padding:12px 16px;border-bottom:1px solid rgba(0,0,0,0.04)">
          <div><div style="font-size:13px;font-weight:500">Caches</div><div style="font-size:11px;color:var(--muted)">Scan application and system caches</div></div>
          <ToggleSwitch v-model="toggleValue" />
        </div>
        <div style="display:flex;align-items:center;justify-content:space-between;padding:12px 16px">
          <div><div style="font-size:13px;font-weight:500">Docker</div><div style="font-size:11px;color:var(--muted)">Scan Docker images and containers</div></div>
          <ToggleSwitch v-model="toggleOff" />
        </div>
      </div>
    </section>

    <!-- ================================================================= -->
    <!-- PROGRESS BARS -->
    <!-- ================================================================= -->
    <section class="showcase-section">
      <h3 class="section-title">Progress Bars</h3>

      <h4 class="showcase-subtitle">Standard (6px)</h4>
      <div style="display:flex;flex-direction:column;gap:12px;margin-bottom:16px">
        <div class="progress-track"><div class="progress-fill progress-fill--accent" style="width:65%"></div></div>
        <div class="progress-track"><div class="progress-fill progress-fill--success" style="width:84%"></div></div>
        <div class="progress-track"><div class="progress-fill progress-fill--warning" style="width:45%"></div></div>
        <div class="progress-track"><div class="progress-fill progress-fill--danger" style="width:92%"></div></div>
      </div>

      <h4 class="showcase-subtitle">Thin (3px) — sensor bars</h4>
      <div style="display:flex;flex-direction:column;gap:8px;margin-bottom:16px">
        <div style="display:flex;align-items:center;gap:8px"><span style="font-size:12px;color:var(--text-secondary);width:40px">CPU</span><div class="progress-track progress-track--thin" style="flex:1"><div class="progress-fill progress-fill--danger" style="width:82%"></div></div><span style="font-size:11px;font-family:var(--font-mono);color:var(--danger-text)">79.3&deg;</span></div>
        <div style="display:flex;align-items:center;gap:8px"><span style="font-size:12px;color:var(--text-secondary);width:40px">GPU</span><div class="progress-track progress-track--thin" style="flex:1"><div class="progress-fill progress-fill--warning" style="width:68%"></div></div><span style="font-size:11px;font-family:var(--font-mono);color:var(--warning-text)">68.3&deg;</span></div>
        <div style="display:flex;align-items:center;gap:8px"><span style="font-size:12px;color:var(--text-secondary);width:40px">SSD</span><div class="progress-track progress-track--thin" style="flex:1"><div class="progress-fill progress-fill--accent" style="width:57%"></div></div><span style="font-size:11px;font-family:var(--font-mono);color:var(--text-secondary)">57.3&deg;</span></div>
      </div>

      <h4 class="showcase-subtitle">Thick (10px) — storage/CPU load</h4>
      <div style="display:flex;flex-direction:column;gap:12px;margin-bottom:16px">
        <div class="progress-labeled">
          <div class="progress-labeled-header"><span class="label">CPU Load</span><span class="value">31%</span></div>
          <div class="progress-track progress-track--thick"><div class="progress-fill progress-fill--accent" style="width:31%"></div></div>
        </div>
        <div class="progress-labeled">
          <div class="progress-labeled-header"><span class="label">Storage</span><span class="value" style="color:var(--warning-text)">84%</span></div>
          <div class="progress-track progress-track--thick"><div class="progress-fill progress-fill--warning" style="width:84%"></div></div>
        </div>
      </div>

      <h4 class="showcase-subtitle">Segmented — memory breakdown</h4>
      <div class="progress-segmented" style="margin-bottom:8px">
        <div class="seg" style="width:19.6%;background:#00b4d8"></div>
        <div class="seg" style="width:24.2%;background:#f97316"></div>
        <div class="seg" style="width:32.5%;background:#a78bfa"></div>
        <div class="seg" style="width:20%;background:#94a3b8"></div>
        <div class="seg" style="width:3.7%;background:#30d158"></div>
      </div>
      <div class="showcase-row" style="gap:12px;font-size:10px;color:var(--text-secondary)">
        <span style="display:flex;align-items:center;gap:4px"><span style="width:8px;height:8px;border-radius:50%;background:#00b4d8"></span> App 4.7 GB</span>
        <span style="display:flex;align-items:center;gap:4px"><span style="width:8px;height:8px;border-radius:50%;background:#f97316"></span> Wired 5.8 GB</span>
        <span style="display:flex;align-items:center;gap:4px"><span style="width:8px;height:8px;border-radius:50%;background:#a78bfa"></span> Compressed 7.8 GB</span>
        <span style="display:flex;align-items:center;gap:4px"><span style="width:8px;height:8px;border-radius:50%;background:#94a3b8"></span> Inactive 4.8 GB</span>
        <span style="display:flex;align-items:center;gap:4px"><span style="width:8px;height:8px;border-radius:50%;background:#30d158"></span> Free 275 MB</span>
      </div>

      <h4 class="showcase-subtitle">Two-segment — app footprint breakdown</h4>
      <div style="display:flex;width:100%;height:22px;border-radius:6px;overflow:hidden;background:rgba(0,0,0,0.04);margin-bottom:4px">
        <div style="width:60%;background:rgba(2,117,244,0.30);display:flex;align-items:center;justify-content:center;font-size:10px;font-weight:500;color:var(--accent-deep)">App 684.9 MB</div>
        <div style="width:40%;background:rgba(251,146,60,0.35);display:flex;align-items:center;justify-content:center;font-size:10px;font-weight:500;color:#c2410c">Leftovers 10.8 GB</div>
      </div>
    </section>

    <!-- ================================================================= -->
    <!-- TAB BAR -->
    <!-- ================================================================= -->
    <section class="showcase-section">
      <h3 class="section-title">TabBar</h3>

      <h4 class="showcase-subtitle">Default (options prop)</h4>
      <div class="showcase-row">
        <TabBar :options="segOptions" v-model="segValue" />
      </div>

      <h4 class="showcase-subtitle">With disabled item</h4>
      <div class="showcase-row">
        <TabBar :options="pillOptions" v-model="pillValue" />
      </div>

      <h4 class="showcase-subtitle">With badge (tabs prop)</h4>
      <div class="showcase-row">
        <TabBar :tabs="tabOptions" v-model="tabValue" />
      </div>
    </section>

    <!-- ================================================================= -->
    <!-- STATUS INDICATORS -->
    <!-- ================================================================= -->
    <section class="showcase-section">
      <h3 class="section-title">Status Indicators</h3>

      <h4 class="showcase-subtitle">Status dots</h4>
      <div class="showcase-row" style="gap: 16px">
        <span style="display:flex;align-items:center;gap:5px"><span class="status-dot status-dot--success"></span> Success</span>
        <span style="display:flex;align-items:center;gap:5px"><span class="status-dot status-dot--warning"></span> Warning</span>
        <span style="display:flex;align-items:center;gap:5px"><span class="status-dot status-dot--danger"></span> Danger</span>
        <span style="display:flex;align-items:center;gap:5px"><span class="status-dot status-dot--accent"></span> Accent</span>
        <span style="display:flex;align-items:center;gap:5px"><span class="status-dot status-dot--muted"></span> Muted</span>
      </div>

      <h4 class="showcase-subtitle">Live indicator</h4>
      <div class="showcase-row">
        <LiveIndicator />
        <span style="width: 24px"></span>
        <LiveIndicator :paused="true" />
      </div>

      <h4 class="showcase-subtitle">Spinners</h4>
      <div class="showcase-row">
        <span class="spinner"></span>
        <span style="width: 12px"></span>
        <span class="spinner-sm" style="border: 2px solid rgba(0,0,0,0.06); border-top-color: var(--accent);"></span>
        <span style="width: 12px"></span>
        <span class="spinner-xs"></span>
      </div>
    </section>

    <!-- ================================================================= -->
    <!-- TOGGLE SWITCH -->
    <!-- ================================================================= -->
    <section class="showcase-section">
      <h3 class="section-title">ToggleSwitch</h3>
      <div class="showcase-row">
        <ToggleSwitch v-model="toggleValue" /> On
        <span style="width: 24px"></span>
        <ToggleSwitch v-model="toggleOff" /> Off
      </div>
    </section>

    <!-- ================================================================= -->
    <!-- CARDS -->
    <!-- ================================================================= -->
    <section class="showcase-section">
      <h3 class="section-title">Cards</h3>

      <h4 class="showcase-subtitle">.card (content card)</h4>
      <div class="card">
        <p>Standard content card with var(--glass) background.</p>
      </div>

      <h4 class="showcase-subtitle">.card-flush (list item card)</h4>
      <div class="card-flush" style="padding: 14px 18px;">
        <p>Card flush — no padding by default, used for list items.</p>
      </div>

      <h4 class="showcase-subtitle">Empty state</h4>
      <EmptyState title="Nothing here" description="This is the shared EmptyState component with icon, title, and description." />
    </section>

    <!-- ================================================================= -->
    <!-- CLOSE / REVEAL BUTTONS -->
    <!-- ================================================================= -->
    <section class="showcase-section">
      <h3 class="section-title">Icon Buttons</h3>
      <div class="showcase-row">
        <button class="btn-close" title="Close">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M2 2 L12 12 M12 2 L2 12"/>
          </svg>
        </button>
        <span style="width: 12px"></span>
        <button class="btn-reveal" title="Reveal in Finder">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 1H2.5A1.5 1.5 0 001 2.5v9A1.5 1.5 0 002.5 13h9a1.5 1.5 0 001.5-1.5V9"/>
            <path d="M8 1h5v5"/>
            <path d="M13 1L6 8"/>
          </svg>
        </button>
      </div>
    </section>

    <!-- ================================================================= -->
    <!-- ONE-OFFS -->
    <!-- ================================================================= -->
    <section class="showcase-section">
      <h3 class="section-title">One-offs</h3>

      <h4 class="showcase-subtitle">FDA Warning Banner</h4>
      <div style="background:rgba(255,252,240,0.85);border:1px solid rgba(229,163,15,0.15);border-radius:var(--radius-md);padding:var(--sp-5)">
        <div style="display:flex;align-items:flex-start;gap:10px">
          <span style="width:8px;height:8px;border-radius:50%;background:var(--warning);margin-top:4px;flex-shrink:0"></span>
          <div>
            <div style="font-size:13px;font-weight:600;color:var(--text);margin-bottom:4px">Limited scan — Full Disk Access required</div>
            <div style="font-size:12px;color:var(--text-secondary);line-height:1.5">Without Full Disk Access, protected folders are skipped.</div>
            <div style="display:flex;gap:8px;margin-top:10px">
              <button class="btn-primary btn-sm">Open System Settings</button>
              <button class="btn-secondary btn-sm">Re-check</button>
            </div>
          </div>
        </div>
      </div>

      <h4 class="showcase-subtitle">Maintenance Info Banner</h4>
      <div style="background:var(--accent-light);border:1px solid rgba(2,117,244,0.1);border-radius:var(--radius-md);padding:var(--sp-4) var(--sp-5);display:flex;align-items:center;gap:10px">
        <span style="width:8px;height:8px;border-radius:50%;background:var(--accent);flex-shrink:0"></span>
        <span style="font-size:13px;color:var(--text-secondary)">Some tasks require administrator privileges.</span>
      </div>

      <h4 class="showcase-subtitle">Error message</h4>
      <div style="background:var(--danger-tint);border:1px solid rgba(217,75,75,0.12);border-radius:var(--radius-sm);padding:12px 16px;font-size:13px;font-weight:500;color:var(--danger-text)">
        Failed to scan: permission denied for /Library/Caches
      </div>

      <h4 class="showcase-subtitle">Success message</h4>
      <div style="background:var(--success-tint);border:1px solid rgba(30,142,90,0.12);border-radius:var(--radius-sm);padding:12px 16px;font-size:13px;font-weight:500;color:var(--success-text)">
        Successfully cleaned 4.3 GB of cache files
      </div>

      <h4 class="showcase-subtitle">Trash hero card</h4>
      <div class="card" style="text-align:center;padding:32px">
        <div style="display:flex;justify-content:center;gap:48px">
          <div><span style="font-size:32px;font-weight:700;font-variant-numeric:tabular-nums">4.8 GB</span><div style="font-size:11px;font-weight:600;text-transform:uppercase;color:var(--text-secondary);margin-top:4px">Total Size</div></div>
          <div style="width:1px;background:rgba(0,0,0,0.08)"></div>
          <div><span style="font-size:32px;font-weight:700;font-variant-numeric:tabular-nums">1,247</span><div style="font-size:11px;font-weight:600;text-transform:uppercase;color:var(--text-secondary);margin-top:4px">Items</div></div>
        </div>
      </div>

      <h4 class="showcase-subtitle">Confirmation card</h4>
      <div class="card" style="display:flex;align-items:center;justify-content:space-between;padding:16px 20px">
        <div>
          <div style="font-size:14px;font-weight:600">Empty Trash</div>
          <div style="font-size:12px;color:var(--muted)">Permanently remove all items from the trash</div>
        </div>
        <div style="display:flex;gap:8px">
          <button class="btn-secondary btn-sm">Cancel</button>
          <button class="btn-danger btn-sm">Yes, Empty Trash</button>
        </div>
      </div>

      <h4 class="showcase-subtitle">Docker status card</h4>
      <div class="card" style="display:flex;align-items:center;justify-content:space-between;padding:16px 20px">
        <div>
          <div style="font-size:15px;font-weight:600">Status</div>
          <span class="badge badge-success" style="margin-top:6px;display:inline-block">Installed</span>
          <div style="font-size:12px;color:var(--muted);margin-top:6px">Reclaimable space: 0B</div>
        </div>
        <div style="display:flex;gap:8px">
          <button class="btn-primary btn-sm">Prune</button>
          <button class="btn-danger btn-sm">Prune All</button>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.showcase {
  max-width: 1440px;
}

.showcase-section {
  margin-bottom: var(--sp-8);
}

.showcase-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-bottom: var(--sp-4);
}

.showcase-subtitle {
  font-size: 12px;
  font-weight: 500;
  color: var(--muted);
  margin-bottom: var(--sp-2);
  margin-top: var(--sp-4);
}

/* ===== COLOR PALETTE ===== */
.palette-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 8px;
  margin-bottom: var(--sp-4);
}
.swatch {
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid rgba(0,0,0,0.06);
}
.swatch-color {
  height: 48px;
}
.swatch-info {
  padding: 6px 8px;
  background: white;
}
.swatch-name {
  font-size: 11px;
  font-weight: 600;
  color: var(--text);
  display: block;
}
.swatch-value {
  font-size: 10px;
  font-family: var(--font-mono);
  color: var(--muted);
}

/* ===== FORM INPUTS ===== */
.input-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.input-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
}

.search-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}
.search-wrapper svg {
  position: absolute;
  left: 10px;
  color: var(--muted);
  pointer-events: none;
}
.search-wrapper input {
  padding-left: 32px;
  width: 100%;
}

/* ===== CUSTOM CHECKBOX ===== */
.inline-svg {
  position: absolute;
  width: 0;
  height: 0;
  pointer-events: none;
  user-select: none;
}
/* Checkbox styles now in Checkbox.vue component */

/* ===== LIST ITEM ===== */
.list-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid rgba(0,0,0,0.04);
}
.list-item:last-child { border-bottom: none; }
.list-item-name { flex: 1; font-weight: 500; font-size: 13px; }
.list-item-meta { font-size: 12px; color: var(--muted); font-family: var(--font-mono); }

/* ===== SLIDER ===== */
.slider-wrapper {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.slider {
  -webkit-appearance: none;
  appearance: none;
  width: 100%;
  height: 20px;
  background: transparent;
  outline: none;
  border: none;
  box-shadow: none;
  cursor: pointer;
}

.slider::-webkit-slider-runnable-track {
  height: 6px;
  border-radius: 3px;
  border: 0;
  outline: 0;
  box-shadow: none;
  -webkit-appearance: none;
  background: linear-gradient(
    to right,
    var(--accent) 0%,
    var(--accent) var(--fill, 0%),
    rgba(0, 0, 0, 0.1) var(--fill, 0%),
    rgba(0, 0, 0, 0.1) 100%
  );
}

.slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: white;
  border: 0.5px solid rgba(0, 0, 0, 0.08);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15), 0 0 0 0.5px rgba(0, 0, 0, 0.04);
  margin-top: -7px;
}

.slider-labels {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--muted);
}

/* ===== RADIO BUTTONS ===== */
.radio-section {
  padding: 14px 18px;
}

.radio-section-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
  margin-bottom: 10px;
}

.radio-divider {
  height: 1px;
  background: rgba(0, 0, 0, 0.06);
  margin: 0 18px;
}

.radio-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 5px 0;
  cursor: pointer;
  font-size: 13px;
  color: var(--text);
  -webkit-user-select: none;
  user-select: none;
}

.radio-item input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}

.radio-dot {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 2px solid rgba(0, 0, 0, 0.2);
  background: white;
  flex-shrink: 0;
  transition: border-color 0.15s, box-shadow 0.15s;
  position: relative;
}

.radio-dot::after {
  content: "";
  position: absolute;
  top: 50%;
  left: 50%;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--accent);
  transform: translate(-50%, -50%) scale(0);
  transition: transform 0.15s ease;
}

.radio-item input:checked + .radio-dot {
  border-color: var(--accent);
}

.radio-item input:checked + .radio-dot::after {
  transform: translate(-50%, -50%) scale(1);
}

.radio-item:hover .radio-dot {
  border-color: var(--accent);
}

/* ===== PROGRESS BARS ===== */
.progress-track {
  width: 100%;
  height: 6px;
  background: rgba(0,0,0,0.06);
  border-radius: 3px;
  overflow: hidden;
}
.progress-track--thin { height: 3px; }
.progress-track--thick { height: 10px; }
.progress-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.4s ease;
}
.progress-fill--accent { background: var(--accent); }
.progress-fill--success { background: var(--success); }
.progress-fill--warning { background: var(--warning); }
.progress-fill--danger { background: var(--danger); }

.progress-segmented {
  display: flex;
  width: 100%;
  height: 10px;
  background: rgba(0,0,0,0.06);
  border-radius: 5px;
  overflow: hidden;
}
.progress-segmented .seg {
  height: 100%;
  transition: width 0.4s ease;
}

.progress-labeled { margin-bottom: 8px; }
.progress-labeled-header {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  font-weight: 500;
  margin-bottom: 4px;
}
.progress-labeled-header .label { color: var(--text-secondary); }
.progress-labeled-header .value { color: var(--text); font-family: var(--font-mono); font-weight: 600; }
</style>
