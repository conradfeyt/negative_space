<script setup lang="ts">
import { ref, onMounted } from "vue";
import { formatSize } from "../utils";
import { showToast } from "../stores/toastStore";
import StatCard from "../components/StatCard.vue";
import {
  vaultSummary,
  vaultEntries,
  vaultError,
  loadVaultSummary,
  restoreFromVault,
  deleteVaultEntry,
} from "../stores/scanStore";
import EmptyState from "../components/EmptyState.vue";
import ViewHeader from "../components/ViewHeader.vue";

const restoring = ref<string | null>(null);
const confirmDeleteId = ref<string | null>(null);

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
  <section class="vault-view">
    <ViewHeader
      title="Vault"
      subtitle="Securely stored sensitive files. Restore or permanently delete."
    />

    <!-- Error -->
    <div v-if="vaultError" class="error-banner">
      <span>{{ vaultError }}</span>
      <button class="btn-close" @click="vaultError = ''">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
      </button>
    </div>

    <!-- Summary -->
    <div v-if="vaultSummary && vaultSummary.file_count > 0" class="vault-summary">
      <StatCard :value="String(vaultSummary.file_count)" label="Secured" />
      <StatCard :value="formatSize(vaultSummary.total_savings)" label="Saved" />
      <StatCard :value="formatSize(vaultSummary.total_compressed_size)" label="Vault Size" />
    </div>

    <!-- Entries -->
    <div v-if="vaultEntries.length > 0" class="entry-list">
      <div v-for="entry in vaultEntries" :key="entry.id" class="card-flush entry-card">
        <div class="entry-main">
          <div class="entry-icon">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
              <path d="M14 2v6h6"/>
            </svg>
          </div>
          <div class="entry-info">
            <span class="entry-name">{{ entry.original_path.split('/').pop() }}</span>
            <span class="entry-path mono text-muted">{{ entry.original_path }}</span>
          </div>
          <div class="entry-stats">
            <span class="entry-size mono">{{ formatSize(entry.original_size) }}</span>
            <span class="entry-status text-muted">Secured</span>
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
      v-if="vaultEntries.length === 0"
      title="Vault is empty"
      description="Files secured from the Sensitive Content view will appear here. You can restore or permanently delete them."
    />
  </section>
</template>

<style scoped>
.vault-view {
  max-width: 1440px;
}

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

.vault-summary {
  display: flex;
  gap: 8px;
  margin-bottom: var(--sp-6);
}

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

.entry-size {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.entry-status {
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
</style>
