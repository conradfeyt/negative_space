/**
 * System monitoring store — vitals, thermal, memory.
 */
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
  VitalsResult,
  ThermalScanResult,
  MemoryScanResult,
} from "../types";

// ---------------------------------------------------------------------------
// System Vitals
// ---------------------------------------------------------------------------

export const vitalsResult = ref<VitalsResult | null>(null);
export const vitalsScanning = ref(false);
export const vitalsScanned = ref(false);
export const vitalsError = ref("");

export async function scanVitals(live = false) {
  if (vitalsScanning.value) return;
  if (!live) {
    vitalsScanning.value = true;
    vitalsError.value = "";
    vitalsResult.value = null;
    vitalsScanned.value = false;
  }
  try {
    vitalsResult.value = await invoke<VitalsResult>("scan_vitals");
    vitalsScanned.value = true;
  } catch (e) {
    if (!live) vitalsError.value = String(e);
  } finally {
    vitalsScanning.value = false;
  }
}

export async function quitProcess(pid: number): Promise<string> {
  return await invoke<string>("quit_process", { pid });
}

export async function forceQuitProcess(pid: number): Promise<string> {
  return await invoke<string>("force_quit_process", { pid });
}

export async function quitProcessGroup(pids: number[]): Promise<string> {
  return await invoke<string>("quit_process_group", { pids });
}

// ---------------------------------------------------------------------------
// Hardware Thermal Sensors (SMC)
// ---------------------------------------------------------------------------

export const thermalResult = ref<ThermalScanResult | null>(null);
export const thermalScanning = ref(false);
export const thermalScanned = ref(false);
export const thermalError = ref("");

export async function scanThermal(live = false) {
  if (thermalScanning.value) return;
  if (!live) {
    thermalScanning.value = true;
    thermalError.value = "";
    thermalResult.value = null;
    thermalScanned.value = false;
  }
  try {
    thermalResult.value = await invoke<ThermalScanResult>("scan_thermal");
    thermalScanned.value = true;
  } catch (e) {
    if (!live) thermalError.value = String(e);
  } finally {
    thermalScanning.value = false;
  }
}

// ---------------------------------------------------------------------------
// Memory
// ---------------------------------------------------------------------------

export const memoryResult = ref<MemoryScanResult | null>(null);
export const memoryScanning = ref(false);
export const memoryScanned = ref(false);
export const memoryError = ref("");

export async function scanMemory(live = false) {
  if (memoryScanning.value) return;
  if (!live) {
    memoryScanning.value = true;
    memoryError.value = "";
    memoryResult.value = null;
    memoryScanned.value = false;
  }
  try {
    memoryResult.value = await invoke<MemoryScanResult>("scan_memory");
    memoryScanned.value = true;
  } catch (e) {
    if (!live) memoryError.value = String(e);
  } finally {
    memoryScanning.value = false;
  }
}
