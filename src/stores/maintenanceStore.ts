/**
 * Maintenance tasks store.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { MaintenanceTask, MaintenanceResult } from "../types";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export const maintenanceTasks = ref<MaintenanceTask[]>([]);
export const maintenanceLoaded = ref(false);

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

export async function loadMaintenanceTasks() {
  try {
    const result = await invoke<{ tasks: MaintenanceTask[] }>(
      "get_maintenance_tasks"
    );
    maintenanceTasks.value = result.tasks;
    maintenanceLoaded.value = true;
  } catch (e) {
    // non-critical
  }
}

export async function runMaintenanceTask(
  taskId: string
): Promise<MaintenanceResult> {
  const idx = maintenanceTasks.value.findIndex((t) => t.id === taskId);
  if (idx !== -1) {
    maintenanceTasks.value[idx].status = "running";
    maintenanceTasks.value[idx].message = "";
  }

  try {
    const result = await invoke<MaintenanceResult>("run_maintenance_task", {
      taskId,
    });
    if (idx !== -1) {
      maintenanceTasks.value[idx].status = result.success
        ? "success"
        : "error";
      maintenanceTasks.value[idx].message = result.message;
    }
    return result;
  } catch (e) {
    if (idx !== -1) {
      maintenanceTasks.value[idx].status = "error";
      maintenanceTasks.value[idx].message = String(e);
    }
    return {
      task_id: taskId,
      success: false,
      message: String(e),
    };
  }
}
