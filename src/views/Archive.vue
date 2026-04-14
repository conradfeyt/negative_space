<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { formatSize } from "../utils";
import { showToast } from "../stores/toastStore";
import StatCard from "../components/StatCard.vue";
import TabBar from "../components/TabBar.vue";
import type { TabOption } from "../components/TabBar.vue";
import {
  archiveSummary,
  archiveEntries,
  archiveCandidates,
  archiveScanning,
  archiveCompressing,
  archiveError,
  loadArchiveSummary,
  scanArchiveCandidates,
  compressToArchive,
  compressDirectoryToArchive,
  restoreFromArchive,
  deleteArchiveEntry,
  removeArchiveCandidates,
} from "../stores/scanStore";
import { useCompressionQueue } from "../composables/useCompressionQueue";
import EmptyState from "../components/EmptyState.vue";
import Checkbox from "../components/Checkbox.vue";
import ViewHeader from "../components/ViewHeader.vue";

// ---------------------------------------------------------------------------
// UI state
// ---------------------------------------------------------------------------

type Tab = "compress" | "archived";
const activeTab = ref<Tab>("compress");

const tabOptions = computed<TabOption[]>(() => [
  { value: "compress", label: "Compress" },
  { value: "archived", label: "Archived", badge: archiveEntries.value.length > 0 ? archiveEntries.value.length : undefined },
]);

const restoring = ref<string | null>(null);
const confirmDeleteId = ref<string | null>(null);
const minAgeDays = ref(30);

// Candidates selection
const selectedCandidates = ref<Set<string>>(new Set());

// ---------------------------------------------------------------------------
// Compression queue (composable)
// ---------------------------------------------------------------------------

const {
  queue,
  compressProgress,
  totalQueueSize,
  addFolderToQueue,
  removeFromQueue,
  clearQueue,
  compressQueue,
} = useCompressionQueue({
  compressDirectoryToArchive,
  compressToArchive,
  onError: (msg) => { archiveError.value = msg; },
  onSuccess: (msg) => showToast(msg),
  compressing: archiveCompressing,
});

// ---------------------------------------------------------------------------
// Candidates (file-level scan)
// ---------------------------------------------------------------------------

async function scanCandidates() {
  selectedCandidates.value = new Set();
  await scanArchiveCandidates("~", 10, minAgeDays.value);
}

function toggleCandidate(path: string) {
  const next = new Set(selectedCandidates.value);
  next.has(path) ? next.delete(path) : next.add(path);
  selectedCandidates.value = next;
}

function toggleAllCandidates() {
  if (selectedCandidates.value.size === archiveCandidates.value.length) {
    selectedCandidates.value = new Set();
  } else {
    selectedCandidates.value = new Set(archiveCandidates.value.map(c => c.path));
  }
}

const totalCandidateSavings = computed(() =>
  archiveCandidates.value
    .filter(c => selectedCandidates.value.has(c.path))
    .reduce((sum, c) => sum + c.estimated_savings, 0)
);

async function compressSelectedCandidates() {
  if (selectedCandidates.value.size === 0) return;
  const paths = Array.from(selectedCandidates.value);
  const result = await compressToArchive(paths);
  if (result.success || result.files_compressed > 0) {
    showToast(`Compressed ${result.files_compressed} file(s), saved ${formatSize(result.total_savings)}`);
    selectedCandidates.value = new Set();
    removeArchiveCandidates(paths);
  }
  if (result.errors.length) archiveError.value = result.errors.join("; ");
}

// ---------------------------------------------------------------------------
// Restore / Delete
// ---------------------------------------------------------------------------

async function handleRestore(entryId: string) {
  restoring.value = entryId;
  const result = await restoreFromArchive(entryId);
  restoring.value = null;
  if (result.success) {
    showToast(`Restored to ${result.restored_path}`);
  } else {
    showToast(result.errors.join("; "), "error");
  }
}

async function handleDelete(entryId: string) {
  confirmDeleteId.value = null;
  await deleteArchiveEntry(entryId);
  showToast("Permanently deleted from archive");
}

onMounted(loadArchiveSummary);
</script>

<template>
  <section class="archive-view">
    <ViewHeader
      title="Archive"
      subtitle="Compress files and folders to reclaim space. Restore anytime."
    >
      <template #actions>
        <button class="btn-secondary" :disabled="archiveCompressing" @click="addFolderToQueue">
          Add Folder...
        </button>
        <button class="btn-primary" :disabled="archiveScanning" @click="scanCandidates">
          <span v-if="archiveScanning" class="spinner-sm"></span>
          {{ archiveScanning ? "Scanning..." : "Find Files" }}
        </button>
      </template>
    </ViewHeader>

    <!-- Error -->
    <div v-if="archiveError" class="error-banner">
      <span>{{ archiveError }}</span>
      <button class="btn-close" @click="archiveError = ''">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
      </button>
    </div>

    <!-- Tab bar -->
    <TabBar :options="tabOptions" v-model="activeTab" class="archive-tab-bar" />

    <!-- ================================================================
         TAB: COMPRESS
         ================================================================ -->
    <template v-if="activeTab === 'compress'">

    <div v-if="archiveSummary && archiveSummary.file_count > 0" class="archive-summary">
      <StatCard :value="String(archiveSummary.file_count)" label="Archived" />
      <StatCard :value="formatSize(archiveSummary.total_savings)" label="Saved" />
      <StatCard :value="formatSize(archiveSummary.total_compressed_size)" label="Archive Size" />
    </div>

    <!-- ================================================================
         COMPRESSION QUEUE
         ================================================================ -->
    <div v-if="queue.length > 0" class="section">
      <div class="section-header">
        <h3 class="section-title">Compression Queue</h3>
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
            <span v-if="item.calculating" class="spinner-sm"></span>
            <span v-else class="queue-item-size mono">{{ formatSize(item.size) }}</span>
            <button class="btn-close" @click="removeFromQueue(item.path)">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
            </button>
          </div>
        </div>
      </div>

      <div class="queue-actions">
        <button
          class="btn-primary btn-compress"
          :disabled="archiveCompressing || queue.some(q => q.calculating)"
          @click="compressQueue"
        >
          <span v-if="archiveCompressing" class="spinner-sm"></span>
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
    <div v-if="archiveScanning" class="scan-state">
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
    <div v-if="!archiveScanning && archiveCandidates.length > 0" class="section">
      <div class="section-header">
        <h3 class="section-title">Compressible Files</h3>
        <span class="section-meta">{{ archiveCandidates.length }} file(s)</span>
      </div>

      <div class="candidates-toolbar">
        <Checkbox
          :model-value="selectedCandidates.size === archiveCandidates.length && archiveCandidates.length > 0"
          @change="toggleAllCandidates"
        >Select all</Checkbox>
        <div class="toolbar-right">
          <span v-if="selectedCandidates.size > 0" class="selected-info">
            {{ selectedCandidates.size }} selected &middot; ~{{ formatSize(totalCandidateSavings) }}
          </span>
          <button
            class="btn-primary btn-sm"
            :disabled="selectedCandidates.size === 0 || archiveCompressing"
            @click="compressSelectedCandidates"
          >
            Compress Selected
          </button>
        </div>
      </div>

      <div class="candidate-list">
        <div
          v-for="candidate in archiveCandidates"
          :key="candidate.path"
          :class="['card-flush', 'candidate-card', { selected: selectedCandidates.has(candidate.path) }]"
          @click="toggleCandidate(candidate.path)"
        >
          <div class="candidate-main">
            <Checkbox
              :model-value="selectedCandidates.has(candidate.path)"
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
    <EmptyState
      v-if="!archiveScanning && archiveCandidates.length === 0 && queue.length === 0"
      title="Compress files to free up space"
      description="Add folders to compress, or scan for large stale files that can be archived."
    />

    </template>

    <!-- ================================================================
         TAB: ARCHIVED
         ================================================================ -->
    <template v-if="activeTab === 'archived'">

    <div v-if="archiveEntries.length > 0" class="entry-list">
      <div v-for="entry in archiveEntries" :key="entry.id" class="card-flush entry-card">
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

    <EmptyState
      v-else
      title="Nothing archived yet"
      description="Compressed files and folders will appear here. You can restore them to their original location at any time."
    />

    </template>

  </section>
</template>

<style scoped>
.archive-view {
  max-width: 1440px;
}

.archive-tab-bar {
  display: flex;
  margin-bottom: var(--sp-6);
}

.archive-tab-bar :deep(.tab-btn) {
  flex: 1;
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
  color: var(--danger-text);
}


.archive-summary {
  display: flex;
  gap: 8px;
  margin-bottom: var(--sp-6);
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

.section-header .section-title {
  margin-bottom: 0;
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

.candidate-main > .checkbox { flex-shrink: 0; }

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

</style>
