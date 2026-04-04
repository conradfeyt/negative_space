const SPARSE_THRESHOLD = 0.8;

/** Compute actual disk size for a file, respecting sparse files. */
export function fileDiskSize(f: { is_sparse: boolean; actual_size: number; apparent_size: number }): number {
  const sparse = f.is_sparse && f.actual_size < f.apparent_size * SPARSE_THRESHOLD;
  return sparse ? f.actual_size : f.apparent_size;
}

// Temperature thresholds (degrees Celsius)
export const TEMP_CRITICAL = 95;
export const TEMP_HOT = 80;
export const TEMP_WARM = 65;
export const TEMP_COOL = 45;

/** Map a temperature to an HSLA color string. */
export function tempToColor(t: number): string {
  if (t >= TEMP_CRITICAL) return "hsla(0, 50%, 48%, 0.85)";
  if (t >= TEMP_HOT) return "hsla(25, 55%, 45%, 0.85)";
  if (t >= TEMP_WARM) return "hsla(40, 55%, 45%, 0.85)";
  if (t >= TEMP_COOL) return "hsla(160, 35%, 42%, 0.85)";
  return "hsla(195, 35%, 42%, 0.85)";
}

export function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";

  const units = ["B", "KB", "MB", "GB", "TB"];
  const k = 1024;
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const index = Math.min(i, units.length - 1);
  const value = bytes / Math.pow(k, index);

  return `${value.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}
