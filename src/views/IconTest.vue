<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface IconDef {
  key: string;
  label: string;
  name: string;
  mode: string;
  style: string;
  base64: string;
}

const icons = ref<IconDef[]>([]);
const loading = ref(true);

const ICON_SIZE = 64; // render at 64px (2x retina), display at 32px CSS

const testIcons: { key: string; label: string; name: string; mode: string; style: string }[] = [
  // === macOS System Settings > Storage — exact order ===
  { key: "applications", label: "Applications", name: "/System/Applications/App Store.app", mode: "app", style: "grayscaleApp" },
  { key: "bin", label: "Bin", name: "trash.fill", mode: "sf", style: "grayBadgeHier" },
  { key: "books", label: "Books", name: "/System/Applications/Books.app", mode: "app", style: "plain" },
  { key: "developer", label: "Developer", name: "hammer.fill", mode: "sf", style: "grayBadgeHier" },
  { key: "documents", label: "Documents", name: "doc.fill", mode: "sf", style: "grayBadgeHier" },
  { key: "icloud", label: "iCloud Drive", name: "/System/Library/CoreServices/Finder.app/Contents/Applications/iCloud Drive.app/Contents/Resources/OpenICloudDriveAppIcon.icns", mode: "file", style: "plain" },
  { key: "ios_files", label: "iOS Files", name: "iphone", mode: "sf", style: "grayBadgeHier" },
  { key: "mail", label: "Mail", name: "/System/Applications/Mail.app", mode: "app", style: "plain" },
  { key: "messages", label: "Messages", name: "/System/Applications/Messages.app", mode: "app", style: "plain" },
  { key: "music", label: "Music", name: "/System/Applications/Music.app", mode: "app", style: "plain" },
  { key: "music_creation", label: "Music Creation", name: "guitars.fill", mode: "sf", style: "grayBadge" },
  { key: "photos", label: "Photos", name: "/System/Applications/Photos.app", mode: "app", style: "plain" },
  { key: "podcasts", label: "Podcasts", name: "/System/Applications/Podcasts.app", mode: "app", style: "plain" },
  { key: "tv", label: "TV", name: "/System/Applications/TV.app", mode: "app", style: "plain" },
  // --- divider ---
  { key: "other_users", label: "Other Users & Shared", name: "person.2.fill", mode: "sf", style: "grayBadge" },
  { key: "macos", label: "macOS", name: "laptopcomputer", mode: "sf", style: "grayBadgeHier" },
  { key: "system_data", label: "System Data", name: "ellipsis", mode: "sf", style: "grayBadge" },
  // --- Negativ_ additions ---
  { key: "docker", label: "Docker", name: "/Applications/Docker.app", mode: "app", style: "plain" },
  { key: "caches", label: "Caches", name: "archivebox.fill", mode: "sf", style: "grayBadge" },
  { key: "other", label: "Other", name: "puzzlepiece.fill", mode: "sf", style: "grayBadge" },
  // --- FDA gate icons ---
  { key: "system_settings", label: "System Settings", name: "/System/Applications/System Settings.app", mode: "app", style: "plain" },
  { key: "privacy_security", label: "Privacy & Security", name: "hand.raised.fill", mode: "sf", style: "blueGradientBadge" },
  { key: "full_disk_access", label: "Full Disk Access", name: "externaldrive.fill", mode: "sf", style: "grayBadge" },
];

onMounted(async () => {
  const results: IconDef[] = [];

  for (const def of testIcons) {
    try {
      const base64 = await invoke<string>("render_sf_symbol", {
        name: def.name,
        size: ICON_SIZE,
        mode: def.mode,
        style: def.style,
      });
      results.push({ ...def, base64: base64 || "" });
    } catch {
      results.push({ ...def, base64: "" });
    }
  }
  icons.value = results;
  loading.value = false;
});
</script>

<template>
  <div class="icon-test">
    <div class="view-header">
      <h2>Icon Test</h2>
      <p class="text-muted">Native macOS icons rendered via Swift bridge</p>
    </div>

    <div v-if="loading" class="text-muted loading-placeholder">
      Loading icons...
    </div>

    <div v-else class="icon-list-container">
      <div class="icon-list">
        <template v-for="(icon, idx) in icons" :key="icon.key">
          <div v-if="icon.key === 'other_users' && idx > 0" class="icon-list-divider"></div>
          <div v-if="icon.key === 'docker' && idx > 0" class="icon-list-divider"></div>
          <div v-if="icon.key === 'system_settings' && idx > 0" class="icon-list-divider"></div>
          <div class="icon-list-row">
            <div class="icon-list-preview">
              <img v-if="icon.base64" :src="icon.base64" :alt="icon.label" width="25" height="25" />
              <span v-else class="icon-missing">?</span>
            </div>
            <span class="icon-list-label">{{ icon.label }}</span>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.icon-test {
  max-width: 600px;
}

.icon-list-container {
  background: rgba(255, 255, 255, 0.3);
  border-radius: 12px;
  border: 0.5px solid rgba(255, 255, 255, 0.5);
  padding: 4px 0;
}

.icon-list {
  display: flex;
  flex-direction: column;
}

.icon-list-row {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 8.5px 16px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
}

.icon-list-row:last-child {
  border-bottom: none;
}

.icon-list-preview {
  width: 25px;
  height: 25px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.icon-list-preview img {
  border-radius: 10px;
}

.icon-missing {
  font-size: 20px;
  color: var(--danger);
}

.icon-list-label {
  font-size: 14px;
  font-weight: 400;
  color: var(--text);
}

.icon-list-divider {
  height: 0;
  border-top: 0.5px solid rgba(0, 0, 0, 0.1);
  margin: 4px 16px;
}

.loading-placeholder {
  padding: var(--sp-10);
  text-align: center;
}
</style>
