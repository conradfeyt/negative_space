import { describe, it, expect, beforeEach, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

import { vaultEntries, vaultCandidates, setVaultEntries, removeCandidates } from "../stores/vaultStore";

describe("vaultStore", () => {
  describe("setVaultEntries", () => {
    it("replaces vault entries", () => {
      setVaultEntries([{ id: "1", original_path: "/a" } as any]);
      expect(vaultEntries.value).toHaveLength(1);
      expect(vaultEntries.value[0].id).toBe("1");
    });

    it("clears entries with empty array", () => {
      setVaultEntries([{ id: "1" } as any]);
      setVaultEntries([]);
      expect(vaultEntries.value).toHaveLength(0);
    });
  });

  describe("removeCandidates", () => {
    beforeEach(() => {
      vaultCandidates.value = [
        { path: "/a", size: 100 } as any,
        { path: "/b", size: 200 } as any,
        { path: "/c", size: 300 } as any,
      ];
    });

    it("removes candidates by path", () => {
      removeCandidates(["/a", "/c"]);
      expect(vaultCandidates.value).toHaveLength(1);
      expect(vaultCandidates.value[0].path).toBe("/b");
    });

    it("does nothing for non-matching paths", () => {
      removeCandidates(["/z"]);
      expect(vaultCandidates.value).toHaveLength(3);
    });
  });
});
