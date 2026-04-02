<script setup lang="ts">
import { ref, onMounted, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { PathAccess, ScanArea } from "../types";

const checking = ref(false);
const hasFullDiskAccess = ref<boolean | null>(null);
const customJsDrag = ref(true);
const DRAG_MODE_KEY = "negative_space_use_custom_js_drag";

// Default scan areas — these are the directories the large file scanner will consider.
// Users can toggle each one on/off.
const scanAreas = ref<ScanArea[]>([
  { id: "desktop", label: "Desktop", path: "~/Desktop", description: "Files on your desktop", enabled: true, access: null },
  { id: "documents", label: "Documents", path: "~/Documents", description: "Your documents folder", enabled: true, access: null },
  { id: "downloads", label: "Downloads", path: "~/Downloads", description: "Downloaded files -- often a major source of clutter", enabled: true, access: null },
  { id: "pictures", label: "Pictures", path: "~/Pictures", description: "Photos and image files", enabled: true, access: null },
  { id: "movies", label: "Movies", path: "~/Movies", description: "Video files", enabled: true, access: null },
  { id: "music", label: "Music", path: "~/Music", description: "Audio files and libraries", enabled: true, access: null },
  { id: "library", label: "Library", path: "~/Library", description: "Application data, caches, preferences", enabled: true, access: null },
  { id: "applications", label: "Applications", path: "/Applications", description: "Installed applications", enabled: true, access: null },
]);

function loadSettings() {
  try {
    const saved = localStorage.getItem("negative_space_scan_areas");
    if (saved) {
      const parsed = JSON.parse(saved) as Record<string, boolean>;
      for (const area of scanAreas.value) {
        if (parsed[area.id] !== undefined) {
          area.enabled = parsed[area.id];
        }
      }
    }

    const savedDragMode = localStorage.getItem(DRAG_MODE_KEY);
    customJsDrag.value = savedDragMode === null ? true : savedDragMode !== "false";
  } catch (_) {
    // ignore parse errors
  }
}

function saveSettings() {
  const settings: Record<string, boolean> = {};
  for (const area of scanAreas.value) {
    settings[area.id] = area.enabled;
  }
  localStorage.setItem("negative_space_scan_areas", JSON.stringify(settings));
  localStorage.setItem(DRAG_MODE_KEY, customJsDrag.value ? "true" : "false");
}

// Save whenever a toggle changes
watch(scanAreas, saveSettings, { deep: true });
watch(customJsDrag, saveSettings);

async function checkAllAccess() {
  checking.value = true;
  try {
    // Check full disk access first
    hasFullDiskAccess.value = await invoke<boolean>("check_full_disk_access");

    if (hasFullDiskAccess.value) {
      // FDA granted -- all areas are accessible and should be enabled.
      for (const area of scanAreas.value) {
        area.access = {
          path: area.path,
          resolved_path: "",
          exists: true,
          readable: true,
        };
        area.enabled = true;
      }
    } else {
      // No FDA -- check each scan area individually.
      const checks = scanAreas.value.map(async (area) => {
        try {
          area.access = await invoke<PathAccess>("check_path_access", { path: area.path });
        } catch (_) {
          area.access = { path: area.path, resolved_path: "", exists: false, readable: false };
        }
      });
      await Promise.all(checks);

      // Auto-disable areas we can't access, enable ones we can.
      for (const area of scanAreas.value) {
        if (area.access && !area.access.readable) {
          area.enabled = false;
        } else if (area.access && area.access.readable) {
          area.enabled = true;
        }
      }
    }
    saveSettings();
  } finally {
    checking.value = false;
  }
}

async function openSettings() {
  try {
    await invoke("open_full_disk_access_settings");
  } catch (_) {
    // fallback
  }
}

const deniedMessage = ref("");

async function toggleArea(area: ScanArea) {
  // If toggling ON an area that has no access, trigger a permission request.
  if (!area.enabled && area.access && !area.access.readable) {
    deniedMessage.value = ""; // clear any old message
    area.enabled = true; // optimistically toggle on
    try {
      const granted = await invoke<boolean>("request_path_access", { path: area.path });
      if (granted) {
        // Access was granted -- update the badge.
        area.access = { ...area.access, readable: true };
      } else {
        // User denied or macOS silently blocked (previously denied).
        // Show a message explaining how to fix it.
        area.enabled = false;
        deniedMessage.value = `Access to ${area.label} was denied. To change this, open System Settings > Privacy & Security > Files and Folders, find Negativ_, and enable access.`;
      }
    } catch (_) {
      area.enabled = false;
    }
  } else {
    area.enabled = !area.enabled;
    deniedMessage.value = "";
  }
}

const enabledCount = computed(() => scanAreas.value.filter((a) => a.enabled).length);

onMounted(() => {
  loadSettings();
  checkAllAccess();
});
</script>

<template>
  <div class="settings">
    <div class="view-header">
      <h2>Settings</h2>
      <p class="text-muted">Manage permissions and scan areas</p>
    </div>

    <!-- Full Disk Access status -->
    <div class="card fda-section">
      <div class="section-header">
        <h3>Full Disk Access</h3>
        <button class="btn-secondary btn-sm" @click="checkAllAccess" :disabled="checking">
          {{ checking ? "Checking..." : "Re-check" }}
        </button>
      </div>

      <div v-if="hasFullDiskAccess === true" class="fda-status fda-granted">
        <span class="status-dot granted"></span>
        <span>Granted -- Negativ_ can access all folders.</span>
      </div>
      <div v-else-if="hasFullDiskAccess === false" class="fda-status fda-denied">
        <span class="status-dot denied"></span>
        <div class="fda-denied-content">
          <span>Not granted -- some folders may be inaccessible.</span>
          <button class="btn-primary btn-sm" @click="openSettings">Open System Settings</button>
        </div>
      </div>
      <div v-else class="fda-status">
        <span class="spinner spinner-xs"></span>
        <span>Checking...</span>
      </div>

      <p class="fda-help text-muted">
        Full Disk Access lets Negativ_ scan all folders without individual permission prompts.
        Open System Settings > Privacy & Security > Full Disk Access to grant it.
      </p>
    </div>

    <!-- Window behavior -->
    <div class="card behavior-section">
      <div class="section-header">
        <h3>Window Drag</h3>
      </div>
      <p class="text-muted section-desc">
        Smooth drag keeps gradients visually anchored during movement. Native drag can feel more macOS-like but may show gradient jitter.
      </p>

      <div class="behavior-row">
        <div class="behavior-info">
          <span class="behavior-label">Smooth gradient drag</span>
          <span class="behavior-desc text-muted">Uses custom JS window dragging with native background sync.</span>
        </div>
        <label class="toggle">
          <input type="checkbox" v-model="customJsDrag" />
          <span class="toggle-slider"></span>
        </label>
      </div>
    </div>

    <!-- Scan Areas -->
    <div class="card areas-section">
      <div class="section-header">
        <h3>Scan Areas</h3>
        <span class="area-summary text-muted">
          {{ enabledCount }} of {{ scanAreas.length }} enabled
        </span>
      </div>
      <p class="text-muted section-desc">
        Choose which directories are included when scanning for large files. Disabled areas will be skipped.
      </p>

      <div v-if="deniedMessage" class="denied-banner">
        <span>{{ deniedMessage }}</span>
        <button class="btn-primary btn-sm" @click="openSettings">Open System Settings</button>
      </div>

      <div class="area-list">
        <div
          v-for="area in scanAreas"
          :key="area.id"
          class="area-item"
          :class="{ disabled: !area.enabled }"
        >
          <div class="area-toggle">
            <label class="toggle">
              <input type="checkbox" :checked="area.enabled" @change="toggleArea(area)" />
              <span class="toggle-slider"></span>
            </label>
          </div>

          <div class="area-info">
            <div class="area-label-row">
              <span class="area-label">{{ area.label }}</span>
              <span class="area-path mono">{{ area.path }}</span>
            </div>
            <span class="area-desc text-muted">{{ area.description }}</span>
          </div>

          <div class="area-access">
            <template v-if="area.access">
              <span v-if="!area.access.exists" class="access-badge missing">Not found</span>
              <span v-else-if="area.access.readable" class="access-badge accessible">Accessible</span>
              <span v-else class="access-badge denied">No access</span>
            </template>
            <span v-else-if="checking" class="spinner spinner-xs"></span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings {
  max-width: 1440px;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--sp-3);
}

.section-header h3 {
  font-size: 16px;
  font-weight: 600;
}

.section-desc {
  font-size: 13px;
  margin-bottom: var(--sp-4);
}

/* FDA section */
.fda-section {
  margin-bottom: var(--sp-6);
}

.fda-status {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-3) 14px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  margin-bottom: var(--sp-3);
}

.fda-granted {
  background: var(--success-tint);
}

.fda-denied {
  background: var(--warning-tint);
}

.fda-denied-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  gap: var(--sp-3);
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-dot.granted {
  background: var(--success);
}

.status-dot.denied {
  background: var(--warning);
}

.fda-help {
  font-size: 12px;
  line-height: 1.5;
}

/* Denied access banner */
.denied-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-3);
  padding: var(--sp-3) var(--sp-4);
  border-radius: var(--radius-sm);
  background: var(--warning-tint);
  border: 1px solid rgba(194, 122, 18, 0.1);
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.5;
  margin-bottom: var(--sp-4);
  animation: fadeIn 0.3s ease;
}

.denied-banner .btn-sm {
  flex-shrink: 0;
}

/* Scan areas */
.areas-section {
  margin-bottom: var(--sp-6);
}

.behavior-section {
  margin-bottom: var(--sp-6);
}

.behavior-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-4);
}

.behavior-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.behavior-label {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.behavior-desc {
  font-size: 12px;
}

.area-summary {
  font-size: 12px;
}

.area-list {
  display: flex;
  flex-direction: column;
}

.area-item {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: var(--sp-3) 0;
  border-bottom: 1px solid var(--border-divider);
  transition: filter 0.2s;
}

.area-item:last-child {
  border-bottom: none;
}

.area-item.disabled {
  opacity: 0.5;
}

.area-toggle {
  flex-shrink: 0;
}

/* Toggle switch */
.toggle {
  position: relative;
  display: inline-block;
  width: 40px;
  height: 22px;
  cursor: pointer;
}

.toggle input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-slider {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.20);
  border-radius: 22px;
  transition: background 0.2s;
}

.toggle-slider::before {
  content: "";
  position: absolute;
  height: 18px;
  width: 18px;
  left: 2px;
  bottom: 2px;
  background: white;
  border-radius: 50%;
  transition: transform 0.2s;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

.toggle input:checked + .toggle-slider {
  background: var(--accent);
}

.toggle input:checked + .toggle-slider::before {
  transform: translateX(18px);
}

.area-info {
  flex: 1;
  min-width: 0;
}

.area-label-row {
  display: flex;
  align-items: baseline;
  gap: var(--sp-2);
  margin-bottom: 2px;
}

.area-label {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.area-path {
  font-size: 11px;
  color: var(--muted);
}

.area-desc {
  font-size: 12px;
  display: block;
}

.area-access {
  flex-shrink: 0;
  display: flex;
  align-items: center;
}

.access-badge {
  font-size: 11px;
  font-weight: 600;
  padding: 3px var(--sp-2);
  border-radius: var(--radius-sm);
  white-space: nowrap;
}

.access-badge.accessible {
  background: var(--success-tint);
  color: var(--success-text);
}

.access-badge.denied {
  background: var(--warning-tint);
  color: var(--warning-text);
}

.access-badge.missing {
  background: transparent;
  color: var(--muted);
}
</style>
