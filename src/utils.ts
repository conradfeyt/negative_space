const SPARSE_THRESHOLD = 0.8;

/** Compute actual disk size for a file, respecting sparse files. */
export function fileDiskSize(f: { is_sparse: boolean; actual_size: number; apparent_size: number }): number {
  const sparse = f.is_sparse && f.actual_size < f.apparent_size * SPARSE_THRESHOLD;
  return sparse ? f.actual_size : f.apparent_size;
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
