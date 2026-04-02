<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { formatSize } from "../utils";
import Toast from "../components/Toast.vue";
import {
  vaultSummary,
  vaultEntries,
  vaultCandidates,
  vaultScanning,
  vaultCompressing,
  vaultError,
  loadVaultSummary,
  scanVaultCandidates,
  compressToVault,
  compressDirectoryToVault,
  restoreFromVault,
  deleteVaultEntry,
} from "../stores/scanStore";

// ---------------------------------------------------------------------------
// Queue: directories/files staged for compression
// ---------------------------------------------------------------------------

interface QueueItem {
  path: string;
  name: string;
  size: number;
  isDirectory: boolean;
  calculating: boolean;
}

type Tab = "compress" | "archived";
const activeTab = ref<Tab>("compress");

const queue = ref<QueueItem[]>([]);
const toast = ref<{ message: string; type: "success" | "error" | "info" } | null>(null);
const restoring = ref<string | null>(null);
const confirmDeleteId = ref<string | null>(null);
const compressProgress = ref<{ current: number; total: number; name: string } | null>(null);
const minAgeDays = ref(30);

// Candidates selection
const selectedCandidates = ref<Set<string>>(new Set());

function showToast(message: string, type: "success" | "error" | "info" = "success") {
  toast.value = { message, type };
}

// ---------------------------------------------------------------------------
// Add to queue
// ---------------------------------------------------------------------------

async function addFolderToQueue() {
  const folder = await open({
    directory: true,
    multiple: false,
    title: "Choose a folder to add to compression queue",
  });
  if (!folder || typeof folder !== "string") return;

  // Check if already in queue
  if (queue.value.some(q => q.path === folder)) {
    showToast("This folder is already in the queue", "info");
    return;
  }

  const name = folder.split("/").pop() || folder;
  const item: QueueItem = { path: folder, name, size: 0, isDirectory: true, calculating: true };
  queue.value.push(item);

  // Calculate size in background
  try {
    const size = await invoke<number>("get_directory_size", { path: folder });
    const idx = queue.value.findIndex(q => q.path === folder);
    if (idx !== -1) {
      queue.value[idx].size = size;
      queue.value[idx].calculating = false;
    }
  } catch {
    const idx = queue.value.findIndex(q => q.path === folder);
    if (idx !== -1) queue.value[idx].calculating = false;
  }
}

function removeFromQueue(path: string) {
  queue.value = queue.value.filter(q => q.path !== path);
}

function clearQueue() {
  queue.value = [];
}

const totalQueueSize = computed(() =>
  queue.value.reduce((sum, q) => sum + q.size, 0)
);

// ---------------------------------------------------------------------------
// Compress queue
// ---------------------------------------------------------------------------

async function compressQueue() {
  if (queue.value.length === 0) return;

  const items = [...queue.value];
  let totalSavings = 0;
  let compressed = 0;
  const errors: string[] = [];

  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    compressProgress.value = { current: i + 1, total: items.length, name: item.name };

    if (item.isDirectory) {
      const result = await compressDirectoryToVault(item.path);
      if (result.success || result.files_compressed > 0) {
        totalSavings += result.total_savings;
        compressed++;
        queue.value = queue.value.filter(q => q.path !== item.path);
      }
      if (result.errors.length) errors.push(...result.errors);
    } else {
      const result = await compressToVault([item.path]);
      if (result.success || result.files_compressed > 0) {
        totalSavings += result.total_savings;
        compressed++;
        queue.value = queue.value.filter(q => q.path !== item.path);
      }
      if (result.errors.length) errors.push(...result.errors);
    }
  }

  compressProgress.value = null;

  if (compressed > 0) {
    showToast(`Archived ${compressed} item(s), saved ${formatSize(totalSavings)}`);
  }
  if (errors.length > 0) {
    vaultError.value = errors.join("; ");
  }
}

// ---------------------------------------------------------------------------
// Candidates (file-level scan)
// ---------------------------------------------------------------------------

async function scanCandidates() {
  selectedCandidates.value = new Set();
  await scanVaultCandidates("~", 10, minAgeDays.value);
}

function toggleCandidate(path: string) {
  const next = new Set(selectedCandidates.value);
  next.has(path) ? next.delete(path) : next.add(path);
  selectedCandidates.value = next;
}

function toggleAllCandidates() {
  if (selectedCandidates.value.size === vaultCandidates.value.length) {
    selectedCandidates.value = new Set();
  } else {
    selectedCandidates.value = new Set(vaultCandidates.value.map(c => c.path));
  }
}

const totalCandidateSavings = computed(() =>
  vaultCandidates.value
    .filter(c => selectedCandidates.value.has(c.path))
    .reduce((sum, c) => sum + c.estimated_savings, 0)
);

async function compressSelectedCandidates() {
  if (selectedCandidates.value.size === 0) return;
  const paths = Array.from(selectedCandidates.value);
  const result = await compressToVault(paths);
  if (result.success || result.files_compressed > 0) {
    showToast(`Compressed ${result.files_compressed} file(s), saved ${formatSize(result.total_savings)}`);
    selectedCandidates.value = new Set();
    vaultCandidates.value = vaultCandidates.value.filter(c => !paths.includes(c.path));
  }
  if (result.errors.length) vaultError.value = result.errors.join("; ");
}

// ---------------------------------------------------------------------------
// Restore / Delete
// ---------------------------------------------------------------------------

async function handleRestore(entryId: string) {
  restoring.value = entryId;
  const result = await restoreFromVault(entryId);
  restoring.value = null;
  if (result.success) {
    showToast(`Restored to ${result.restored_path}`);
  } else {
    showToast(result.errors.join("; "), "error");
  }
}

async function handleDelete(entryId: string) {
  confirmDeleteId.value = null;
  await deleteVaultEntry(entryId);
  showToast("Permanently deleted from vault");
}

onMounted(loadVaultSummary);
</script>

<template>
  <div class="vault-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Vault</h2>
          <p class="text-muted">Compress files and folders to reclaim space. Restore anytime.</p>
        </div>
        <div class="header-buttons">
          <button class="btn-secondary" :disabled="vaultCompressing" @click="addFolderToQueue">
            Add Folder...
          </button>
          <button class="btn-primary" :disabled="vaultScanning" @click="scanCandidates">
            <span v-if="vaultScanning" class="spinner spinner-sm"></span>
            {{ vaultScanning ? "Scanning..." : "Find Files" }}
          </button>
        </div>
      </div>
    </div>

    <!-- Toast -->
    <Toast
      v-if="toast"
      :message="toast.message"
      :type="toast.type"
      @dismiss="toast = null"
    />

    <!-- Error -->
    <div v-if="vaultError" class="error-banner">
      <span>{{ vaultError }}</span>
      <button class="dismiss-btn" @click="vaultError = ''">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
      </button>
    </div>

    <!-- Tab bar -->
    <div class="tab-bar">
      <button
        :class="['tab-btn', { active: activeTab === 'compress' }]"
        @click="activeTab = 'compress'"
      >
        Compress
      </button>
      <button
        :class="['tab-btn', { active: activeTab === 'archived' }]"
        @click="activeTab = 'archived'"
      >
        Archived
        <span v-if="vaultEntries.length > 0" class="tab-count">{{ vaultEntries.length }}</span>
      </button>
    </div>

    <!-- ================================================================
         TAB: COMPRESS
         ================================================================ -->
    <template v-if="activeTab === 'compress'">

    <!-- Vault Summary -->
    <div v-if="vaultSummary && vaultSummary.file_count > 0" class="vault-summary">
      <div class="summary-stat">
        <span class="summary-stat-value">{{ vaultSummary.file_count }}</span>
        <span class="summary-stat-label">Archived</span>
      </div>
      <div class="summary-stat">
        <span class="summary-stat-value">{{ formatSize(vaultSummary.total_savings) }}</span>
        <span class="summary-stat-label">Saved</span>
      </div>
      <div class="summary-stat">
        <span class="summary-stat-value">{{ formatSize(vaultSummary.total_compressed_size) }}</span>
        <span class="summary-stat-label">Vault Size</span>
      </div>
    </div>

    <!-- ================================================================
         COMPRESSION QUEUE
         ================================================================ -->
    <div v-if="queue.length > 0" class="section">
      <div class="section-header">
        <h3>Compression Queue</h3>
        <div class="section-actions">
          <span class="section-meta">{{ queue.length }} item(s) &middot; {{ formatSize(totalQueueSize) }}</span>
          <button class="btn-ghost btn-sm" @click="clearQueue">Clear</button>
        </div>
      </div>

      <div class="queue-list">
        <div v-for="item in queue" :key="item.path" class="queue-item card-flush">
          <div class="queue-item-left">
            <svg class="queue-icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
            </svg>
            <div class="queue-item-info">
              <span class="queue-item-name">{{ item.name }}</span>
              <span class="queue-item-path mono text-muted">{{ item.path }}</span>
            </div>
          </div>
          <div class="queue-item-right">
            <span v-if="item.calculating" class="spinner spinner-sm"></span>
            <span v-else class="queue-item-size mono">{{ formatSize(item.size) }}</span>
            <button class="btn-remove" @click="removeFromQueue(item.path)">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
            </button>
          </div>
        </div>
      </div>

      <div class="queue-actions">
        <button
          class="btn-primary btn-compress"
          :disabled="vaultCompressing || queue.some(q => q.calculating)"
          @click="compressQueue"
        >
          <span v-if="vaultCompressing" class="spinner spinner-sm"></span>
          <template v-if="compressProgress">
            Compressing {{ compressProgress.current }}/{{ compressProgress.total }}: {{ compressProgress.name }}
          </template>
          <template v-else>
            Compress {{ queue.length }} Item(s)
          </template>
        </button>
      </div>
    </div>

    <!-- ================================================================
         SCANNING STATE
         ================================================================ -->
    <div v-if="vaultScanning" class="scan-state">
      <div class="scan-animation">
        <span class="scan-dot"></span>
        <span class="scan-dot"></span>
        <span class="scan-dot"></span>
      </div>
      <span>Scanning for compressible files...</span>
    </div>

    <!-- ================================================================
         FILE CANDIDATES (from scan)
         ================================================================ -->
    <div v-if="!vaultScanning && vaultCandidates.length > 0" class="section">
      <div class="section-header">
        <h3>Compressible Files</h3>
        <span class="section-meta">{{ vaultCandidates.length }} file(s)</span>
      </div>

      <div class="candidates-toolbar">
        <label class="select-all-label">
          <input
            type="checkbox"
            :checked="selectedCandidates.size === vaultCandidates.length && vaultCandidates.length > 0"
            @change="toggleAllCandidates"
          />
          Select all
        </label>
        <div class="toolbar-right">
          <span v-if="selectedCandidates.size > 0" class="selected-info">
            {{ selectedCandidates.size }} selected &middot; ~{{ formatSize(totalCandidateSavings) }}
          </span>
          <button
            class="btn-primary btn-sm"
            :disabled="selectedCandidates.size === 0 || vaultCompressing"
            @click="compressSelectedCandidates"
          >
            Compress Selected
          </button>
        </div>
      </div>

      <div class="candidate-list">
        <div
          v-for="candidate in vaultCandidates"
          :key="candidate.path"
          :class="['card-flush', 'candidate-card', { selected: selectedCandidates.has(candidate.path) }]"
          @click="toggleCandidate(candidate.path)"
        >
          <div class="candidate-main">
            <input
              type="checkbox"
              :checked="selectedCandidates.has(candidate.path)"
              @click.stop
              @change="toggleCandidate(candidate.path)"
            />
            <div class="candidate-info">
              <span class="candidate-name">{{ candidate.name }}</span>
              <span class="candidate-path mono text-muted">{{ candidate.path }}</span>
            </div>
            <div class="candidate-stats">
              <span class="candidate-savings">~{{ formatSize(candidate.estimated_savings) }}</span>
              <span class="candidate-size mono text-muted">{{ formatSize(candidate.size) }}</span>
            </div>
          </div>
          <div v-if="candidate.recently_accessed" class="candidate-warning">
            Recently accessed — check that no apps depend on this file after compression.
          </div>
        </div>
      </div>
    </div>

    <!-- Empty state (compress tab) -->
    <div
      v-if="!vaultScanning && vaultCandidates.length === 0 && queue.length === 0"
      class="empty-state-card"
    >
      <div class="empty-icon">
        <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 8V5a2 2 0 00-2-2H5a2 2 0 00-2 2v3m18 0v11a2 2 0 01-2 2H5a2 2 0 01-2-2V8m18 0H3m7 4h4"/>
        </svg>
      </div>
      <p class="empty-title">Compress files to free up space</p>
      <p class="empty-desc text-muted">
        Add folders to compress, or scan for large stale files that can be archived.
      </p>
    </div>

    </template>

    <!-- ================================================================
         TAB: ARCHIVED
         ================================================================ -->
    <template v-if="activeTab === 'archived'">

    <div v-if="vaultEntries.length > 0" class="entry-list">
      <div v-for="entry in vaultEntries" :key="entry.id" class="card-flush entry-card">
        <div class="entry-main">
          <div class="entry-icon">
            <svg v-if="entry.file_type === 'directory'" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
            </svg>
            <svg v-else width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
              <path d="M14 2v6h6"/>
            </svg>
          </div>
          <div class="entry-info">
            <span class="entry-name">{{ entry.original_path.split('/').pop() }}</span>
            <span class="entry-path mono text-muted">{{ entry.original_path }}</span>
          </div>
          <div class="entry-stats">
            <span class="entry-savings">{{ formatSize(entry.original_size - entry.compressed_size) }} saved</span>
            <span class="entry-ratio text-muted">{{ Math.round((1 - entry.compression_ratio) * 100) }}% smaller</span>
          </div>
          <div class="entry-actions">
            <button
              class="btn-primary btn-sm"
              :disabled="restoring === entry.id"
              @click="handleRestore(entry.id)"
            >
              {{ restoring === entry.id ? "Restoring..." : "Restore" }}
            </button>
            <button
              v-if="confirmDeleteId !== entry.id"
              class="btn-ghost btn-sm"
              @click="confirmDeleteId = entry.id"
            >Delete</button>
            <template v-else>
              <button class="btn-danger btn-sm" @click="handleDelete(entry.id)">Confirm</button>
              <button class="btn-ghost btn-sm" @click="confirmDeleteId = null">Cancel</button>
            </template>
          </div>
        </div>
        <div class="entry-meta text-muted">
          {{ entry.archived_at }} &middot;
          {{ formatSize(entry.original_size) }} → {{ formatSize(entry.compressed_size) }} &middot;
          {{ entry.file_type }}
        </div>
      </div>
    </div>

    <div v-else class="empty-state-card">
      <div class="empty-icon">
        <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 8V5a2 2 0 00-2-2H5a2 2 0 00-2 2v3m18 0v11a2 2 0 01-2 2H5a2 2 0 01-2-2V8m18 0H3m7 4h4"/>
        </svg>
      </div>
      <p class="empty-title">Nothing archived yet</p>
      <p class="empty-desc text-muted">
        Compressed files and folders will appear here. You can restore them to their original location at any time.
      </p>
    </div>

    </template>

  </div>
</template>

<style scoped>
.vault-view {
  max-width: 1440px;
}

.header-buttons {
  display: flex;
  gap: var(--sp-2);
}

/* Tab bar */
.tab-bar {
  display: flex;
  gap: 2px;
  margin-bottom: var(--sp-6);
  padding: 3px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.04);
}

.tab-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 7px 16px;
  font-size: 13px;
  font-weight: 500;
  color: var(--muted);
  border: none;
  border-radius: 8px;
  background: transparent;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.tab-btn:hover:not(.active) {
  color: var(--text-secondary);
}

.tab-btn.active {
  background: rgba(255, 255, 255, 0.8);
  color: var(--text);
  font-weight: 600;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
}

.tab-count {
  font-size: 11px;
  font-weight: 600;
  padding: 0 6px;
  border-radius: 10px;
  background: var(--accent-light);
  color: var(--accent-deep);
  line-height: 1.6;
}

/* Error banner (dismissible) */
.error-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 14px;
  margin-bottom: var(--sp-4);
  border-radius: 10px;
  font-size: 13px;
  font-weight: 500;
  background: rgba(255, 69, 58, 0.1);
  border: 1px solid rgba(255, 69, 58, 0.15);
  color: #c03030;
}

.dismiss-btn {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: inherit;
  opacity: 0.5;
  cursor: pointer;
}

.dismiss-btn:hover {
  opacity: 1;
  background: rgba(0, 0, 0, 0.06);
}

/* Vault summary */
.vault-summary {
  display: flex;
  gap: 8px;
  margin-bottom: var(--sp-6);
}

.summary-stat {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 14px 12px;
  border-radius: var(--radius-md);
  background: var(--surface);
  box-shadow: var(--shadow-sm);
}

.summary-stat-value {
  font-size: 18px;
  font-weight: 700;
  color: var(--text);
  font-variant-numeric: tabular-nums;
}

.summary-stat-label {
  font-size: 10px;
  font-weight: 600;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.4px;
}

/* Sections */
.section {
  margin-bottom: var(--sp-8);
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

.section-actions {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.section-meta {
  font-size: 12px;
  color: var(--muted);
}

/* ----------------------------------------------------------------
   Queue
   ---------------------------------------------------------------- */
.queue-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
}

.queue-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-3) var(--sp-4);
}

.queue-item-left {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  flex: 1;
  min-width: 0;
}

.queue-icon {
  flex-shrink: 0;
  color: var(--muted);
}

.queue-item-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.queue-item-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.queue-item-path {
  font-size: 11px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.queue-item-right {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  flex-shrink: 0;
}

.queue-item-size {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.btn-remove {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.btn-remove:hover {
  background: rgba(255, 69, 58, 0.1);
  color: var(--danger);
}

.queue-actions {
  margin-top: var(--sp-4);
  display: flex;
  justify-content: flex-end;
}

.btn-compress {
  min-width: 200px;
  justify-content: center;
  display: flex;
  align-items: center;
  gap: 8px;
}

/* ----------------------------------------------------------------
   Archived entries
   ---------------------------------------------------------------- */
.entry-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
}

.entry-card {
  padding: var(--sp-4) var(--sp-5);
}

.entry-main {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.entry-icon {
  flex-shrink: 0;
  color: var(--muted);
}

.entry-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.entry-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.entry-path {
  font-size: 11px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.entry-stats {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  flex-shrink: 0;
}

.entry-savings {
  font-size: 13px;
  font-weight: 600;
  color: var(--success);
}

.entry-ratio {
  font-size: 11px;
}

.entry-actions {
  display: flex;
  gap: var(--sp-2);
  flex-shrink: 0;
  margin-left: var(--sp-2);
}

.entry-meta {
  font-size: 11px;
  margin-top: var(--sp-2);
  padding-top: var(--sp-2);
  border-top: 1px solid var(--border-divider);
}

/* ----------------------------------------------------------------
   Scanning state
   ---------------------------------------------------------------- */
.scan-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--sp-4);
  padding: 48px 0;
  font-size: 13px;
  color: var(--muted);
}

.scan-animation {
  display: flex;
  gap: 6px;
}

.scan-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--accent);
  animation: scanPulse 1.4s ease-in-out infinite;
}

.scan-dot:nth-child(2) { animation-delay: 0.2s; }
.scan-dot:nth-child(3) { animation-delay: 0.4s; }

@keyframes scanPulse {
  0%, 80%, 100% { opacity: 0.2; transform: scale(0.8); }
  40% { opacity: 1; transform: scale(1.1); }
}

/* ----------------------------------------------------------------
   Candidates
   ---------------------------------------------------------------- */
.candidates-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--sp-3);
}

.select-all-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text-secondary);
  cursor: pointer;
}

.toolbar-right {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.selected-info {
  font-size: 12px;
  color: var(--accent-deep);
  font-weight: 500;
}

.candidate-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
}

.candidate-card {
  padding: var(--sp-3) var(--sp-4);
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.candidate-card.selected {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-light);
}

.candidate-card:hover {
  background: var(--surface-alt);
}

.candidate-main {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.candidate-main input[type="checkbox"] { flex-shrink: 0; }

.candidate-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.candidate-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.candidate-path {
  font-size: 11px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.candidate-stats {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  flex-shrink: 0;
}

.candidate-savings {
  font-size: 13px;
  font-weight: 600;
  color: var(--success);
}

.candidate-size { font-size: 11px; }

.candidate-warning {
  margin: var(--sp-2) 0 0 28px;
  padding: var(--sp-2) var(--sp-3);
  font-size: 11px;
  line-height: 1.5;
  color: var(--warning-text);
  background: var(--warning-tint);
  border-radius: var(--radius-sm);
}

/* ----------------------------------------------------------------
   Empty state
   ---------------------------------------------------------------- */
.empty-state-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 64px 32px;
}

.empty-icon {
  color: var(--muted);
  opacity: 0.4;
  margin-bottom: var(--sp-4);
}

.empty-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text);
  margin-bottom: var(--sp-2);
}

.empty-desc {
  font-size: 13px;
  max-width: 380px;
  line-height: 1.6;
}
</style>
