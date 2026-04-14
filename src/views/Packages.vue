<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { formatSize } from "../utils";
import { showToast } from "../stores/toastStore";
import {
  packagesResult,
  packagesScanning,
  packagesScanned,
  packagesError,
  scanPackages,
  customProbes,
  loadCustomProbes,
  saveCustomProbes,
  deleteCustomProbe,
  testProbeCommand,
} from "../stores/scanStore";
import type { CustomProbe, CommandRecord } from "../types";
import StatCard from "../components/StatCard.vue";
import EmptyState from "../components/EmptyState.vue";
import LoadingState from "../components/LoadingState.vue";
import CollapsibleSection from "../components/CollapsibleSection.vue";
import ViewHeader from "../components/ViewHeader.vue";
import Modal from "../components/Modal.vue";
import AppSelect from "../components/AppSelect.vue";

const expandedManagers = ref<Set<string>>(new Set());
const expandedRuntimes = ref<Set<string>>(new Set());
const showDeps = ref<Set<string>>(new Set());
const showCommands = ref<Set<string>>(new Set());

async function scan() {
  expandedManagers.value = new Set();
  expandedRuntimes.value = new Set();
  showDeps.value = new Set();
  showCommands.value = new Set();
  await scanPackages();
}

function toggleManager(id: string) {
  const next = new Set(expandedManagers.value);
  next.has(id) ? next.delete(id) : next.add(id);
  expandedManagers.value = next;
}

function toggleRuntime(id: string) {
  const next = new Set(expandedRuntimes.value);
  next.has(id) ? next.delete(id) : next.add(id);
  expandedRuntimes.value = next;
}

function toggleDeps(pkgKey: string) {
  const next = new Set(showDeps.value);
  next.has(pkgKey) ? next.delete(pkgKey) : next.add(pkgKey);
  showDeps.value = next;
}

function toggleCommands(id: string) {
  const next = new Set(showCommands.value);
  next.has(id) ? next.delete(id) : next.add(id);
  showCommands.value = next;
}

function visiblePackages(managerId: string) {
  const mgr = packagesResult.value?.managers.find((m) => m.id === managerId);
  if (!mgr) return [];
  return mgr.packages;
}

function formatCmd(rec: CommandRecord): string {
  return [rec.program, ...rec.args].join(" ");
}

const totalManagerSize = computed(() =>
  packagesResult.value?.managers.reduce((s, m) => s + m.total_size, 0) ?? 0
);

const totalRuntimeSize = computed(() =>
  packagesResult.value?.runtimes.reduce((s, r) => s + r.total_size, 0) ?? 0
);

watch(packagesError, (err) => {
  if (err) showToast(err, "error");
});

// ---------------------------------------------------------------------------
// Custom probe editor
// ---------------------------------------------------------------------------

const showProbeModal = ref(false);
const editingProbeId = ref<string | null>(null);
const probeTestResult = ref<CommandRecord | null>(null);
const probeTesting = ref(false);
const probeSaving = ref(false);

function emptyProbe(): CustomProbe {
  return {
    id: crypto.randomUUID(),
    name: "",
    probe_type: "manager",
    enabled: true,
    detect: { program: "", args: [] },
    version: null,
    list_packages: null,
    list_parse_mode: "none",
    size_paths: [],
    install_path: "",
    uninstall_hint: "",
  };
}

const probeForm = ref<CustomProbe>(emptyProbe());
const probeDetectArgs = ref("");
const probeVersionProgram = ref("");
const probeVersionArgs = ref("");
const probeListProgram = ref("");
const probeListArgs = ref("");
const probeSizePaths = ref("");

function openNewProbe() {
  editingProbeId.value = null;
  probeForm.value = emptyProbe();
  syncFormToFields();
  probeTestResult.value = null;
  showProbeModal.value = true;
}

function openEditProbe(probe: CustomProbe) {
  editingProbeId.value = probe.id;
  probeForm.value = JSON.parse(JSON.stringify(probe));
  syncFormToFields();
  probeTestResult.value = null;
  showProbeModal.value = true;
}

function syncFormToFields() {
  const f = probeForm.value;
  probeDetectArgs.value = f.detect.args.join(" ");
  probeVersionProgram.value = f.version?.program ?? "";
  probeVersionArgs.value = f.version?.args.join(" ") ?? "";
  probeListProgram.value = f.list_packages?.program ?? "";
  probeListArgs.value = f.list_packages?.args.join(" ") ?? "";
  probeSizePaths.value = f.size_paths.join(", ");
}

function splitArgs(s: string): string[] {
  return s.trim().split(/\s+/).filter(Boolean);
}

function collectProbeFromForm(): CustomProbe {
  const f = probeForm.value;
  const probe: CustomProbe = {
    ...f,
    detect: { program: f.detect.program, args: splitArgs(probeDetectArgs.value) },
    version: probeVersionProgram.value
      ? { program: probeVersionProgram.value, args: splitArgs(probeVersionArgs.value) }
      : null,
    list_packages: probeListProgram.value
      ? { program: probeListProgram.value, args: splitArgs(probeListArgs.value) }
      : null,
    size_paths: probeSizePaths.value
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean),
  };
  return probe;
}

async function testDetect() {
  const probe = collectProbeFromForm();
  probeTesting.value = true;
  probeTestResult.value = null;
  try {
    probeTestResult.value = await testProbeCommand(
      probe.detect.program,
      probe.detect.args
    );
  } catch (e) {
    probeTestResult.value = {
      program: probe.detect.program,
      args: probe.detect.args,
      purpose: "test",
      success: false,
      duration_ms: 0,
      output_preview: String(e),
    };
  } finally {
    probeTesting.value = false;
  }
}

async function saveProbe() {
  const probe = collectProbeFromForm();
  if (!probe.name.trim()) {
    showToast("Probe name is required", "error");
    return;
  }
  if (!probe.detect.program.trim()) {
    showToast("Detection command is required", "error");
    return;
  }

  probeSaving.value = true;
  try {
    const existing = [...customProbes.value];
    const idx = existing.findIndex((p) => p.id === probe.id);
    if (idx >= 0) {
      existing[idx] = probe;
    } else {
      existing.push(probe);
    }
    await saveCustomProbes(existing);
    showProbeModal.value = false;
    showToast(`Saved custom probe "${probe.name}"`, "success");
  } catch (e) {
    showToast(String(e), "error");
  } finally {
    probeSaving.value = false;
  }
}

async function removeProbe(id: string) {
  try {
    await deleteCustomProbe(id);
    showToast("Custom probe deleted", "success");
  } catch (e) {
    showToast(String(e), "error");
  }
}

function findCustomProbe(resultId: string): CustomProbe | undefined {
  const bareId = resultId.replace(/^custom-/, "");
  return customProbes.value.find((p) => p.id === bareId);
}

onMounted(() => {
  loadCustomProbes();
});
</script>

<template>
  <section class="packages-view">
    <ViewHeader
      title="Packages"
      subtitle="Package managers, installed packages, and language runtimes"
    >
      <template #actions>
        <button class="btn-secondary" @click="openNewProbe">+ Custom Probe</button>
        <button
          class="btn-primary scan-btn"
          :disabled="packagesScanning"
          @click="scan"
        >
          <span v-if="packagesScanning" class="spinner-sm"></span>
          {{ packagesScanning ? "Scanning..." : "Scan" }}
        </button>
      </template>
    </ViewHeader>

    <LoadingState v-if="packagesScanning" message="Detecting package managers and runtimes..." />

    <div v-else-if="packagesScanned && packagesResult">
      <!-- Summary -->
      <div class="stats-row">
        <StatCard :value="String(packagesResult.managers.length)" label="Package managers" />
        <StatCard :value="String(packagesResult.runtimes.length)" label="Runtimes" />
        <StatCard :value="formatSize(packagesResult.total_size)" label="Total disk usage" />
      </div>

      <!-- Package Managers -->
      <div v-if="packagesResult.managers.length > 0" class="section">
        <div class="section-header">
          <h3 class="section-title">Package Managers</h3>
          <span class="section-size mono text-muted">{{ formatSize(totalManagerSize) }}</span>
        </div>

        <div v-for="mgr in packagesResult.managers" :key="mgr.id" class="card-flush manager-card">
          <CollapsibleSection
            :expanded="expandedManagers.has(mgr.id)"
            @toggle="toggleManager(mgr.id)"
          >
            <template #header>
              <div class="manager-title">
                <span class="manager-name">{{ mgr.name }}</span>
                <span v-if="mgr.is_custom" class="badge pill badge-info">custom</span>
                <span class="text-muted manager-version">{{ mgr.version }}</span>
              </div>
              <div class="manager-stats">
                <span class="manager-count text-muted">
                  {{ mgr.packages.filter(p => p.is_top_level).length }} packages
                  <span v-if="mgr.total_package_count > mgr.packages.filter(p => p.is_top_level).length">
                    ({{ mgr.total_package_count }} with deps)
                  </span>
                </span>
                <span class="manager-size mono">{{ formatSize(mgr.total_size) }}</span>
              </div>
            </template>

            <div class="manager-body">
            <div class="manager-path mono text-muted">{{ mgr.install_path }}</div>

            <div class="pkg-list">
              <div
                v-for="pkg in visiblePackages(mgr.id)"
                :key="mgr.id + '/' + pkg.name"
                class="pkg-item"
              >
                <div class="pkg-row">
                  <div class="pkg-info">
                    <span class="pkg-name">{{ pkg.name }}</span>
                    <span class="pkg-version mono text-muted">{{ pkg.version }}</span>
                    <span v-if="!pkg.is_top_level" class="badge pill badge-neutral">dep</span>
                  </div>
                  <div class="pkg-right">
                    <span v-if="pkg.size > 0" class="pkg-size mono">{{ formatSize(pkg.size) }}</span>
                    <button
                      v-if="pkg.dependencies.length > 0"
                      class="badge pill badge-accent deps-toggle"
                      @click.stop="toggleDeps(mgr.id + '/' + pkg.name)"
                    >
                      {{ showDeps.has(mgr.id + '/' + pkg.name) ? 'hide deps' : pkg.dependencies.length + ' deps' }}
                    </button>
                  </div>
                </div>

                <div
                  v-if="showDeps.has(mgr.id + '/' + pkg.name) && pkg.dependencies.length > 0"
                  class="pkg-deps"
                >
                  <span
                    v-for="dep in pkg.dependencies"
                    :key="dep"
                    class="dep-tag mono"
                  >{{ dep }}</span>
                </div>

                <div v-if="showDeps.has(mgr.id + '/' + pkg.name)" class="pkg-removal">
                  <div class="removal-cmd">
                    <span class="removal-label">Remove:</span>
                    <code class="mono">{{ pkg.uninstall_command }}</code>
                  </div>
                  <div v-if="pkg.removal_warning" class="removal-warning">
                    {{ pkg.removal_warning }}
                  </div>
                </div>
              </div>
            </div>

            <!-- Manager-level uninstall hint -->
            <div class="manager-hint">
              <span class="removal-label">Manager:</span>
              <span class="text-muted">{{ mgr.uninstall_hint }}</span>
            </div>

            <!-- Command transparency -->
            <div v-if="mgr.commands_run && mgr.commands_run.length > 0" class="commands-section">
              <button class="commands-toggle" @click.stop="toggleCommands(mgr.id)">
                {{ showCommands.has(mgr.id) ? '▾' : '▸' }}
                Commands run ({{ mgr.commands_run.length }})
              </button>
              <div v-if="showCommands.has(mgr.id)" class="commands-list">
                <div v-for="(cmd, i) in mgr.commands_run" :key="i" class="cmd-row">
                  <span class="cmd-status" :class="cmd.success ? 'ok' : 'fail'">{{ cmd.success ? '✓' : '✗' }}</span>
                  <code class="cmd-text mono">{{ formatCmd(cmd) }}</code>
                  <span class="cmd-time mono text-muted">{{ cmd.duration_ms }}ms</span>
                </div>
              </div>
            </div>

            <!-- Edit/delete for custom probes -->
            <div v-if="mgr.is_custom" class="custom-probe-actions">
              <button class="btn-ghost" @click="openEditProbe(findCustomProbe(mgr.id)!)">Edit probe</button>
              <button class="btn-ghost danger" @click="removeProbe(mgr.id.replace('custom-', ''))">Delete probe</button>
            </div>
            </div>
          </CollapsibleSection>
        </div>
      </div>

      <!-- Runtimes -->
      <div v-if="packagesResult.runtimes.length > 0" class="section">
        <div class="section-header">
          <h3 class="section-title">Runtimes</h3>
          <span class="section-size mono text-muted">{{ formatSize(totalRuntimeSize) }}</span>
        </div>

        <div v-for="rt in packagesResult.runtimes" :key="rt.id" class="card-flush runtime-card">
          <CollapsibleSection
            :expanded="expandedRuntimes.has(rt.id)"
            @toggle="toggleRuntime(rt.id)"
          >
            <template #header>
              <div class="runtime-title">
                <span class="runtime-name">{{ rt.name }}</span>
                <span v-if="rt.is_custom" class="badge pill badge-info">custom</span>
                <span class="badge source pill" :class="rt.install_method">
                  {{ rt.install_method }}
                </span>
              </div>
              <div class="runtime-stats">
                <span class="text-muted">{{ rt.versions.length }} version{{ rt.versions.length !== 1 ? 's' : '' }}</span>
                <span class="runtime-size mono">{{ formatSize(rt.total_size) }}</span>
              </div>
            </template>

            <div class="runtime-body">
            <div class="runtime-path mono text-muted">{{ rt.install_path }}</div>

            <div class="version-list">
              <div
                v-for="ver in rt.versions"
                :key="ver.path"
                class="version-item"
              >
                <div class="version-info">
                  <span class="version-name">{{ ver.version }}</span>
                  <span v-if="ver.active" class="badge pill badge-accent">active</span>
                </div>
                <div class="version-right">
                  <span v-if="ver.size > 0" class="version-size mono">{{ formatSize(ver.size) }}</span>
                </div>
              </div>
            </div>

            <div class="runtime-removal">
              <div class="removal-cmd">
                <span class="removal-label">Remove:</span>
                <span class="text-muted">{{ rt.uninstall_hint }}</span>
              </div>
              <div v-if="rt.removal_warning" class="removal-warning">
                {{ rt.removal_warning }}
              </div>
            </div>

            <!-- Command transparency -->
            <div v-if="rt.commands_run && rt.commands_run.length > 0" class="commands-section">
              <button class="commands-toggle" @click.stop="toggleCommands(rt.id)">
                {{ showCommands.has(rt.id) ? '▾' : '▸' }}
                Commands run ({{ rt.commands_run.length }})
              </button>
              <div v-if="showCommands.has(rt.id)" class="commands-list">
                <div v-for="(cmd, i) in rt.commands_run" :key="i" class="cmd-row">
                  <span class="cmd-status" :class="cmd.success ? 'ok' : 'fail'">{{ cmd.success ? '✓' : '✗' }}</span>
                  <code class="cmd-text mono">{{ formatCmd(cmd) }}</code>
                  <span class="cmd-time mono text-muted">{{ cmd.duration_ms }}ms</span>
                </div>
              </div>
            </div>

            <!-- Edit/delete for custom probes -->
            <div v-if="rt.is_custom" class="custom-probe-actions">
              <button class="btn-ghost" @click="openEditProbe(findCustomProbe(rt.id)!)">Edit probe</button>
              <button class="btn-ghost danger" @click="removeProbe(rt.id.replace('custom-', ''))">Delete probe</button>
            </div>
            </div>
          </CollapsibleSection>
        </div>
      </div>

      <!-- Empty state -->
      <EmptyState
        v-if="packagesResult.managers.length === 0 && packagesResult.runtimes.length === 0"
        title="No package managers or runtimes detected"
        description="Homebrew, npm, pip, Cargo, and other package managers will appear here if installed."
      />
    </div>

    <!-- Custom probe editor modal -->
    <Modal
      :visible="showProbeModal"
      :title="editingProbeId ? 'Edit Custom Probe' : 'New Custom Probe'"
      wide
      @close="showProbeModal = false"
    >
      <div class="probe-form">
        <!-- Row 1: Name + Type -->
        <div class="form-row">
          <div class="form-group flex-1">
            <label class="form-label">Name</label>
            <input v-model="probeForm.name" class="form-input" placeholder="e.g. Poetry, Volta, SDKMAN" />
          </div>
          <div class="form-group">
            <label class="form-label">Type</label>
            <div class="radio-group">
              <label class="radio-label">
                <input type="radio" value="manager" v-model="probeForm.probe_type" /> Manager
              </label>
              <label class="radio-label">
                <input type="radio" value="runtime" v-model="probeForm.probe_type" /> Runtime
              </label>
            </div>
          </div>
        </div>

        <!-- Row 2: Detect command -->
        <fieldset class="form-fieldset">
          <legend class="form-legend">Detect <span class="text-muted">(required)</span></legend>
          <div class="form-row">
            <div class="form-group cmd-program">
              <label class="form-label">Program</label>
              <input v-model="probeForm.detect.program" class="form-input mono" placeholder="which" />
            </div>
            <div class="form-group flex-1">
              <label class="form-label">Arguments</label>
              <input v-model="probeDetectArgs" class="form-input mono" placeholder="poetry" />
            </div>
            <button
              class="btn-secondary btn-sm probe-test-btn"
              :disabled="probeTesting || !probeForm.detect.program"
              @click="testDetect"
            >
              {{ probeTesting ? '...' : 'Test' }}
            </button>
          </div>
          <div v-if="probeTestResult" class="test-result" :class="probeTestResult.success ? 'test-ok' : 'test-fail'">
            <span class="cmd-status" :class="probeTestResult.success ? 'ok' : 'fail'">
              {{ probeTestResult.success ? '✓' : '✗' }}
            </span>
            <code class="mono test-output">{{ probeTestResult.output_preview || '(no output)' }}</code>
            <span class="cmd-time mono text-muted">{{ probeTestResult.duration_ms }}ms</span>
          </div>
        </fieldset>

        <!-- Row 3: Version command -->
        <fieldset class="form-fieldset">
          <legend class="form-legend">Version <span class="text-muted">(optional)</span></legend>
          <div class="form-row">
            <div class="form-group cmd-program">
              <input v-model="probeVersionProgram" class="form-input mono" placeholder="poetry" />
            </div>
            <div class="form-group flex-1">
              <input v-model="probeVersionArgs" class="form-input mono" placeholder="--version" />
            </div>
          </div>
        </fieldset>

        <!-- Row 4: List packages (manager only) -->
        <fieldset v-if="probeForm.probe_type === 'manager'" class="form-fieldset">
          <legend class="form-legend">List packages <span class="text-muted">(optional)</span></legend>
          <div class="form-row">
            <div class="form-group cmd-program">
              <input v-model="probeListProgram" class="form-input mono" placeholder="poetry" />
            </div>
            <div class="form-group flex-1">
              <input v-model="probeListArgs" class="form-input mono" placeholder="show --no-dev" />
            </div>
            <div class="form-group cmd-parse">
              <AppSelect
                v-model="probeForm.list_parse_mode"
                :options="[
                  { value: 'none', label: 'Don\'t parse' },
                  { value: 'lines', label: 'Lines (name ver)' },
                  { value: 'json', label: 'JSON [{name, version}]' },
                ]"
              />
            </div>
          </div>
        </fieldset>

        <!-- Row 4: Paths -->
        <div class="form-row">
          <div class="form-group flex-1">
            <label class="form-label">Size paths <span class="text-muted">(comma-separated, ~ ok)</span></label>
            <input v-model="probeSizePaths" class="form-input mono" placeholder="~/.poetry, ~/Library/Caches/pypoetry" />
          </div>
          <div class="form-group" style="width: 200px; flex-shrink: 0;">
            <label class="form-label">Install path</label>
            <input v-model="probeForm.install_path" class="form-input mono" placeholder="~/.poetry" />
          </div>
        </div>

        <!-- Row 5: Uninstall hint -->
        <div class="form-group">
          <label class="form-label">Uninstall hint</label>
          <input v-model="probeForm.uninstall_hint" class="form-input" placeholder="curl -sSL ... | python3 - --uninstall" />
        </div>
      </div>

      <template #actions>
        <button class="btn-secondary" @click="showProbeModal = false">Cancel</button>
        <button class="btn-primary" :disabled="probeSaving" @click="saveProbe">
          {{ probeSaving ? 'Saving...' : editingProbeId ? 'Update' : 'Save' }}
        </button>
      </template>
    </Modal>
  </section>
</template>

<style scoped>
.packages-view {
  max-width: 1440px;
}

/* Stats row */
.stats-row {
  display: flex;
  gap: 8px;
  margin-bottom: var(--sp-6);
  flex-wrap: wrap;
}

/* Sections */
.section {
  margin-bottom: var(--sp-8);
}

.section-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: var(--sp-3);
}

.section-header .section-title {
  margin-bottom: 0;
}

.section-size {
  font-size: 13px;
}

/* Manager cards */
.manager-card,
.runtime-card {
  margin-bottom: var(--sp-2);
}

.manager-card :deep(.collapsible-header),
.runtime-card :deep(.collapsible-header) {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: var(--sp-3) var(--sp-4);
  cursor: pointer;
  transition: background 0.15s;
}

.manager-card :deep(.collapsible-header:hover),
.runtime-card :deep(.collapsible-header:hover) {
  background: var(--surface-alt);
}

.manager-title,
.runtime-title {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
}

.manager-name,
.runtime-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.manager-version {
  font-size: 12px;
}

.manager-stats,
.runtime-stats {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
  font-size: 13px;
}

.manager-size,
.runtime-size {
  font-weight: 600;
  color: var(--text);
  min-width: 64px;
  text-align: right;
}

.manager-count {
  font-size: 12px;
}

/* Manager body */
.manager-body,
.runtime-body {
  border-top: 1px solid var(--border-divider);
  padding: var(--sp-3) var(--sp-4) var(--sp-4);
  animation: slideDown 0.15s ease;
}

.manager-path,
.runtime-path {
  font-size: 11px;
  margin-bottom: var(--sp-3);
  padding: 4px 8px;
  background: rgba(0, 0, 0, 0.03);
  border-radius: 4px;
  word-break: break-all;
}

/* Package list */
.pkg-list {
  display: flex;
  flex-direction: column;
}

.pkg-item {
  border-bottom: 1px solid var(--border-divider);
  padding: 6px 0;
}

.pkg-item:last-child {
  border-bottom: none;
}

.pkg-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.pkg-info {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  flex: 1;
}

.pkg-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

.pkg-version {
  font-size: 11px;
}

.pkg-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.pkg-size {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  min-width: 56px;
  text-align: right;
}

.deps-toggle {
  cursor: pointer;
  border: none;
  transition: opacity 0.15s;
}

.deps-toggle:hover {
  opacity: 0.75;
}

.pkg-deps {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 6px 0 4px;
}

.dep-tag {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 3px;
  background: rgba(0, 0, 0, 0.04);
  color: var(--text-secondary);
}

/* Removal info */
.pkg-removal,
.runtime-removal {
  padding: 6px 0 2px;
}

.removal-cmd {
  display: flex;
  align-items: baseline;
  gap: 6px;
  font-size: 12px;
  margin-bottom: 4px;
}

.removal-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  flex-shrink: 0;
}

.removal-cmd code {
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 3px;
  background: rgba(0, 0, 0, 0.04);
  color: var(--text);
  word-break: break-all;
}

.removal-warning {
  font-size: 12px;
  color: var(--warning);
  padding: 4px 0;
  line-height: 1.5;
}

.manager-hint {
  margin-top: var(--sp-3);
  padding-top: var(--sp-3);
  border-top: 1px solid var(--border-divider);
  font-size: 12px;
  display: flex;
  align-items: baseline;
  gap: 6px;
}

/* Version list (runtimes) */
.version-list {
  display: flex;
  flex-direction: column;
}

.version-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px solid var(--border-divider);
}

.version-item:last-child {
  border-bottom: none;
}

.version-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.version-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
}

.version-right {
  flex-shrink: 0;
}

.version-size {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
}

/* Command transparency */
.commands-section {
  margin-top: var(--sp-3);
  padding-top: var(--sp-3);
  border-top: 1px solid var(--border-divider);
}

.commands-toggle {
  background: none;
  border: none;
  padding: 0;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  font-family: inherit;
  transition: color 0.15s;
}

.commands-toggle:hover {
  color: var(--text);
}

.commands-list {
  margin-top: var(--sp-2);
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.cmd-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  padding: 3px 0;
}

.cmd-status {
  font-size: 11px;
  width: 14px;
  text-align: center;
  flex-shrink: 0;
}

.cmd-status.ok {
  color: var(--success);
}

.cmd-status.fail {
  color: var(--danger);
}

.cmd-text {
  font-size: 11px;
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text);
}

.cmd-time {
  font-size: 10px;
  flex-shrink: 0;
  min-width: 40px;
  text-align: right;
}

/* Custom probe actions */
.custom-probe-actions {
  margin-top: var(--sp-3);
  padding-top: var(--sp-3);
  border-top: 1px solid var(--border-divider);
  display: flex;
  gap: var(--sp-2);
}

/* Probe form */
.probe-form {
  display: flex;
  flex-direction: column;
  gap: var(--sp-4);
}

.form-row {
  display: flex;
  gap: var(--sp-2);
  align-items: flex-end;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.flex-1 {
  flex: 1;
  min-width: 0;
}

.form-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.form-input {
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.5);
  backdrop-filter: blur(4px);
  font-size: 13px;
  color: var(--text);
  outline: none;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.form-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(0, 136, 255, 0.12);
}

.form-input::placeholder {
  color: rgba(0, 0, 0, 0.25);
}

.form-select {
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.5);
  font-size: 13px;
  color: var(--text);
  outline: none;
  appearance: none;
  cursor: pointer;
}

.form-fieldset {
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: var(--sp-3);
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
  background: rgba(0, 0, 0, 0.015);
}

.form-legend {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  padding: 0 4px;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.form-legend .text-muted {
  text-transform: none;
  letter-spacing: 0;
  font-weight: 400;
}

.radio-group {
  display: flex;
  gap: var(--sp-3);
  padding-top: 2px;
}

.radio-label {
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 5px;
  cursor: pointer;
  color: var(--text);
}

.cmd-program {
  width: 160px;
  flex-shrink: 0;
}

.cmd-parse {
  width: 170px;
  flex-shrink: 0;
}

.probe-test-btn {
  align-self: flex-end;
  flex-shrink: 0;
}

/* Test result */
.test-result {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-radius: 6px;
  font-size: 12px;
}

.test-result.test-ok {
  background: rgba(52, 199, 89, 0.1);
  border: 1px solid rgba(52, 199, 89, 0.2);
}

.test-result.test-fail {
  background: rgba(255, 59, 48, 0.08);
  border: 1px solid rgba(255, 59, 48, 0.15);
}

.test-output {
  font-size: 11px;
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text);
}
</style>
