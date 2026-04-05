import { describe, it, expect, beforeEach, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

import { largeFiles, removeDeletedFiles } from "../stores/largeFilesStore";

describe("largeFilesStore", () => {
  beforeEach(() => {
    largeFiles.value = [
      { path: "/a.txt", name: "a.txt", size: 100, apparent_size: 100, modified: "2026-01-01" } as any,
      { path: "/b.txt", name: "b.txt", size: 200, apparent_size: 200, modified: "2026-01-01" } as any,
      { path: "/c.txt", name: "c.txt", size: 300, apparent_size: 300, modified: "2026-01-01" } as any,
    ];
  });

  it("removes files matching the given paths", () => {
    removeDeletedFiles(new Set(["/a.txt", "/c.txt"]));
    expect(largeFiles.value).toHaveLength(1);
    expect(largeFiles.value[0].path).toBe("/b.txt");
  });

  it("does nothing when no paths match", () => {
    removeDeletedFiles(new Set(["/z.txt"]));
    expect(largeFiles.value).toHaveLength(3);
  });

  it("handles empty set", () => {
    removeDeletedFiles(new Set());
    expect(largeFiles.value).toHaveLength(3);
  });
});
