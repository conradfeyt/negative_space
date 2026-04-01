<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { formatSize } from "../utils";
import {
  caches,
  cachesScanning,
  cachesScanned,
  cachesError,
  scanCaches,
  deleteFiles,
  totalCacheSize,
  hasFullDiskAccess,
  checkFullDiskAccess,
} from "../stores/scanStore";

const selected = ref<Set<string>>(new Set());
const deleting = ref(false);
const successMsg = ref("");
const deleteError = ref("");

async function openFdaSettings() {
  try { await invoke("open_full_disk_access_settings"); } catch (_) {}
}
async function recheckFda() { await checkFullDiskAccess(); }

async function scan() {
  successMsg.value = "";
  deleteError.value = "";
  selected.value = new Set();
  await scanCaches();
}

async function cleanSelected() {
  if (selected.value.size === 0) return;
  deleting.value = true;
  deleteError.value = "";
  successMsg.value = "";
  try {
    const paths = Array.from(selected.value);
    const result = await deleteFiles(paths);
    if (result.success) {
      successMsg.value = `Cleaned ${result.deleted_count} cache(s), freed ${formatSize(result.freed_bytes)}`;
      caches.value = caches.value.filter((e) => !selected.value.has(e.path));
      selected.value = new Set();
    }
    if (result.errors.length > 0) deleteError.value = result.errors.join("; ");
  } catch (e) { deleteError.value = String(e); }
  finally { deleting.value = false; }
}

function toggleSelect(path: string) {
  const next = new Set(selected.value);
  if (next.has(path)) next.delete(path); else next.add(path);
  selected.value = next;
}

function toggleAll() {
  if (allSelected.value) selected.value = new Set();
  else selected.value = new Set(caches.value.map((e) => e.path));
}

const allSelected = computed(() => caches.value.length > 0 && selected.value.size === caches.value.length);
const totalSelected = computed(() => caches.value.filter((e) => selected.value.has(e.path)).reduce((sum, e) => sum + e.size, 0));
</script>

<template>
  <div class="caches-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Caches</h2>
          <p class="text-muted">Application and system caches</p>
        </div>
        <button class="btn-primary scan-btn" :disabled="cachesScanning" @click="scan">
          <span v-if="cachesScanning" class="spinner-sm"></span>
          {{ cachesScanning ? "Scanning..." : "Scan" }}
        </button>
      </div>
    </div>

    <div v-if="hasFullDiskAccess === false" class="fda-warning-banner">
      <span class="fda-warning-dot"></span>
      <div class="fda-warning-body">
        <div class="fda-warning-title">Limited scan -- Full Disk Access required</div>
        <div class="fda-warning-text">
          Without Full Disk Access, only Xcode and CoreSimulator caches are shown.
          Other app caches in ~/Library/Caches are skipped.
        </div>
        <div class="fda-warning-actions">
          <button class="btn-fda btn-fda-primary" @click="openFdaSettings">Open System Settings</button>
          <button class="btn-fda btn-fda-secondary" @click="recheckFda">Re-check</button>
        </div>
      </div>
    </div>

    <div v-if="cachesError" class="error-message">{{ cachesError }}</div>
    <div v-if="deleteError" class="error-message">{{ deleteError }}</div>
    <div v-if="successMsg" class="success-message">{{ successMsg }}</div>

    <div v-if="cachesScanning" class="loading-state">
      <span class="spinner"></span>
      <span>Scanning caches...</span>
    </div>

    <div v-else-if="cachesScanned && caches.length === 0" class="card empty-state">
      <p>No caches found</p>
    </div>

    <template v-else-if="caches.length > 0">
      <div class="summary-bar">
        <span class="results-count">
          {{ caches.length }} cache(s) -- {{ formatSize(totalCacheSize) }} total
        </span>
        <div class="results-actions">
          <span v-if="selected.size > 0" class="selected-info">
            {{ selected.size }} selected ({{ formatSize(totalSelected) }})
          </span>
          <button class="btn-danger" :disabled="selected.size === 0 || deleting" @click="cleanSelected">
            <span v-if="deleting" class="spinner-sm"></span>
            {{ deleting ? "Cleaning..." : "Clean Selected" }}
          </button>
        </div>
      </div>

      <div class="cache-list">
        <div
          v-for="entry in caches"
          :key="entry.path"
          :class="['card', 'cache-item', { selected: selected.has(entry.path) }]"
        >
          <div class="cache-item-left">
            <input type="checkbox" :checked="selected.has(entry.path)" @change="toggleSelect(entry.path)" />
            <div class="cache-info">
              <span class="cache-name">{{ entry.name }}</span>
              <span class="cache-path mono truncate" :title="entry.path">{{ entry.path }}</span>
            </div>
          </div>
          <div class="cache-item-right">
            <div class="cache-meta">
              <span class="cache-size mono">{{ formatSize(entry.size) }}</span>
              <span class="cache-count text-muted">{{ entry.item_count }} item(s)</span>
            </div>
          </div>
        </div>
      </div>

      <div class="select-all-row">
        <label class="select-all-label">
          <input type="checkbox" :checked="allSelected" @change="toggleAll" />
          Select all
        </label>
      </div>
    </template>
  </div>
</template>

<style scoped>
.caches-view { max-width: 740px; }

.cache-list { display: flex; flex-direction: column; gap: 8px; }

.cache-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--sp-4) var(--sp-5);
  transition: border-color 0.15s ease, box-shadow 0.15s ease, background 0.15s ease;
}

.cache-item.selected {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-light);
}

.cache-item-left { display: flex; align-items: center; gap: 14px; min-width: 0; flex: 1; }
.cache-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.cache-name { font-size: 14px; font-weight: 600; color: var(--text); }
.cache-path { font-size: 12px; color: var(--muted); max-width: 400px; }
.cache-item-right { flex-shrink: 0; margin-left: var(--sp-4); }
.cache-meta { display: flex; flex-direction: column; align-items: flex-end; gap: 2px; }
.cache-size { font-size: 14px; font-weight: 600; color: var(--text); }
.cache-count { font-size: 12px; }

.select-all-row { margin-top: var(--sp-3); padding: 0 var(--sp-1); }
.select-all-label { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--text-secondary); cursor: pointer; }
</style>
