<script setup lang="ts">
import { ref, onMounted } from "vue";
import { formatSize } from "../utils";
import {
  trashInfo,
  trashLoading,
  trashError,
  loadTrashInfo,
  emptyTrash as storeEmptyTrash,
} from "../stores/scanStore";

const emptying = ref(false);
const confirmEmpty = ref(false);
const result = ref("");

async function handleEmptyTrash() {
  emptying.value = true;
  result.value = "";
  confirmEmpty.value = false;
  try {
    const cleanResult = await storeEmptyTrash();
    if (cleanResult.success) {
      result.value = `Trash emptied, freed ${formatSize(cleanResult.freed_bytes)}`;
    }
    if (cleanResult.errors.length > 0) {
      trashError.value = cleanResult.errors.join("; ");
    }
  } catch (e) {
    trashError.value = String(e);
  } finally {
    emptying.value = false;
  }
}

function requestEmpty() {
  confirmEmpty.value = true;
}

function cancelEmpty() {
  confirmEmpty.value = false;
}

onMounted(loadTrashInfo);
</script>

<template>
  <div class="trash-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Trash</h2>
          <p class="text-muted">View and empty system trash</p>
        </div>
        <button
          class="btn-secondary"
          :disabled="trashLoading"
          @click="loadTrashInfo"
        >
          Refresh
        </button>
      </div>
    </div>

    <div v-if="trashError" class="error-message">{{ trashError }}</div>
    <div v-if="result" class="success-message">{{ result }}</div>

    <div v-if="trashLoading" class="loading-state">
      <span class="spinner"></span>
      <span>Loading trash info...</span>
    </div>

    <template v-else-if="trashInfo">
      <div class="card trash-card">
        <div class="trash-stats">
          <div class="stat-block">
            <span class="stat-value-large">
              {{ formatSize(trashInfo.size) }}
            </span>
            <span class="stat-label">Total Size</span>
          </div>
          <div class="stat-divider"></div>
          <div class="stat-block">
            <span class="stat-value-large">
              {{ trashInfo.item_count.toLocaleString() }}
            </span>
            <span class="stat-label">Items</span>
          </div>
        </div>
      </div>

      <div class="card action-card">
        <template v-if="!confirmEmpty">
          <div class="action-row">
            <div>
              <h3>Empty Trash</h3>
              <p class="text-muted">
                Permanently remove all items from the trash
              </p>
            </div>
            <button
              class="btn-danger"
              :disabled="emptying || trashInfo.item_count === 0"
              @click="requestEmpty"
            >
              Empty Trash
            </button>
          </div>
        </template>

        <template v-else>
          <div class="confirm-block">
            <p class="confirm-text">
              Are you sure you want to permanently delete
              <strong>
                {{ trashInfo.item_count.toLocaleString() }} item(s)
              </strong>
              ({{ formatSize(trashInfo.size) }})? This cannot be undone.
            </p>
            <div class="confirm-actions">
              <button class="btn-secondary" @click="cancelEmpty">
                Cancel
              </button>
              <button
                class="btn-danger"
                :disabled="emptying"
                @click="handleEmptyTrash"
              >
                <span v-if="emptying" class="spinner spinner-sm"></span>
                {{ emptying ? "Emptying..." : "Yes, Empty Trash" }}
              </button>
            </div>
          </div>
        </template>
      </div>
    </template>
  </div>
</template>

<style scoped>
.trash-view {
  max-width: 1440px;
}

.trash-card {
  margin-bottom: var(--sp-4);
}

.trash-stats {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 48px;
  padding: var(--sp-4) 0;
}

.stat-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--sp-1);
}

.stat-value-large {
  font-size: 32px;
  font-weight: 700;
  color: var(--text);
  letter-spacing: -0.5px;
}

.stat-label {
  font-size: 13px;
  color: var(--muted);
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.stat-divider {
  width: 1px;
  height: 48px;
  background: var(--border);
}

.action-card {
  margin-bottom: var(--sp-4);
}

.action-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.action-row h3 {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 2px;
}

.action-row p {
  font-size: 13px;
}

.confirm-block {
  text-align: center;
  padding: var(--sp-2) 0;
}

.confirm-text {
  font-size: 14px;
  color: var(--text);
  margin-bottom: var(--sp-4);
  line-height: 1.6;
}

.confirm-actions {
  display: flex;
  justify-content: center;
  gap: 10px;
}

.confirm-actions .btn-danger {
  display: flex;
  align-items: center;
  gap: 6px;
}
</style>
