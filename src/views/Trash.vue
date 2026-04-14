<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { formatSize } from "../utils";
import { showToast } from "../stores/toastStore";
import {
  trashInfo,
  trashLoading,
  trashError,
  loadTrashInfo,
  emptyTrash as storeEmptyTrash,
} from "../stores/scanStore";
import LoadingState from "../components/LoadingState.vue";
import ViewHeader from "../components/ViewHeader.vue";
import ConfirmPanel from "../components/ConfirmPanel.vue";

const emptying = ref(false);
const confirmEmpty = ref(false);

async function handleEmptyTrash() {
  emptying.value = true;
  confirmEmpty.value = false;
  try {
    const cleanResult = await storeEmptyTrash();
    if (cleanResult.success) {
      showToast(`Trash emptied, freed ${formatSize(cleanResult.freed_bytes)}`, "success");
    }
    if (cleanResult.errors.length > 0) {
      showToast(cleanResult.errors.join("; "), "error");
    }
  } catch (e) {
    showToast(String(e), "error");
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

watch(trashError, (err) => {
  if (err) showToast(err, "error");
});
</script>

<template>
  <section class="trash-view">
    <ViewHeader title="Trash" subtitle="View and empty system trash">
      <template #actions>
        <button
          class="btn-secondary"
          :disabled="trashLoading"
          @click="loadTrashInfo"
        >
          Refresh
        </button>
      </template>
    </ViewHeader>

    <LoadingState v-if="trashLoading" message="Loading trash info..." />

    <template v-else-if="trashInfo">
      <div class="card trash-card">
        <dl class="trash-stats">
          <div class="stat-block">
            <dd class="stat-value-large">
              {{ formatSize(trashInfo.size) }}
            </dd>
            <dt class="stat-label">Total Size</dt>
          </div>
          <div class="stat-divider"></div>
          <div class="stat-block">
            <dd class="stat-value-large">
              {{ trashInfo.item_count.toLocaleString() }}
            </dd>
            <dt class="stat-label">Items</dt>
          </div>
        </dl>
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
          <ConfirmPanel
            confirm-label="Yes, Empty Trash"
            loading-label="Emptying..."
            :loading="emptying"
            danger
            @confirm="handleEmptyTrash"
            @cancel="cancelEmpty"
          >
            Are you sure you want to permanently delete
            <strong>{{ trashInfo.item_count.toLocaleString() }} item(s)</strong>
            ({{ formatSize(trashInfo.size) }})? This cannot be undone.
          </ConfirmPanel>
        </template>
      </div>
    </template>
  </section>
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
  margin: 0;
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
  margin: 0;
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
</style>
