<script setup lang="ts">
import { ref, computed, watch } from "vue";
import type { LaunchItem, AppTrustInfo, ShellInitFinding, Severity } from "../types";
import { showToast } from "../stores/toastStore";
import {
  securityResult,
  securityScanning,
  securityScanned,
  securityError,
  scanSecurity,
  disableLaunchItem,
  removeLaunchItem,
} from "../stores/scanStore";
import StatCard from "../components/StatCard.vue";
import EmptyState from "../components/EmptyState.vue";
import ChevronIcon from "../components/ChevronIcon.vue";
import CollapsibleSection from "../components/CollapsibleSection.vue";
import LoadingState from "../components/LoadingState.vue";
import ViewHeader from "../components/ViewHeader.vue";

type SectionId = "launch" | "trust" | "shell";

const expandedSections = ref<Set<SectionId>>(new Set(["launch", "trust", "shell"]));
const expandedLaunch = ref<Set<string>>(new Set());
const expandedTrust = ref<Set<string>>(new Set());
const expandedShell = ref<Set<string>>(new Set());
const actionInProgress = ref<string | null>(null);

const severityLabel: Record<Severity, string> = {
  malicious: "Malicious",
  likely_unwanted: "Likely Unwanted",
  suspicious: "Suspicious",
  informational: "Informational",
};

const severityClass: Record<Severity, string> = {
  malicious: "severity-malicious",
  likely_unwanted: "severity-unwanted",
  suspicious: "severity-suspicious",
  informational: "severity-info",
};

const launchItemsWithFindings = computed(() => {
  if (!securityResult.value) return [];
  return securityResult.value.launch_items.filter((i) => i.findings.length > 0);
});

const appTrustWithFindings = computed(() => {
  if (!securityResult.value) return [];
  return securityResult.value.app_trust.filter((a) => a.findings.length > 0);
});

async function scan() {
  expandedLaunch.value = new Set();
  expandedTrust.value = new Set();
  expandedShell.value = new Set();
  await scanSecurity();
}

function toggleSection(id: SectionId) {
  const next = new Set(expandedSections.value);
  if (next.has(id)) {
    next.delete(id);
  } else {
    next.add(id);
  }
  expandedSections.value = next;
}

function toggleLaunch(path: string) {
  const next = new Set(expandedLaunch.value);
  next.has(path) ? next.delete(path) : next.add(path);
  expandedLaunch.value = next;
}

function toggleTrust(path: string) {
  const next = new Set(expandedTrust.value);
  next.has(path) ? next.delete(path) : next.add(path);
  expandedTrust.value = next;
}

function toggleShell(id: string) {
  const next = new Set(expandedShell.value);
  next.has(id) ? next.delete(id) : next.add(id);
  expandedShell.value = next;
}

function shellKey(sf: ShellInitFinding): string {
  return `${sf.file_path}:${sf.line_number}`;
}

function highestSeverityLaunch(item: LaunchItem): Severity {
  return highestSeverity(item.findings.map((f) => f.severity));
}

function highestSeverityTrust(item: AppTrustInfo): Severity {
  return highestSeverity(item.findings.map((f) => f.severity));
}

function highestSeverity(severities: Severity[]): Severity {
  const order: Severity[] = ["malicious", "likely_unwanted", "suspicious", "informational"];
  for (const s of order) {
    if (severities.includes(s)) return s;
  }
  return "informational";
}

async function handleDisable(item: LaunchItem) {
  actionInProgress.value = `disable-${item.path}`;
  try {
    const result = await disableLaunchItem(item.path);
    if (result.success) {
      showToast(`Disabled "${item.label}"`, "success");
      item.is_enabled = false;
    }
    if (result.errors.length > 0) {
      showToast(result.errors.join("; "), "error");
    }
  } catch (e) {
    showToast(String(e), "error");
  } finally {
    actionInProgress.value = null;
  }
}

async function handleRemove(item: LaunchItem) {
  actionInProgress.value = `remove-${item.path}`;
  try {
    const result = await removeLaunchItem(item.path);
    if (result.success) {
      showToast(`Removed "${item.label}"`, "success");
      if (securityResult.value) {
        securityResult.value.launch_items = securityResult.value.launch_items.filter(
          (i) => i.path !== item.path
        );
      }
    }
    if (result.errors.length > 0) {
      showToast(result.errors.join("; "), "error");
    }
  } catch (e) {
    showToast(String(e), "error");
  } finally {
    actionInProgress.value = null;
  }
}

watch(securityError, (err) => {
  if (err) showToast(err, "error");
});
</script>

<template>
  <section class="security-view">
    <ViewHeader
      title="Security Audit"
      subtitle="Inspect startup items, app trust, and shell configuration"
    >
      <template #actions>
        <button
          class="btn-primary scan-btn"
          :disabled="securityScanning"
          @click="scan"
        >
          <span v-if="securityScanning" class="spinner-sm"></span>
          {{ securityScanning ? "Scanning..." : "Scan" }}
        </button>
      </template>
    </ViewHeader>

    <LoadingState v-if="securityScanning" message="Running security audit..." />

    <template v-else-if="securityScanned && securityResult">
      <!-- Summary Cards -->
      <div class="summary-row">
        <StatCard :value="String(securityResult.summary.total_findings)" label="Total Findings" />
        <StatCard :value="String(securityResult.summary.malicious)" label="Malicious" value-color="var(--danger-text)" label-color="var(--danger-text)" />
        <StatCard :value="String(securityResult.summary.likely_unwanted)" label="Likely Unwanted" value-color="var(--warning-text)" label-color="var(--warning-text)" />
        <StatCard :value="String(securityResult.summary.suspicious)" label="Suspicious" value-color="var(--warning-text)" label-color="var(--warning-text)" />
        <StatCard :value="String(securityResult.summary.informational)" label="Informational" value-color="var(--info-text)" label-color="var(--info-text)" />
      </div>

      <!-- Launch Items Section -->
      <div class="section">
        <CollapsibleSection
          :expanded="expandedSections.has('launch')"
          @toggle="toggleSection('launch')"
        >
          <template #header>
            <h3 class="section-title">Launch Items</h3>
            <span class="badge badge-accent">
              {{ securityResult.launch_items.length }}
            </span>
            <span
              v-if="launchItemsWithFindings.length > 0"
              class="badge badge-warning"
            >
              {{ launchItemsWithFindings.length }} with findings
            </span>
          </template>
          <div class="section-body">
          <EmptyState
            v-if="securityResult.launch_items.length === 0"
            title="No launch items found"
            description="Startup agents and daemons will appear here if detected."
          />

          <div
            v-for="item in securityResult.launch_items"
            :key="item.path"
            class="card-flush finding-item"
          >
            <div class="finding-row" @click="toggleLaunch(item.path)">
              <div class="finding-info">
                <div class="finding-name-row">
                  <ChevronIcon :expanded="expandedLaunch.has(item.path)" />
                  <span class="finding-name">{{ item.label }}</span>
                  <span
                    v-if="item.findings.length > 0"
                    :class="['badge', severityClass[highestSeverityLaunch(item)]]"
                  >
                    {{ severityLabel[highestSeverityLaunch(item)] }}
                  </span>
                  <span v-if="!item.is_enabled" class="badge badge-neutral">Disabled</span>
                  <span v-if="!item.program_exists" class="badge badge-danger">Missing Program</span>
                </div>
                <div class="finding-detail">
                  <span class="mono text-muted">{{ item.path }}</span>
                </div>
              </div>
              <div class="finding-actions">
                <span class="finding-location text-muted">{{ item.location }}</span>
                <button
                  v-if="item.is_enabled"
                  class="btn-secondary btn-sm"
                  :disabled="actionInProgress === `disable-${item.path}`"
                  @click.stop="handleDisable(item)"
                >
                  {{ actionInProgress === `disable-${item.path}` ? "..." : "Disable" }}
                </button>
                <button
                  class="btn-danger btn-sm"
                  :disabled="actionInProgress === `remove-${item.path}`"
                  @click.stop="handleRemove(item)"
                >
                  {{ actionInProgress === `remove-${item.path}` ? "..." : "Remove" }}
                </button>
              </div>
            </div>

            <div v-if="expandedLaunch.has(item.path)" class="finding-expanded">
              <div class="expanded-section">
                <span class="expanded-label">Program</span>
                <span class="mono expanded-path">{{ item.program }}</span>
                <span v-if="!item.program_exists" class="text-danger"> (not found)</span>
              </div>
              <div class="expanded-section">
                <span class="expanded-label">Code Signing</span>
                <span v-if="item.is_signed" class="text-muted">
                  Signed by {{ item.signer }}
                </span>
                <span v-else class="text-danger">Not signed</span>
              </div>
              <div
                v-for="finding in item.findings"
                :key="finding.id"
                class="finding-detail-card"
              >
                <div class="finding-detail-header">
                  <span :class="['badge', severityClass[finding.severity]]">
                    {{ severityLabel[finding.severity] }}
                  </span>
                  <span class="finding-detail-title">{{ finding.title }}</span>
                </div>
                <p class="finding-description">{{ finding.description }}</p>
                <div v-if="finding.evidence.length > 0" class="expanded-section">
                  <span class="expanded-label">Evidence</span>
                  <ul class="evidence-list">
                    <li v-for="(ev, idx) in finding.evidence" :key="idx" class="mono">
                      {{ ev }}
                    </li>
                  </ul>
                </div>
                <div class="expanded-section">
                  <span class="expanded-label">Suggested Action</span>
                  <span class="finding-description">{{ finding.suggested_action }}</span>
                </div>
              </div>
              <div v-if="item.findings.length === 0" class="expanded-section">
                <span class="text-muted">No findings -- item appears clean</span>
              </div>
            </div>
          </div>
          </div>
        </CollapsibleSection>
      </div>

      <!-- App Trust Section -->
      <div class="section">
        <CollapsibleSection
          :expanded="expandedSections.has('trust')"
          @toggle="toggleSection('trust')"
        >
          <template #header>
            <h3 class="section-title">App Trust</h3>
            <span class="badge badge-accent">
              {{ securityResult.app_trust.length }}
            </span>
            <span
              v-if="appTrustWithFindings.length > 0"
              class="badge badge-warning"
            >
              {{ appTrustWithFindings.length }} with findings
            </span>
          </template>
          <div class="section-body">
          <EmptyState
            v-if="securityResult.app_trust.length === 0"
            title="No app trust issues found"
            description="Applications with code signing or notarization concerns will appear here."
          />

          <div
            v-for="app in securityResult.app_trust"
            :key="app.path"
            class="card-flush finding-item finding-item--collapsible"
          >
            <CollapsibleSection
              :expanded="expandedTrust.has(app.path)"
              @toggle="toggleTrust(app.path)"
            >
              <template #header>
                <div class="finding-info">
                  <div class="finding-name-row">
                    <span class="finding-name">{{ app.name }}</span>
                    <span
                      v-if="app.findings.length > 0"
                      :class="['badge', severityClass[highestSeverityTrust(app)]]"
                    >
                      {{ severityLabel[highestSeverityTrust(app)] }}
                    </span>
                    <span v-if="!app.is_signed" class="badge badge-danger">Unsigned</span>
                    <span v-else-if="!app.signature_valid" class="badge badge-warning">Invalid Signature</span>
                    <span v-if="app.is_notarized" class="badge badge-success">Notarized</span>
                  </div>
                  <div class="finding-detail finding-detail--flush">
                    <span class="mono text-muted">{{ app.bundle_id || "---" }}</span>
                  </div>
                </div>
              </template>
              <div class="finding-expanded">
              <div class="expanded-section">
                <span class="expanded-label">Path</span>
                <span class="mono expanded-path">{{ app.path }}</span>
              </div>
              <div class="expanded-section">
                <span class="expanded-label">Code Signing</span>
                <span v-if="app.is_signed" class="text-muted">
                  Signed by {{ app.signer }}
                  <span v-if="!app.signature_valid" class="text-danger"> (invalid)</span>
                </span>
                <span v-else class="text-danger">Not signed</span>
              </div>
              <div class="expanded-section">
                <span class="expanded-label">Notarization</span>
                <span :class="app.is_notarized ? '' : 'text-muted'">
                  {{ app.is_notarized ? "Notarized by Apple" : "Not notarized" }}
                </span>
              </div>
              <div class="expanded-section">
                <span class="expanded-label">Quarantine Flag</span>
                <span class="text-muted">
                  {{ app.has_quarantine ? "Present" : "Not present" }}
                </span>
              </div>
              <div
                v-for="finding in app.findings"
                :key="finding.id"
                class="finding-detail-card"
              >
                <div class="finding-detail-header">
                  <span :class="['badge', severityClass[finding.severity]]">
                    {{ severityLabel[finding.severity] }}
                  </span>
                  <span class="finding-detail-title">{{ finding.title }}</span>
                </div>
                <p class="finding-description">{{ finding.description }}</p>
                <div v-if="finding.evidence.length > 0" class="expanded-section">
                  <span class="expanded-label">Evidence</span>
                  <ul class="evidence-list">
                    <li v-for="(ev, idx) in finding.evidence" :key="idx" class="mono">
                      {{ ev }}
                    </li>
                  </ul>
                </div>
                <div class="expanded-section">
                  <span class="expanded-label">Suggested Action</span>
                  <span class="finding-description">{{ finding.suggested_action }}</span>
                </div>
              </div>
              <div v-if="app.findings.length === 0" class="expanded-section">
                <span class="text-muted">No findings -- app appears trustworthy</span>
              </div>
              </div>
            </CollapsibleSection>
          </div>
          </div>
        </CollapsibleSection>
      </div>

      <!-- Shell Init Section -->
      <div class="section">
        <CollapsibleSection
          :expanded="expandedSections.has('shell')"
          @toggle="toggleSection('shell')"
        >
          <template #header>
            <h3 class="section-title">Shell Init</h3>
            <span class="badge badge-accent">
              {{ securityResult.shell_findings.length }}
            </span>
          </template>
          <div class="section-body">
          <EmptyState
            v-if="securityResult.shell_findings.length === 0"
            title="No suspicious shell configuration found"
            description="Shell init file findings will appear here if any are detected."
          />

          <div
            v-for="sf in securityResult.shell_findings"
            :key="shellKey(sf)"
            class="card-flush finding-item finding-item--collapsible"
          >
            <CollapsibleSection
              :expanded="expandedShell.has(shellKey(sf))"
              @toggle="toggleShell(shellKey(sf))"
            >
              <template #header>
                <div class="finding-info">
                  <div class="finding-name-row">
                    <span class="finding-name">{{ sf.finding.title }}</span>
                    <span :class="['badge', severityClass[sf.finding.severity]]">
                      {{ severityLabel[sf.finding.severity] }}
                    </span>
                  </div>
                  <div class="finding-detail finding-detail--flush">
                    <span class="mono text-muted">{{ sf.file_path }}:{{ sf.line_number }}</span>
                  </div>
                </div>
              </template>
              <div class="finding-expanded">
              <div class="expanded-section">
                <span class="expanded-label">Line Content</span>
                <pre class="shell-line mono">{{ sf.line_content }}</pre>
              </div>
              <div class="expanded-section">
                <span class="expanded-label">Description</span>
                <span class="finding-description">{{ sf.finding.description }}</span>
              </div>
              <div v-if="sf.finding.evidence.length > 0" class="expanded-section">
                <span class="expanded-label">Evidence</span>
                <ul class="evidence-list">
                  <li v-for="(ev, idx) in sf.finding.evidence" :key="idx" class="mono">
                    {{ ev }}
                  </li>
                </ul>
              </div>
              <div class="expanded-section">
                <span class="expanded-label">Suggested Action</span>
                <span class="finding-description">{{ sf.finding.suggested_action }}</span>
              </div>
              </div>
            </CollapsibleSection>
          </div>
          </div>
        </CollapsibleSection>
      </div>
    </template>
  </section>
</template>

<style scoped>
.security-view {
  max-width: 1440px;
}

/* Summary Cards */
.summary-row {
  display: flex;
  gap: var(--sp-3);
  margin-bottom: var(--sp-6);
}


/* Sections */
.section {
  margin-bottom: var(--sp-6);
}

.section :deep(.collapsible-header) {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-3) 0;
  cursor: pointer;
  user-select: none;
  border-radius: var(--radius-sm);
  transition: background 0.15s ease;
}

.section :deep(.collapsible-header:hover) {
  background: var(--surface-alt);
}

.section :deep(.collapsible-header .section-title) {
  margin-bottom: 0;
}

.finding-item--collapsible :deep(.collapsible-header) {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  justify-content: flex-start;
  padding: var(--sp-4) var(--sp-5);
  cursor: pointer;
  transition: background 0.15s ease;
}

.finding-item--collapsible :deep(.collapsible-header:hover) {
  background: var(--surface-alt);
}

.finding-item--collapsible :deep(.collapsible-header .finding-info) {
  flex: 1;
  min-width: 0;
}

.finding-detail--flush {
  padding-left: 0;
}

.section-body {
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
  animation: slideDown 0.15s ease;
}

/* Finding Items */
.finding-item {
  overflow: hidden;
}

.finding-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--sp-4) var(--sp-5);
  cursor: pointer;
  transition: background 0.15s ease;
}

.finding-row:hover {
  background: var(--surface-alt);
}

.finding-info {
  display: flex;
  flex-direction: column;
  gap: var(--sp-1);
  min-width: 0;
  flex: 1;
}

.finding-name-row {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
}

.finding-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.finding-detail {
  padding-left: 28px;
  font-size: 12px;
}

.finding-actions {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  flex-shrink: 0;
  margin-left: var(--sp-4);
}

.finding-location {
  font-size: 12px;
  white-space: nowrap;
}

/* Severity Badges — softer / calmer tints */
.severity-malicious {
  background: var(--danger-tint);
  color: var(--danger-text);
}

.severity-unwanted {
  background: var(--warning-tint);
  color: var(--warning-text);
}

.severity-suspicious {
  background: var(--info-tint);
  color: var(--info-text);
}

.severity-info {
  background: var(--accent-light);
  color: var(--accent-deep);
}

/* Expanded Details */
.finding-expanded {
  border-top: 1px solid var(--border-divider);
  padding: var(--sp-4) var(--sp-5) var(--sp-4) 46px;
  background: transparent;
  animation: slideDown 0.15s ease;
}

.expanded-section {
  margin-bottom: var(--sp-3);
}

.expanded-section:last-child {
  margin-bottom: 0;
}

.expanded-label {
  display: block;
  font-size: 12px;
  font-weight: 600;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: var(--sp-1);
}

.expanded-path {
  font-size: 12px;
  color: var(--text-secondary);
  word-break: break-all;
}

.finding-detail-card {
  background: transparent;
  border: 1px solid rgba(0, 0, 0, 0.12);
  border-radius: var(--radius-sm);
  padding: 14px;
  margin-bottom: var(--sp-3);
}

.finding-detail-card:last-child {
  margin-bottom: 0;
}

.finding-detail-header {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  margin-bottom: var(--sp-2);
}

.finding-detail-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.finding-description {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.5;
  margin-bottom: var(--sp-2);
}

.evidence-list {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: var(--sp-1);
}

.evidence-list li {
  font-size: 12px;
  color: var(--text-secondary);
  padding: var(--sp-1) var(--sp-2);
  background: transparent;
  border-radius: var(--radius-sm);
  word-break: break-all;
}

.shell-line {
  font-size: 12px;
  color: var(--text-secondary);
  background: rgba(0, 0, 0, 0.03);
  border: 1px solid rgba(0, 0, 0, 0.12);
  border-radius: var(--radius-sm);
  padding: var(--sp-2) var(--sp-3);
  overflow-x: auto;
  white-space: pre;
}

.text-danger {
  color: var(--danger-text);
}
</style>
