<script setup lang="ts">
import { ref, computed } from "vue";
import type { LaunchItem, AppTrustInfo, ShellInitFinding, Severity } from "../types";
import {
  securityResult,
  securityScanning,
  securityScanned,
  securityError,
  scanSecurity,
  disableLaunchItem,
  removeLaunchItem,
} from "../stores/scanStore";

type SectionId = "launch" | "trust" | "shell";

const expandedSections = ref<Set<SectionId>>(new Set(["launch", "trust", "shell"]));
const expandedLaunch = ref<Set<string>>(new Set());
const expandedTrust = ref<Set<string>>(new Set());
const expandedShell = ref<Set<string>>(new Set());
const actionInProgress = ref<string | null>(null);
const successMsg = ref("");

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
  successMsg.value = "";
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
  successMsg.value = "";
  try {
    const result = await disableLaunchItem(item.path);
    if (result.success) {
      successMsg.value = `Disabled "${item.label}"`;
      item.is_enabled = false;
    }
    if (result.errors.length > 0) {
      securityError.value = result.errors.join("; ");
    }
  } catch (e) {
    securityError.value = String(e);
  } finally {
    actionInProgress.value = null;
  }
}

async function handleRemove(item: LaunchItem) {
  actionInProgress.value = `remove-${item.path}`;
  successMsg.value = "";
  try {
    const result = await removeLaunchItem(item.path);
    if (result.success) {
      successMsg.value = `Removed "${item.label}"`;
      if (securityResult.value) {
        securityResult.value.launch_items = securityResult.value.launch_items.filter(
          (i) => i.path !== item.path
        );
      }
    }
    if (result.errors.length > 0) {
      securityError.value = result.errors.join("; ");
    }
  } catch (e) {
    securityError.value = String(e);
  } finally {
    actionInProgress.value = null;
  }
}
</script>

<template>
  <section class="security-view">
    <div class="view-header">
      <div class="view-header-top">
        <div>
          <h2>Security Audit</h2>
          <p class="text-muted">
            Inspect startup items, app trust, and shell configuration
          </p>
        </div>
        <button
          class="btn-primary scan-btn"
          :disabled="securityScanning"
          @click="scan"
        >
          <span v-if="securityScanning" class="spinner-sm"></span>
          {{ securityScanning ? "Scanning..." : "Scan" }}
        </button>
      </div>
    </div>

    <div v-if="securityError" class="error-message">{{ securityError }}</div>
    <div v-if="successMsg" class="success-message">{{ successMsg }}</div>

    <div v-if="securityScanning" class="loading-state">
      <span class="spinner"></span>
      <span>Running security audit...</span>
    </div>

    <template v-else-if="securityScanned && securityResult">
      <!-- Summary Cards -->
      <div class="summary-row">
        <div class="summary-card">
          <span class="summary-count">{{ securityResult.summary.total_findings }}</span>
          <span class="summary-label text-muted">Total Findings</span>
        </div>
        <div class="summary-card summary-malicious">
          <span class="summary-count">{{ securityResult.summary.malicious }}</span>
          <span class="summary-label">Malicious</span>
        </div>
        <div class="summary-card summary-unwanted">
          <span class="summary-count">{{ securityResult.summary.likely_unwanted }}</span>
          <span class="summary-label">Likely Unwanted</span>
        </div>
        <div class="summary-card summary-suspicious">
          <span class="summary-count">{{ securityResult.summary.suspicious }}</span>
          <span class="summary-label">Suspicious</span>
        </div>
        <div class="summary-card summary-info">
          <span class="summary-count">{{ securityResult.summary.informational }}</span>
          <span class="summary-label">Informational</span>
        </div>
      </div>

      <!-- Launch Items Section -->
      <div class="section">
        <div class="section-header" tabindex="0" role="button" :aria-expanded="expandedSections.has('launch')" @click="toggleSection('launch')" @keydown.enter="toggleSection('launch')" @keydown.space.prevent="toggleSection('launch')">
          <span class="expand-chevron" :class="{ expanded: expandedSections.has('launch') }">
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg>
          </span>
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
        </div>

        <div v-if="expandedSections.has('launch')" class="section-body">
          <div v-if="securityResult.launch_items.length === 0" class="card empty-state">
            <p class="text-muted">No launch items found</p>
          </div>

          <div
            v-for="item in securityResult.launch_items"
            :key="item.path"
            class="card-flush finding-item"
          >
            <div class="finding-row" @click="toggleLaunch(item.path)">
              <div class="finding-info">
                <div class="finding-name-row">
                  <span class="expand-chevron" :class="{ expanded: expandedLaunch.has(item.path) }">
                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg>
                  </span>
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
      </div>

      <!-- App Trust Section -->
      <div class="section">
        <div class="section-header" tabindex="0" role="button" :aria-expanded="expandedSections.has('trust')" @click="toggleSection('trust')" @keydown.enter="toggleSection('trust')" @keydown.space.prevent="toggleSection('trust')">
          <span class="expand-chevron" :class="{ expanded: expandedSections.has('trust') }">
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg>
          </span>
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
        </div>

        <div v-if="expandedSections.has('trust')" class="section-body">
          <div v-if="securityResult.app_trust.length === 0" class="card empty-state">
            <p class="text-muted">No app trust issues found</p>
          </div>

          <div
            v-for="app in securityResult.app_trust"
            :key="app.path"
            class="card-flush finding-item"
          >
            <div class="finding-row" @click="toggleTrust(app.path)">
              <div class="finding-info">
                <div class="finding-name-row">
                  <span class="expand-chevron" :class="{ expanded: expandedTrust.has(app.path) }">
                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg>
                  </span>
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
                <div class="finding-detail">
                  <span class="mono text-muted">{{ app.bundle_id || "---" }}</span>
                </div>
              </div>
            </div>

            <div v-if="expandedTrust.has(app.path)" class="finding-expanded">
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
          </div>
        </div>
      </div>

      <!-- Shell Init Section -->
      <div class="section">
        <div class="section-header" tabindex="0" role="button" :aria-expanded="expandedSections.has('shell')" @click="toggleSection('shell')" @keydown.enter="toggleSection('shell')" @keydown.space.prevent="toggleSection('shell')">
          <span class="expand-chevron" :class="{ expanded: expandedSections.has('shell') }">
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg>
          </span>
          <h3 class="section-title">Shell Init</h3>
          <span class="badge badge-accent">
            {{ securityResult.shell_findings.length }}
          </span>
        </div>

        <div v-if="expandedSections.has('shell')" class="section-body">
          <div v-if="securityResult.shell_findings.length === 0" class="card empty-state">
            <p class="text-muted">No suspicious shell configuration found</p>
          </div>

          <div
            v-for="sf in securityResult.shell_findings"
            :key="shellKey(sf)"
            class="card-flush finding-item"
          >
            <div class="finding-row" @click="toggleShell(shellKey(sf))">
              <div class="finding-info">
                <div class="finding-name-row">
                  <span class="expand-chevron" :class="{ expanded: expandedShell.has(shellKey(sf)) }">
                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 2 L8 6 L4 10"/></svg>
                  </span>
                  <span class="finding-name">{{ sf.finding.title }}</span>
                  <span :class="['badge', severityClass[sf.finding.severity]]">
                    {{ severityLabel[sf.finding.severity] }}
                  </span>
                </div>
                <div class="finding-detail">
                  <span class="mono text-muted">{{ sf.file_path }}:{{ sf.line_number }}</span>
                </div>
              </div>
            </div>

            <div v-if="expandedShell.has(shellKey(sf))" class="finding-expanded">
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
          </div>
        </div>
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

.summary-card {
  flex: 1;
  background: var(--surface);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
  padding: var(--sp-4);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--sp-1);
  transition: box-shadow 0.2s ease;
}

.summary-card:hover {
  box-shadow: var(--shadow-md);
}

.summary-count {
  font-size: 24px;
  font-weight: 700;
  color: var(--text);
}

.summary-label {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.summary-malicious {
  border-bottom: 3px solid var(--danger-text);
}

.summary-malicious .summary-count {
  color: var(--danger-text);
}

.summary-malicious .summary-label {
  color: var(--danger-text);
}

.summary-unwanted {
  border-bottom: 3px solid var(--warning-text);
}

.summary-unwanted .summary-count {
  color: var(--warning-text);
}

.summary-unwanted .summary-label {
  color: var(--warning-text);
}

.summary-suspicious {
  border-bottom: 3px solid var(--warning);
}

.summary-suspicious .summary-count {
  color: var(--warning-text);
}

.summary-suspicious .summary-label {
  color: var(--warning-text);
}

.summary-info {
  border-bottom: 3px solid var(--accent);
}

.summary-info .summary-count {
  color: var(--info-text);
}

.summary-info .summary-label {
  color: var(--info-text);
}

/* Sections */
.section {
  margin-bottom: var(--sp-6);
}

.section-header {
  display: flex;
  align-items: center;
  gap: var(--sp-3);
  padding: var(--sp-3) 0;
  cursor: pointer;
  user-select: none;
  border-radius: var(--radius-sm);
  transition: background 0.15s ease;
}

.section-header:hover {
  background: var(--surface-alt);
}

.section-header .section-title {
  margin-bottom: 0;
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
