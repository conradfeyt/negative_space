<script setup lang="ts">
import { ref, onMounted } from "vue";
import { BUILD_NUMBER } from "./buildNumber";
import { invoke } from "@tauri-apps/api/core";
import { useRouter, useRoute } from "vue-router";
import { checkFullDiskAccess, hasFullDiskAccess as _hasFullDiskAccess, domainStatus, checkDockerInstalled, dockerInstalled, restoreAllCaches, checkIntelligence } from "./stores/scanStore";
import { useScreenGradient } from "./composables/useScreenGradient";
import type { DomainStatus } from "./stores/scanStore";

interface NavItem {
  id: string;
  label: string;
  icon: string | string[];
  section?: string;
}

const router = useRouter();
const route = useRoute();

// SVG path data for sidebar icons — simple, rounded, system-adjacent.
// Monochrome stroke icons, consistent 24x24 viewBox.
const icons = {
  dashboard:    "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-4 0v-6a1 1 0 011-1h2a1 1 0 011 1v6m-6 0h6",
  largeFiles:   "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z",
  caches:       "M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10",
  logs:         "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2",
  docker:       [
    "M18 9.1V5a2 2 0 0 0-4 0",
    "M18 5a2 2 0 0 1 4 0",
    "M6 9.7 3.9 8.4C2.7 7.7 2 6.4 2 5V3c2 0 4 2 4 2s2-2 4-2v2c0 1.4-.7 2.7-1.9 3.4l-3.8 2.4A5 5 0 0 0 7 20h12c1.7 0 3-1.3 3-3v-3c0-2.8-2.2-5-5-5-2.7 0-5.1 1.4-6.4 3.6L9.7 14A2 2 0 0 1 6 13Z",
    "M15 15h.01",
  ],
  apps:         "M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4",
  trash:        "M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16",
  browsers:     "M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9",
  duplicates:   "M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z",
  vault:        "M21 8V5a2 2 0 00-2-2H5a2 2 0 00-2 2v3m18 0v11a2 2 0 01-2 2H5a2 2 0 01-2-2V8m18 0H3m7 4h4",
  spaceMap:     "M11 3.055A9.001 9.001 0 1020.945 13H11V3.055z M20.488 9H15V3.512A9.025 9.025 0 0120.488 9z",
  maintenance:  "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z",
  security:     "M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z",
  memory:       "M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z",
  packages:     "M20 7l-8-4-8 4m16 0v10l-8 4-8-4V7m16 0l-8 4-8-4m8 4v10",
  cpu:          "M6 18V6h12v12H6zM9 3v3m6-3v3M9 18v3m6-3v3M3 9h3m-3 6h3M18 9h3m-3 6h3",
  vitals:       "M3 12h4l3-9 4 18 3-9h4",
  thermal:      "M12 9V3m0 6a3 3 0 100 6 3 3 0 000-6zm-1-6h2v6h-2V3zM12 21a6 6 0 01-4-10.46V4a4 4 0 118 0v6.54A6 6 0 0112 21z",
  settings:     "M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4",
};

const navSections: { label: string; items: NavItem[] }[] = [
  {
    label: "Overview",
    items: [
      { id: "dashboard", label: "Dashboard", icon: icons.dashboard },
    ],
  },
  {
    label: "Cleanup",
    items: [
      { id: "large-files", label: "Large Files", icon: icons.largeFiles },
      { id: "caches", label: "Caches", icon: icons.caches },
      { id: "logs", label: "Logs", icon: icons.logs },
      { id: "docker", label: "Docker", icon: icons.docker },
      { id: "apps", label: "Apps", icon: icons.apps },
      { id: "trash", label: "Trash", icon: icons.trash },
      { id: "browsers", label: "Browsers", icon: icons.browsers },
      { id: "duplicates", label: "Duplicates", icon: icons.duplicates },
      { id: "vault", label: "Vault", icon: icons.vault },
    ],
  },
  {
    label: "Analysis",
    items: [
      { id: "space-map", label: "Space Map", icon: icons.spaceMap },
      { id: "cpu", label: "CPU", icon: icons.cpu },
      { id: "memory", label: "Memory", icon: icons.memory },
      { id: "packages", label: "Packages", icon: icons.packages },
    ],
  },
  {
    label: "System",
    items: [
      { id: "thermal", label: "Thermal", icon: icons.thermal },
      { id: "maintenance", label: "Maintenance", icon: icons.maintenance },
      { id: "security", label: "Security", icon: icons.security },
      { id: "settings", label: "Settings", icon: icons.settings },
    ],
  },
];

// Map sidebar nav IDs to domain keys in domainStatus
const navToDomain: Record<string, string> = {
  "large-files": "largeFiles",
  "caches":      "caches",
  "logs":        "logs",
  "docker":      "docker",
  "apps":        "apps",
  "trash":       "trash",
  "browsers":    "browsers",
  "security":    "security",
};

// Get the domain scan status for a nav item (returns undefined for items without domains)
function getDomainStatus(navId: string): DomainStatus | undefined {
  const key = navToDomain[navId];
  if (!key) return undefined;
  return domainStatus.value[key]?.status;
}

// Domains where item count is more meaningful than total size
const countBasedDomains = new Set(["largeFiles", "duplicates"]);

// Format a scan-result badge for a completed domain
function getDomainBadge(navId: string): string | null {
  const key = navToDomain[navId];
  if (!key) return null;
  const info = domainStatus.value[key];
  if (!info || info.status !== "done") return null;
  if (countBasedDomains.has(key)) {
    return info.itemCount > 0 ? String(info.itemCount) : null;
  }
  return info.totalSize > 0 ? formatBadgeSize(info.totalSize) : null;
}

// Compact size format for badges: "4.2G", "800M", "12K"
function formatBadgeSize(bytes: number): string {
  if (bytes === 0) return "0";
  const units = ["B", "K", "M", "G", "T"];
  const k = 1024;
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), units.length - 1);
  const val = bytes / Math.pow(k, i);
  return (i === 0 ? val.toFixed(0) : val.toFixed(val < 10 ? 1 : 0)) + units[i];
}

// FDA gate state
const fdaChecked = ref(false);
const fdaDismissed = ref(false);

function navigateTo(navId: string) {
  router.push({ name: navId });
}

async function openSystemSettings() {
  try {
    await invoke("open_full_disk_access_settings");
  } catch (e) { console.debug('[settings] FDA settings open failed:', e); }
}

async function recheckFda() {
  await checkFullDiskAccess();
}

function skipFda() {
  fdaDismissed.value = true;
}

const showApp = () => import.meta.env.DEV || _hasFullDiskAccess.value || fdaDismissed.value;

// Screen-anchored gradient background (renders both main + sidebar gradients,
// handles position polling, monitor topology watching, and custom JS drag).
const { startDrag } = useScreenGradient();

// Build info for FDA gate footer
const buildNumber = ref(0);
import("./buildNumber").then(m => { buildNumber.value = m.BUILD_NUMBER ?? 0; }).catch(() => {});

// FDA gate native icons — retry loading since invoke may not be ready on cold start
const iconSystemSettings = ref("");
const iconPrivacy = ref("");
const iconFda = ref("");
const iconNegativ = ref("");
const iconFdaSidebar = ref("");

async function loadFdaIcons() {
  for (let attempt = 0; attempt < 5; attempt++) {
    try {
      if (!iconSystemSettings.value) {
        const b64 = await invoke<string>("render_sf_symbol", { name: "/System/Applications/System Settings.app", size: 40, mode: "app", style: "plain" });
        if (b64) iconSystemSettings.value = b64;
      }
      if (!iconPrivacy.value) {
        const b64 = await invoke<string>("render_sf_symbol", { name: "hand.raised.fill", size: 40, mode: "sf", style: "blueGradientBadge" });
        if (b64) iconPrivacy.value = b64;
      }
      if (!iconFda.value) {
        const b64 = await invoke<string>("render_sf_symbol", { name: "externaldrive.fill", size: 40, mode: "sf", style: "grayBadge" });
        if (b64) iconFda.value = b64;
      }
      if (!iconNegativ.value) {
        const b64 = await invoke<string>("render_sf_symbol", { name: "NSApplicationIcon", size: 40, mode: "system", style: "plain" });
        if (b64) iconNegativ.value = b64;
      }
      if (!iconFdaSidebar.value) {
        const b64 = await invoke<string>("render_sf_symbol", { name: "exclamationmark.triangle.fill", size: 32, mode: "sf", style: "multicolor" });
        if (b64) iconFdaSidebar.value = b64;
      }
      if (iconSystemSettings.value && iconPrivacy.value && iconFda.value && iconNegativ.value && iconFdaSidebar.value) break;
    } catch (_) { /* retry */ }
    await new Promise(r => setTimeout(r, 500));
  }
}
onMounted(loadFdaIcons);

onMounted(async () => {
  try {
    await checkFullDiskAccess();
  } catch (_) { /* FDA check may fail in dev mode — proceed anyway */ }
  fdaChecked.value = true;
  // Fire-and-forget: don't block render waiting for these
  void checkDockerInstalled();
  void restoreAllCaches();
  void checkIntelligence();
});
</script>

<template>
  <!-- FDA Setup Gate -->
  <div v-if="fdaChecked && !showApp()" class="fda-gate">
    <div class="fda-drag-strip" @mousedown="startDrag"></div>
    <div class="fda-gate-content">
      <div class="fda-gate-icon">
        <svg width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
          <path d="M9 12l2 2 4-4"/>
        </svg>
      </div>
      <h1>Welcome to Negativ_</h1>
      <p class="fda-gate-subtitle">
        Negativ_ needs <strong style="color: var(--accent)">Full Disk Access</strong> to see your whole Mac.
      </p>
      <p class="fda-gate-detail">
        Without it, protected folders like Desktop, Documents, and Downloads
        will be skipped. You can always grant access later in Settings.
      </p>

      <div class="fda-gate-steps">
        <div class="fda-step">
          <span class="fda-step-num">1</span>
          <span class="fda-step-verb">Open</span>
          <img v-if="iconSystemSettings" :src="iconSystemSettings" alt="" class="fda-step-icon" />
          <strong>System Settings</strong>
        </div>
        <div class="fda-step">
          <span class="fda-step-num">2</span>
          <span class="fda-step-verb">Go to</span>
          <img v-if="iconPrivacy" :src="iconPrivacy" alt="" class="fda-step-icon" />
          <strong>Privacy &amp; Security</strong>
        </div>
        <div class="fda-step">
          <span class="fda-step-num">3</span>
          <span class="fda-step-verb">Find</span>
          <img v-if="iconFda" :src="iconFda" alt="" class="fda-step-icon" />
          <strong>Full Disk Access</strong>
        </div>
        <div class="fda-step">
          <span class="fda-step-num">4</span>
          <span class="fda-step-verb">Toggle</span>
          <img v-if="iconNegativ" :src="iconNegativ" alt="" class="fda-step-icon" />
          <strong>Negativ_</strong> on
        </div>
      </div>

      <div class="fda-gate-actions">
        <button class="btn-primary" @click="openSystemSettings" style="padding: 12px 28px; font-size: 14px;">
          Open System Settings
        </button>
        <button class="btn-secondary" @click="recheckFda" style="padding: 12px 28px; font-size: 14px;">
          Re-check Access
        </button>
      </div>
      <button class="btn-gate-skip" @click="skipFda">Skip for now</button>
    </div>
    <div class="fda-gate-version">v0.1.0 (build {{ buildNumber }})</div>
  </div>

  <!-- Main App — sidebar gradient is full-bleed, content panel inset -->
  <div v-else-if="fdaChecked" class="app-layout">
    <!-- Sidebar nav — sits on the native NSImageView gradient layer (behind webview) -->
    <aside class="sidebar" @mousedown="startDrag">
      <div class="sidebar-header" @mousedown="startDrag">
        <h1 class="app-title">Negativ_</h1>
        <button
          v-if="_hasFullDiskAccess === false"
          class="fda-sidebar-icon"
          title="Full Disk Access not granted — click to open Settings"
          @click.stop="router.push({ name: 'settings' })"
        >
          <img v-if="iconFdaSidebar" :src="iconFdaSidebar" alt="Warning" width="24" height="24" />
          <span v-else style="font-size: 14px;">&#9888;</span>
        </button>
      </div>

      <nav class="sidebar-nav">
        <template v-for="(section, sIdx) in navSections" :key="section.label">
          <div v-if="sIdx > 0" class="nav-divider"></div>
          <span class="nav-section-label">{{ section.label }}</span>
          <button
            v-for="item in section.items"
            v-show="item.id !== 'docker' || dockerInstalled !== false"
            :key="item.id"
            :class="['nav-item', { active: route.name === item.id }]"
            @click="navigateTo(item.id)"
          >
            <svg class="nav-icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <template v-if="Array.isArray(item.icon)">
                <path v-for="(d, pi) in item.icon" :key="pi" :d="d"/>
              </template>
              <path v-else :d="item.icon"/>
            </svg>
            <span class="nav-label">{{ item.label }}</span>
            <!-- Domain scan status badge -->
            <span v-if="getDomainStatus(item.id) === 'scanning'" class="nav-badge nav-badge-scanning">
              <span class="spinner-xs"></span>
            </span>
            <span v-else-if="getDomainBadge(item.id)" class="nav-badge nav-badge-result">
              {{ getDomainBadge(item.id) }}
            </span>
            <span v-else-if="getDomainStatus(item.id) === 'done'" class="nav-badge nav-badge-done">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
            </span>
            <span v-else-if="getDomainStatus(item.id) === 'error'" class="nav-badge nav-badge-error">
              <span class="nav-badge-dot"></span>
            </span>
          </button>
        </template>
      </nav>

      <div class="sidebar-footer">
        <span class="version-tag">v0.1.0 (build {{ BUILD_NUMBER }})</span>
      </div>
    </aside>

    <!-- Content panel — inset with rounded corners, main gradient inside -->
    <main class="content-panel">
      <div class="content">
        <div class="content-drag-strip" @mousedown="startDrag"></div>
        <router-view v-slot="{ Component }">
          <KeepAlive>
            <component :is="Component" />
          </KeepAlive>
        </router-view>
      </div>
    </main>
  </div>

  <!-- Loading -->
  <div v-else class="fda-gate">
    <div class="fda-drag-strip" @mousedown="startDrag"></div>
    <div class="fda-gate-content">
      <div class="spinner spinner-centered"></div>
      <p style="color: var(--muted); font-size: 14px;">Checking permissions...</p>
    </div>
  </div>
</template>

<style scoped>
/* ==========================================================================
   FDA Gate — onboarding screen
   ========================================================================== */
.fda-gate {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.05);
  position: relative;
}

.fda-drag-strip {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 48px;
}

.fda-gate-content {
  position: relative;
  max-width: 480px;
  text-align: center;
  padding: 36px 44px;
  background: rgba(255, 255, 255, 0.85);
  border: 1px solid rgba(255, 255, 255, 0.6);
  border-radius: var(--radius-xl);
  box-shadow: 0 8px 40px rgba(0, 0, 0, 0.12), 0 2px 8px rgba(0, 0, 0, 0.06);
}

.btn-gate-skip {
  position: absolute;
  top: 14px;
  right: 18px;
  background: none;
  border: none;
  font-size: 12px;
  font-weight: 500;
  color: var(--muted);
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  transition: color 0.15s, background 0.15s;
}

.btn-gate-skip:hover {
  color: var(--text-secondary);
  background: rgba(0, 0, 0, 0.04);
}

.fda-gate-icon {
  color: var(--accent);
  margin-bottom: 16px;
}

.fda-gate-content h1 {
  font-size: 26px;
  font-weight: 700;
  color: var(--text);
  letter-spacing: -0.5px;
  margin-bottom: 8px;
}

.fda-gate-subtitle {
  font-size: 15px;
  font-weight: 500;
  color: var(--text-secondary);
  line-height: 1.55;
  margin-bottom: 6px;
}

.fda-gate-detail {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
  line-height: 1.6;
  margin-bottom: 20px;
}

.fda-gate-steps {
  text-align: left;
  margin: 24px 0;
  padding: 16px 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
}

.fda-step {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: var(--text);
}

.fda-step-num {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.08);
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
}

.fda-step-verb {
  width: 42px;
  flex-shrink: 0;
  text-align: left;
}

.fda-step-icon {
  width: 20px;
  height: 20px;
  border-radius: 5px;
  flex-shrink: 0;
  margin-right: -4px;
}

.fda-gate-actions {
  display: flex;
  gap: 10px;
  justify-content: center;
}

.fda-gate-version {
  position: absolute;
  bottom: 16px;
  left: 50%;
  transform: translateX(-50%);
  font-size: 11px;
  color: rgba(255, 255, 255, 0.5);
  font-family: var(--font-mono);
}

/* ==========================================================================
   App Layout — sidebar gradient is full-bleed (#app::before),
   content panel is inset with rounded corners
   ========================================================================== */
.app-layout {
  position: relative;
  height: 100vh;
  overflow: hidden;
}

/* Subtle white wash over sidebar + gutters to soften the native gradient */
.app-layout::before {
  content: '';
  position: absolute;
  inset: 0;
  background: rgba(255, 255, 255, 0.05);
  z-index: 1;
  pointer-events: none;
}

/* ==========================================================================
   Sidebar — nav text sits directly on the full-bleed sidebar gradient.
   No backdrop-filter needed — the sidebar gradient IS the background.
   ========================================================================== */
.sidebar {
  position: absolute;
  top: 0;
  left: 0;
  bottom: 0;
  width: 230px;
  background: transparent;
  display: flex;
  flex-direction: column;
  user-select: none;
  z-index: 3;
}

.sidebar-header {
  padding: 48px 22px 18px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.fda-sidebar-icon {
  background: none;
  border: none;
  color: var(--warning);
  cursor: pointer;
  padding: 2px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  opacity: 0.85;
  transition: opacity 0.15s;
  animation: fda-pulse 3s ease-in-out 2;
}

.fda-sidebar-icon:hover {
  opacity: 1;
}

@keyframes fda-pulse {
  0%, 100% { opacity: 0.85; }
  50% { opacity: 0.5; }
}

.app-title {
  font-size: 17px;
  font-weight: 700;
  color: var(--sidebar-text);
  letter-spacing: -0.3px;
}

.sidebar-nav {
  flex: 1;
  padding: 4px 10px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

/* Section labels */
.nav-section-label {
  display: block;
  font-size: 10px;
  font-weight: 600;
  color: var(--sidebar-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.8px;
  padding: 12px 12px 5px;
}

.nav-divider {
  height: 1px;
  background: var(--sidebar-divider);
  margin: 4px 12px;
}

/* Nav items */
.nav-item {
  display: flex;
  align-items: center;
  gap: 11px;
  width: 100%;
  padding: 8px 12px;
  border-radius: 10px;
  background: transparent;
  border: none;
  cursor: pointer;
  transition: background 0.15s ease, box-shadow 0.15s ease;
  text-align: left;
}

.nav-item:hover {
  background: var(--sidebar-hover);
}

/* Active pill — aqua-to-mint gradient */
.nav-item.active {
  background: linear-gradient(135deg, rgba(2, 117, 244, 0.60) 0%, rgba(2, 117, 244, 0.40) 100%);
  box-shadow: 0 1px 6px rgba(2, 117, 244, 0.20);
}

.nav-icon {
  flex-shrink: 0;
  color: rgba(255, 255, 255, 0.75);
  transition: color 0.15s;
}

.nav-item:hover .nav-icon {
  color: var(--sidebar-text);
}

.nav-item.active .nav-icon {
  color: var(--sidebar-text);
}

.nav-label {
  font-size: 13px;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.75);
  line-height: 1.3;
  transition: color 0.15s;
}

.nav-item:hover .nav-label {
  color: var(--sidebar-text);
}

.nav-item.active .nav-label {
  color: var(--sidebar-text);
  font-weight: 600;
}

/* Nav status badges — appear at right edge of nav items */
.nav-badge {
  margin-left: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.nav-badge-scanning {
  opacity: 0.85;
}

/* Override spinner-xs color to match sidebar white text */
.nav-badge-scanning :deep(.spinner-xs) {
  border-color: rgba(255, 255, 255, 0.15);
  border-top-color: rgba(255, 255, 255, 0.8);
  width: 12px;
  height: 12px;
}

.nav-item.active .nav-badge-scanning :deep(.spinner-xs) {
  border-color: rgba(255, 255, 255, 0.2);
  border-top-color: var(--sidebar-text);
}

.nav-badge-result {
  font-size: 9px;
  font-weight: 600;
  letter-spacing: 0.02em;
  padding: 1px 6px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.18);
  color: rgba(255, 255, 255, 0.85);
  line-height: 1.4;
}

.nav-item.active .nav-badge-result {
  background: rgba(255, 255, 255, 0.22);
  color: rgba(255, 255, 255, 0.95);
}

.nav-badge-done {
  color: var(--success);
  opacity: 0.8;
}

.nav-badge-error .nav-badge-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--danger);
  opacity: 0.8;
}

.sidebar-footer {
  padding: 14px 22px;
  border-top: 1px solid var(--sidebar-divider);
}

.version-tag {
  font-size: 11px;
  color: var(--sidebar-text);
  font-family: var(--font-mono);
  opacity: 0.7;
}

/* ==========================================================================
   Content panel — inset from edges, rounded corners.
   The sidebar gradient wraps around this panel (visible in gutters).
   Main (warm) gradient shows through via ::before, white overlay on top.
   ========================================================================== */
.content-panel {
  position: absolute;
  top: 8px;
  right: 8px;
  bottom: 8px;
  left: 238px; /* 230px sidebar + 8px gap */
  border-radius: 12px;
  overflow: hidden;
  z-index: 2;
}

/* Main (warm) gradient — inside content panel, screen-anchored via
   background-position. Jitter is imperceptible through the white overlay. */
.content-panel::before {
  content: '';
  position: absolute;
  inset: -20px;
  background-image: var(--gradient-bg, none);
  background-size: var(--screen-w, 3840px) var(--screen-h, 2160px);
  background-position:
    calc(var(--bg-pos-x, 0px) + 20px - 238px)
    calc(var(--bg-pos-y, 0px) + 20px - 8px);
  background-repeat: no-repeat;
  z-index: 0;
}

/* White frosted overlay on top of main gradient for readability */
.content-panel::after {
  content: '';
  position: absolute;
  inset: 0;
  background: rgba(255, 255, 255, 0.70);
  z-index: 1;
}

.content {
  position: relative;
  z-index: 2;
  height: 100%;
  overflow-y: auto;
  padding: 0 40px 40px;
}

/* Invisible drag region at top of content area */
.content-drag-strip {
  height: 48px;
  flex-shrink: 0;
}

/* View transitions removed — opacity animations cause white flash on transparent
   Tauri windows when composited over blur filters. Instant swap is cleanest. */

.spinner-centered { margin: 0 auto var(--sp-4); }
</style>
