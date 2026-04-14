import { describe, it, expect, beforeEach, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

import { archiveEntries, archiveCandidates, setArchiveEntries, removeArchiveCandidates } from "../stores/archiveStore";

describe("archiveStore", () => {
  describe("setArchiveEntries", () => {
    it("replaces archive entries", () => {
      setArchiveEntries([{ id: "1", original_path: "/a" } as any]);
      expect(archiveEntries.value).toHaveLength(1);
      expect(archiveEntries.value[0].id).toBe("1");
    });

    it("clears entries with empty array", () => {
      setArchiveEntries([{ id: "1" } as any]);
      setArchiveEntries([]);
      expect(archiveEntries.value).toHaveLength(0);
    });
  });

  describe("removeArchiveCandidates", () => {
    beforeEach(() => {
      archiveCandidates.value = [
        { path: "/a", size: 100 } as any,
        { path: "/b", size: 200 } as any,
        { path: "/c", size: 300 } as any,
      ];
    });

    it("removes candidates by path", () => {
      removeArchiveCandidates(["/a", "/c"]);
      expect(archiveCandidates.value).toHaveLength(1);
      expect(archiveCandidates.value[0].path).toBe("/b");
    });

    it("does nothing for non-matching paths", () => {
      removeArchiveCandidates(["/z"]);
      expect(archiveCandidates.value).toHaveLength(3);
    });
  });
});
