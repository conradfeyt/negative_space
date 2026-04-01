export interface DiskUsage {
  total: number;
  used: number;
  free: number;
  percentage: number;
}

export interface FileInfo {
  path: string;
  name: string;
  apparent_size: number;
  actual_size: number;
  modified: string;
  is_sparse: boolean;
}

export interface CacheEntry {
  path: string;
  name: string;
  size: number;
  item_count: number;
}

export interface LogEntry {
  path: string;
  name: string;
  size: number;
  modified: string;
}

export interface DockerInfo {
  installed: boolean;
  running: boolean;
  images: DockerItem[];
  total_reclaimable: string;
  disk_usage_raw: string;
}

export interface DockerItem {
  name: string;
  size: string;
  id: string;
  item_type: string;
}

export interface AppInfo {
  name: string;
  path: string;
  /** Size of the .app bundle itself in bytes */
  size: number;
  bundle_id: string;
  leftover_paths: string[];
  /** Sum of leftover file sizes in bytes */
  leftover_size: number;
  /** Total disk footprint: size + leftover_size */
  footprint: number;
  /** Base64-encoded PNG data URI for the app icon (empty string if unavailable) */
  icon_base64: string;
  /** Install source: "homebrew", "app-store", or "manual" */
  install_source: string;
}

export interface TrashInfo {
  size: number;
  item_count: number;
}

export interface CleanResult {
  success: boolean;
  freed_bytes: number;
  deleted_count: number;
  errors: string[];
}

export interface LargeFileScanResult {
  files: FileInfo[];
  skipped_paths: string[];
}

export interface PathAccess {
  path: string;
  resolved_path: string;
  exists: boolean;
  readable: boolean;
}

export interface ScanArea {
  id: string;
  label: string;
  path: string;
  description: string;
  enabled: boolean;
  access: PathAccess | null;
}

export type Severity = "malicious" | "likely_unwanted" | "suspicious" | "informational";

export interface SecurityFinding {
  id: string;
  category: string;
  severity: Severity;
  title: string;
  description: string;
  path: string;
  evidence: string[];
  suggested_action: string;
}

export interface LaunchItem {
  path: string;
  label: string;
  program: string;
  program_exists: boolean;
  is_enabled: boolean;
  is_signed: boolean;
  signer: string;
  location: string;
  findings: SecurityFinding[];
}

export interface AppTrustInfo {
  path: string;
  name: string;
  is_signed: boolean;
  signature_valid: boolean;
  signer: string;
  is_notarized: boolean;
  has_quarantine: boolean;
  bundle_id: string;
  findings: SecurityFinding[];
}

export interface ShellInitFinding {
  file_path: string;
  line_number: number;
  line_content: string;
  finding: SecurityFinding;
}

export interface SecuritySummary {
  total_findings: number;
  malicious: number;
  likely_unwanted: number;
  suspicious: number;
  informational: number;
}

export interface SecurityScanResult {
  launch_items: LaunchItem[];
  app_trust: AppTrustInfo[];
  shell_findings: ShellInitFinding[];
  summary: SecuritySummary;
}

// Browser cleanup types

export interface BrowserDataCategory {
  id: string;
  label: string;
  paths: string[];
  size: number;
  safe_to_clean: boolean;
  warning: string;
  tcc_protected: boolean;
}

export interface BrowserInfo {
  id: string;
  name: string;
  app_path: string;
  installed: boolean;
  data_categories: BrowserDataCategory[];
  total_size: number;
}

export interface BrowserScanResult {
  browsers: BrowserInfo[];
  total_size: number;
  has_fda: boolean;
}

export interface BrowserCleanResult {
  success: boolean;
  freed_bytes: number;
  deleted_count: number;
  errors: string[];
}

// Disk map / space visualization types

export interface DiskNode {
  name: string;
  path: string;
  size: number;
  expanded: boolean;
  children: DiskNode[];
  category: string;
  /** Unix timestamp (seconds since epoch). Null until enriched by async recency scan. */
  modified?: number | null;
}

export interface DiskMapResult {
  root: DiskNode;
  disk_total: number;
  disk_used: number;
  disk_free: number;
}

/** Metadata about a cached disk map scan (returned by list_disk_map_caches). */
export interface CacheMetadata {
  /** Unique ID — the filename stem, e.g. "spacemap-2025-03-10T14:30:00" */
  id: string;
  /** ISO 8601 timestamp string of when the scan was saved */
  timestamp: string;
  /** How many seconds ago this cache was saved */
  age_seconds: number;
}

// System maintenance types

export interface MaintenanceTask {
  id: string;
  name: string;
  description: string;
  requires_admin: boolean;
  status: string;
  message: string;
  warning: string;
  commands: string[];
  services_affected: string[];
  paths_affected: string[];
  destructive: boolean;
  reversible_info: string;
}

export interface MaintenanceResult {
  task_id: string;
  success: boolean;
  message: string;
}

export interface MaintenanceTaskList {
  tasks: MaintenanceTask[];
}

// Duplicate file finder types

export interface DuplicateFile {
  path: string;
  name: string;
  size: number;
  modified: string;
  parent_dir: string;
}

export interface DuplicateGroup {
  hash: string;
  size: number;
  files: DuplicateFile[];
  wasted_bytes: number;
}

export interface DuplicateScanResult {
  groups: DuplicateGroup[];
  total_duplicate_files: number;
  total_wasted_bytes: number;
  files_scanned: number;
  stage1_candidates: number;
  stage2_candidates: number;
  skipped_paths: string[];
}

// Memory analysis types

export interface ProcessInfo {
  pid: number;
  ppid: number;
  rss_bytes: number;
  mem_percent: number;
  name: string;
  command: string;
  description: string;
}

export interface ProcessGroup {
  name: string;
  category: string;
  description: string;
  total_rss_bytes: number;
  total_mem_percent: number;
  process_count: number;
  processes: ProcessInfo[];
}

export interface MemoryStats {
  total_bytes: number;
  used_bytes: number;
  active_bytes: number;
  inactive_bytes: number;
  wired_bytes: number;
  free_bytes: number;
  compressed_bytes: number;
  app_bytes: number;
}

export interface MemoryScanResult {
  stats: MemoryStats;
  groups: ProcessGroup[];
  total_processes: number;
}

// File preview types (tagged union — discriminated by `kind`)

export interface FilePreviewImage {
  kind: "Image";
  data: string; // base64-encoded PNG
  width: number;
  height: number;
  file_type: string;
  file_size: number;
  file_name: string;
}

export interface FilePreviewText {
  kind: "Text";
  content: string;
  total_lines: number;
  truncated: boolean;
  file_type: string;
  file_size: number;
  file_name: string;
}

export interface FilePreviewMetadata {
  kind: "Metadata";
  file_type: string;
  file_size: number;
  file_name: string;
  mime_guess: string;
}

export interface FilePreviewError {
  kind: "Error";
  message: string;
  file_name: string;
}

export type FilePreview =
  | FilePreviewImage
  | FilePreviewText
  | FilePreviewMetadata
  | FilePreviewError;

// ---------------------------------------------------------------------------
// Packages & Runtimes
// ---------------------------------------------------------------------------

export interface InstalledPackage {
  name: string;
  version: string;
  size: number;
  is_top_level: boolean;
  dependencies: string[];
  uninstall_command: string;
  removal_warning: string;
}

export interface PackageManagerInfo {
  id: string;
  name: string;
  version: string;
  install_path: string;
  total_size: number;
  packages: InstalledPackage[];
  total_package_count: number;
  detected: boolean;
  uninstall_hint: string;
}

export interface RuntimeVersion {
  version: string;
  active: boolean;
  path: string;
  size: number;
}

export interface RuntimeInfo {
  id: string;
  name: string;
  install_method: string;
  install_path: string;
  total_size: number;
  versions: RuntimeVersion[];
  uninstall_hint: string;
  removal_warning: string;
}

export interface PackageScanResult {
  managers: PackageManagerInfo[];
  runtimes: RuntimeInfo[];
  total_size: number;
}

// ---------------------------------------------------------------------------
// Streaming large-file scan event payloads
// ---------------------------------------------------------------------------

export interface LargeFileFound {
  file: FileInfo;
}

export interface LargeFileScanProgress {
  current_dir: string;
  files_found: number;
}

export interface LargeFileScanDone {
  total_files: number;
  skipped_paths: string[];
}

// ---------------------------------------------------------------------------
// System Vitals — thermal monitoring, CPU hogs, remediation
// ---------------------------------------------------------------------------

export type ThermalState = "Nominal" | "Fair" | "Serious" | "Critical" | "Unknown";

export interface VitalsProcess {
  pid: number;
  ppid: number;
  cpu_percent: number;
  rss_bytes: number;
  name: string;
  command: string;
  description: string;
}

export interface VitalsGroup {
  name: string;
  category: string;
  description: string;
  total_cpu_percent: number;
  total_rss_bytes: number;
  process_count: number;
  likely_idle: boolean;
  suggestion: string | null;
  suggestion_severity: string | null;
  can_quit: boolean;
  processes: VitalsProcess[];
}

export interface SystemLoad {
  load_1m: number;
  load_5m: number;
  load_15m: number;
  cpu_cores: number;
  cpu_usage_percent: number;
  uptime_seconds: number;
  uptime_display: string;
}

export interface BatteryInfo {
  installed: boolean;
  charge_percent: number;
  is_charging: boolean;
  ac_connected: boolean;
  health_percent: number;
  cycle_count: number;
  max_capacity_mah: number;
  design_capacity_mah: number;
  temperature_celsius: number;
  condition: string;
  power_source: string;
}

export interface VitalsResult {
  thermal_state: ThermalState;
  thermal_description: string;
  load: SystemLoad;
  groups: VitalsGroup[];
  total_processes: number;
  total_cpu_percent: number;
  background_agent_count: number;
  headline: string;
  battery: BatteryInfo | null;
}

// ---------------------------------------------------------------------------
// Hardware Thermal Sensors — direct SMC readings
// ---------------------------------------------------------------------------

export interface ThermalSensor {
  /** Raw SMC 4-character key, e.g. "Tp01", "Tg0V" */
  key: string;
  /** Human-readable name, e.g. "CPU P-Core 1", "GPU Cluster 2" */
  name: string;
  /** Category: "cpu", "gpu", "memory", "storage", "battery", "airflow", "ambient", "vrm", "wireless", "other" */
  category: string;
  /** Temperature in degrees Celsius */
  temp_celsius: number;
}

export interface FanReading {
  id: number;
  name: string;
  current_rpm: number;
  min_rpm: number;
  max_rpm: number;
  /** Current speed as percentage of range (0-100) */
  percent: number;
}

export interface CategorySummary {
  category: string;
  label: string;
  avg_celsius: number;
  max_celsius: number;
  sensor_count: number;
}

export interface ThermalScanResult {
  sensors: ThermalSensor[];
  summaries: CategorySummary[];
  fans: FanReading[];
  hottest_sensor: ThermalSensor | null;
  assessment: string;
  sensor_count: number;
  chip_name: string;
}

// ---------------------------------------------------------------------------
// Apple Intelligence
// ---------------------------------------------------------------------------

export interface FileClassification {
  path: string;
  safety: "safe" | "probably_safe" | "risky";
  explanation: string;
  confidence: number;
}

export interface ScanSummaryOutput {
  summary: string;
  ai_generated: boolean;
}

// ---------------------------------------------------------------------------
// Vault (compress-instead-of-delete)
// ---------------------------------------------------------------------------

export interface VaultEntry {
  id: string;
  original_path: string;
  original_size: number;
  compressed_size: number;
  compression_ratio: number;
  blake3_hash: string;
  vault_filename: string;
  archived_at: string;
  original_modified: string;
  original_accessed: string;
  permissions: number;
  file_type: string;
}

export interface VaultSummary {
  file_count: number;
  total_original_size: number;
  total_compressed_size: number;
  total_savings: number;
}

export interface CompressionCandidate {
  path: string;
  name: string;
  size: number;
  estimated_compressed_size: number;
  estimated_savings: number;
  estimated_ratio: number;
  last_accessed: string;
  last_modified: string;
  file_type: string;
  recently_accessed: boolean;
}

export interface CompressResult {
  success: boolean;
  files_compressed: number;
  total_original_size: number;
  total_compressed_size: number;
  total_savings: number;
  errors: string[];
}

export interface RestoreResult {
  success: boolean;
  restored_path: string;
  errors: string[];
}
