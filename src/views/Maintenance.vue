<script setup lang="ts">
import { ref, onMounted } from "vue";
import {
  maintenanceTasks,
  maintenanceLoaded,
  loadMaintenanceTasks,
  runMaintenanceTask,
} from "../stores/scanStore";

// Track which task detail panels are expanded
const expandedTasks = ref<Set<string>>(new Set());

onMounted(async () => {
  if (!maintenanceLoaded.value) {
    await loadMaintenanceTasks();
  }
});

async function runTask(taskId: string) {
  await runMaintenanceTask(taskId);
}

function toggleDetails(taskId: string) {
  const next = new Set(expandedTasks.value);
  if (next.has(taskId)) {
    next.delete(taskId);
  } else {
    next.add(taskId);
  }
  expandedTasks.value = next;
}

function statusLabel(status: string): string {
  switch (status) {
    case "running":
      return "Running...";
    case "success":
      return "Done";
    case "error":
      return "Failed";
    default:
      return "Run";
  }
}
</script>

<template>
  <section class="maintenance-view">
    <div class="view-header">
      <div>
        <h2>System Maintenance</h2>
        <p class="text-muted">
          macOS housekeeping tasks -- review what each task touches before running
        </p>
      </div>
    </div>

    <div class="info-banner">
      <span class="info-dot"></span>
      <div class="info-body">
        <div class="info-title">About these tasks</div>
        <div class="info-text">
          Each task shows exactly which commands will run, which services are
          affected, and which files or paths are touched. Expand "What this
          touches" to review before running. Tasks marked "Admin" will prompt
          for your system password via macOS's native dialog.
        </div>
      </div>
    </div>

    <div v-if="!maintenanceLoaded" class="loading-state">
      <span class="spinner"></span>
      <span>Loading tasks...</span>
    </div>

    <div v-else class="task-list">
      <div
        v-for="task in maintenanceTasks"
        :key="task.id"
        :class="['card-flush', 'task-card', `status-${task.status}`]"
      >
        <!-- Task header: name, badges, description, run button -->
        <div class="task-main">
          <div class="task-info">
            <div class="task-name-row">
              <span class="task-name">{{ task.name }}</span>
              <span
                v-if="task.requires_admin"
                class="badge-pill badge-warning"
                title="Requires administrator password"
              >
                Admin
              </span>
              <span
                v-if="task.destructive"
                class="badge-pill badge-danger"
                title="Deletes or modifies data"
              >
                Modifies data
              </span>
              <span
                v-else
                class="badge-pill badge-success"
              >
                No data changed
              </span>
              <span
                v-if="task.reversible_info"
                class="badge-pill badge-accent"
              >
                Reversible
              </span>
              <span
                v-else
                class="badge-pill badge-neutral"
              >
                Not reversible
              </span>
            </div>
            <p class="task-description text-muted">
              {{ task.description }}
            </p>
            <p v-if="task.warning" class="task-warning">
              {{ task.warning }}
            </p>
          </div>
          <div class="task-action">
            <button
              :class="[
                'task-btn',
                task.status === 'success'
                  ? 'btn-task-success'
                  : task.status === 'error'
                    ? 'btn-task-error'
                    : 'btn-primary',
              ]"
              :disabled="task.status === 'running'"
              @click="runTask(task.id)"
            >
              <span
                v-if="task.status === 'running'"
                class="spinner-sm"
              ></span>
              {{ statusLabel(task.status) }}
            </button>
          </div>
        </div>

        <!-- Expandable details toggle -->
        <div
          class="details-toggle"
          @click="toggleDetails(task.id)"
        >
          <span
            class="expand-chevron"
            :class="{ expanded: expandedTasks.has(task.id) }"
          ><svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg></span>
          <span class="details-toggle-label">What this touches</span>
        </div>

        <!-- Expanded details: commands, services, paths -->
        <div v-if="expandedTasks.has(task.id)" class="details-panel">
          <!-- Commands -->
          <div class="detail-section">
            <div class="detail-section-label">Commands executed</div>
            <div class="detail-items">
              <code
                v-for="(cmd, idx) in task.commands"
                :key="idx"
                class="detail-command"
              >{{ cmd }}</code>
            </div>
          </div>

          <!-- Services affected -->
          <div class="detail-section">
            <div class="detail-section-label">Services affected</div>
            <ul class="detail-list">
              <li
                v-for="(svc, idx) in task.services_affected"
                :key="idx"
              >{{ svc }}</li>
            </ul>
          </div>

          <!-- Paths affected -->
          <div class="detail-section">
            <div class="detail-section-label">Paths / data affected</div>
            <ul class="detail-list">
              <li
                v-for="(p, idx) in task.paths_affected"
                :key="idx"
              >{{ p }}</li>
            </ul>
          </div>

          <!-- How this reverses -->
          <div v-if="task.reversible_info" class="detail-section">
            <div class="detail-section-label">How this reverses</div>
            <p class="detail-reversible-text">{{ task.reversible_info }}</p>
          </div>
          <div v-else class="detail-section">
            <div class="detail-section-label">Reversibility</div>
            <p class="detail-not-reversible-text">
              This action cannot be undone. The removed data will not be
              automatically recreated by macOS.
            </p>
          </div>
        </div>

        <!-- Result message -->
        <div
          v-if="task.message"
          :class="[
            'task-result',
            task.status === 'success'
              ? 'task-result-success'
              : 'task-result-error',
          ]"
        >
          {{ task.message }}
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.maintenance-view {
  max-width: 1440px;
}

/* Info banner */
.info-banner {
  display: flex;
  align-items: flex-start;
  gap: var(--sp-3);
  padding: var(--sp-4) var(--sp-5);
  border-radius: var(--radius-md);
  background: var(--accent-light);
  border: 1px solid rgba(0, 180, 216, 0.1);
  margin-bottom: var(--sp-6);
}

.info-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--accent);
  flex-shrink: 0;
  margin-top: 5px;
}

.info-body {
  flex: 1;
}

.info-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  margin-bottom: var(--sp-1);
}

.info-text {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.5;
}

/* Task list */
.task-list {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}

.task-card {
  transition: border-color 0.2s;
}

.task-card.status-success {
  border: 1px solid rgba(48, 209, 88, 0.12);
}

.task-card.status-error {
  border: 1px solid rgba(255, 69, 58, 0.12);
}

.task-main {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: var(--sp-5) var(--sp-5) var(--sp-3);
  gap: var(--sp-5);
}

.task-info {
  flex: 1;
  min-width: 0;
}

.task-name-row {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  margin-bottom: var(--sp-2);
  flex-wrap: wrap;
}

.task-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}

.task-description {
  font-size: 13px;
  line-height: 1.5;
  margin-bottom: 0;
}

.task-warning {
  font-size: 12px;
  color: var(--warning);
  margin-top: var(--sp-2);
  font-weight: 500;
  line-height: 1.4;
}

.task-action {
  flex-shrink: 0;
}

.task-btn {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  padding: var(--sp-2) var(--sp-5);
  white-space: nowrap;
  min-width: 100px;
  justify-content: center;
}

.btn-task-success {
  background: var(--success);
  color: white;
}

.btn-task-success:hover {
  opacity: 0.9;
}

.btn-task-error {
  background: var(--danger);
  color: white;
}

.btn-task-error:hover {
  background: var(--danger-hover);
}

/* Details toggle */
.details-toggle {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  padding: var(--sp-2) var(--sp-5) var(--sp-3);
  cursor: pointer;
  user-select: none;
}

.details-toggle:hover .details-toggle-label {
  color: var(--accent);
}

.details-toggle-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--muted);
  transition: color 0.15s;
}

/* Details panel */
.details-panel {
  border-top: 1px solid var(--border-divider);
  padding: var(--sp-4) var(--sp-5);
  background: transparent;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.detail-section {
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
}

.detail-section-label {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
}

.detail-items {
  display: flex;
  flex-direction: column;
  gap: var(--sp-1);
}

.detail-command {
  display: block;
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.5;
  color: var(--text);
  background: var(--border-subtle);
  padding: var(--sp-2) var(--sp-3);
  border-radius: var(--radius-sm);
  word-break: break-all;
}

.detail-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.detail-list li {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.5;
  padding: 2px 0 2px 14px;
  position: relative;
}

.detail-list li::before {
  content: "";
  position: absolute;
  left: 0;
  top: 8px;
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--border);
}

.detail-reversible-text {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.6;
  margin: 0;
  padding: var(--sp-2) var(--sp-3);
  background: rgba(0, 180, 216, 0.12);
  border-left: 3px solid var(--accent);
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
}

.detail-not-reversible-text {
  font-size: 12px;
  color: var(--danger);
  line-height: 1.6;
  margin: 0;
  padding: var(--sp-2) var(--sp-3);
  background: rgba(255, 69, 58, 0.10);
  border-left: 3px solid var(--danger);
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
}

/* Result message */
.task-result {
  padding: var(--sp-3) var(--sp-5);
  font-size: 12px;
  line-height: 1.5;
  border-top: 1px solid var(--border-divider);
}

.task-result-success {
  background: var(--success-tint);
  color: var(--success-text);
}

.task-result-error {
  background: var(--danger-tint);
  color: var(--danger);
}

.btn-task-success:active,
.btn-task-error:active {
  transform: scale(0.98);
}
</style>
