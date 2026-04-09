<script setup lang="ts">
import { ref, onMounted } from "vue";
import { showToast } from "../stores/toastStore";
import {
  dockerInfo,
  dockerLoading,
  dockerError,
  loadDockerInfo,
  cleanDocker,
} from "../stores/scanStore";

const cleaning = ref(false);

async function prune(pruneAll: boolean) {
  cleaning.value = true;
  try {
    const result = await cleanDocker(pruneAll);
    if (result.success) {
      showToast(
        pruneAll
          ? "Docker system pruned (including unused images)"
          : "Docker system pruned",
        "success"
      );
    }
    if (result.errors.length > 0) {
      showToast(result.errors.join("; "), "error");
    }
    // Refresh info after prune
    await loadDockerInfo();
  } catch (e) {
    showToast(String(e), "error");
  } finally {
    cleaning.value = false;
  }
}

onMounted(loadDockerInfo);
</script>

<template>
  <section class="docker-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Docker</h2>
          <p class="text-muted">Manage Docker images and reclaim space</p>
        </div>
        <button
          v-if="dockerInfo?.installed"
          class="btn-secondary"
          :disabled="dockerLoading"
          @click="loadDockerInfo"
        >
          Refresh
        </button>
      </div>
    </div>

    <div v-if="dockerError" class="error-message">{{ dockerError }}</div>

    <div v-if="dockerLoading" class="loading-state">
      <span class="spinner"></span>
      <span>Checking Docker status...</span>
    </div>

    <template v-else-if="dockerInfo">
      <div v-if="!dockerInfo.installed" class="card not-installed">
        <div class="not-installed-content">
          <h3>Docker Not Installed</h3>
          <p class="text-muted">
            Docker was not detected on this system. Install Docker Desktop to
            manage containers and images.
          </p>
        </div>
      </div>

      <div v-else-if="!dockerInfo.running" class="card not-installed">
        <div class="not-installed-content">
          <h3>Docker Daemon Not Running</h3>
          <p class="text-muted">
            The Docker CLI is installed but the daemon is not responding.
            Start your Docker runtime and retry.
          </p>
          <button class="btn-primary mt-4" @click="loadDockerInfo">
            Retry
          </button>
        </div>
      </div>

      <template v-else>
        <div class="card status-card">
          <div class="status-header">
            <div>
              <h3>Status</h3>
              <span class="badge badge-success">Installed</span>
            </div>
            <div class="prune-actions">
              <button
                class="btn-primary"
                :disabled="cleaning"
                @click="prune(false)"
              >
                <span v-if="cleaning" class="spinner-sm"></span>
                {{ cleaning ? "Pruning..." : "Prune" }}
              </button>
              <button
                class="btn-danger"
                :disabled="cleaning"
                @click="prune(true)"
              >
                Prune All
              </button>
            </div>
          </div>
          <p v-if="dockerInfo.total_reclaimable" class="reclaimable">
            Reclaimable space: {{ dockerInfo.total_reclaimable }}
          </p>
        </div>

        <div v-if="dockerInfo.images.length > 0" class="card-flush images-card">
          <h3>Images</h3>
          <div class="table-container">
            <table>
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Type</th>
                  <th class="col-size">Size</th>
                  <th class="col-id">ID</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="image in dockerInfo.images" :key="image.id">
                  <td class="cell-name truncate">{{ image.name }}</td>
                  <td>
                    <span class="badge badge-accent">{{ image.item_type }}</span>
                  </td>
                  <td class="col-size">{{ image.size }}</td>
                  <td class="col-id mono text-muted">{{ image.id }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <div
          v-if="dockerInfo.disk_usage_raw"
          class="card raw-output-card"
        >
          <h3>Disk Usage Details</h3>
          <pre class="raw-output mono">{{ dockerInfo.disk_usage_raw }}</pre>
        </div>
      </template>
    </template>
  </section>
</template>

<style scoped>
.docker-view {
  max-width: 1440px;
}

.not-installed {
  text-align: center;
  padding: 48px var(--sp-6);
}

.not-installed-content h3 {
  font-size: 18px;
  font-weight: 600;
  margin-bottom: var(--sp-2);
}

.not-installed-content p {
  font-size: 14px;
  max-width: 400px;
  margin: 0 auto;
}

.status-card {
  margin-bottom: var(--sp-6);
}

.status-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.status-header h3 {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 6px;
}

.prune-actions {
  display: flex;
  gap: var(--sp-2);
}

.prune-actions .btn-primary,
.prune-actions .btn-danger {
  display: flex;
  align-items: center;
  gap: 6px;
  white-space: nowrap;
}

.reclaimable {
  margin-top: var(--sp-3);
  font-size: 14px;
  color: var(--text-secondary);
  font-weight: 400;
}

.images-card {
  margin-bottom: var(--sp-6);
}

.images-card h3 {
  font-size: 16px;
  font-weight: 600;
  padding: var(--sp-5);
  margin-bottom: 0;
}

.col-id {
  width: 130px;
}

.cell-name {
  max-width: 300px;
  font-weight: 400;
}

.raw-output-card {
  margin-bottom: var(--sp-6);
}

.raw-output-card h3 {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: var(--sp-3);
}

.raw-output {
  background: transparent;
  border-radius: var(--radius-sm);
  padding: var(--sp-4);
  overflow-x: auto;
  white-space: pre;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-secondary);
}

.mt-4 { margin-top: var(--sp-4); }
</style>
