<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { formatSize } from "../utils";
import {
  logs,
  logsScanning,
  logsScanned,
  logsError,
  scanLogs,
  deleteFiles,
  totalLogSize,
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
  await scanLogs();
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
      successMsg.value = `Cleaned ${result.deleted_count} log(s), freed ${formatSize(result.freed_bytes)}`;
      logs.value = logs.value.filter((l) => !selected.value.has(l.path));
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
  else selected.value = new Set(logs.value.map((l) => l.path));
}

const allSelected = computed(() => logs.value.length > 0 && selected.value.size === logs.value.length);
const totalSelected = computed(() => logs.value.filter((l) => selected.value.has(l.path)).reduce((sum, l) => sum + l.size, 0));
</script>

<template>
  <div class="logs-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Logs</h2>
          <p class="text-muted">System and application log files</p>
        </div>
        <button class="btn-primary scan-btn" :disabled="logsScanning" @click="scan">
          <span v-if="logsScanning" class="spinner-sm"></span>
          {{ logsScanning ? "Scanning..." : "Scan" }}
        </button>
      </div>
    </div>

    <div v-if="hasFullDiskAccess === false" class="fda-warning-banner">
      <span class="fda-warning-dot"></span>
      <div class="fda-warning-body">
        <div class="fda-warning-title">Limited scan -- Full Disk Access required</div>
        <div class="fda-warning-text">
          Without Full Disk Access, only /var/log is scanned.
          ~/Library/Logs is skipped.
        </div>
        <div class="fda-warning-actions">
          <button class="btn-fda btn-fda-primary" @click="openFdaSettings">Open System Settings</button>
          <button class="btn-fda btn-fda-secondary" @click="recheckFda">Re-check</button>
        </div>
      </div>
    </div>

    <div v-if="logsError" class="error-message">{{ logsError }}</div>
    <div v-if="deleteError" class="error-message">{{ deleteError }}</div>
    <div v-if="successMsg" class="success-message">{{ successMsg }}</div>

    <div v-if="logsScanning" class="loading-state">
      <span class="spinner"></span>
      <span>Scanning log files...</span>
    </div>

    <div v-else-if="logsScanned && logs.length === 0" class="card empty-state">
      <p>No log files found</p>
    </div>

    <template v-else-if="logs.length > 0">
      <div class="card-flush">
        <div class="results-header">
          <span class="results-count">
            {{ logs.length }} log(s) -- {{ formatSize(totalLogSize) }} total
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

        <div class="table-container">
          <table>
            <thead>
              <tr>
                <th class="col-check"><input type="checkbox" :checked="allSelected" @change="toggleAll" /></th>
                <th>Name</th>
                <th>Path</th>
                <th class="col-size">Size</th>
                <th class="col-date">Modified</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="log in logs" :key="log.path">
                <td class="col-check"><input type="checkbox" :checked="selected.has(log.path)" @change="toggleSelect(log.path)" /></td>
                <td class="cell-name truncate">{{ log.name }}</td>
                <td class="cell-path mono truncate" :title="log.path">{{ log.path }}</td>
                <td class="col-size mono">{{ formatSize(log.size) }}</td>
                <td class="col-date text-muted">{{ log.modified }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.logs-view { max-width: 1440px; }
.cell-name { max-width: 200px; font-weight: 400; }
.cell-path { max-width: 280px; }
</style>
