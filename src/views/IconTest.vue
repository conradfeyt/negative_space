<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface IconDef {
  key: string;
  label: string;
  name: string;
  mode: string;
  base64: string;
}

const icons = ref<IconDef[]>([]);
const loading = ref(true);

const testIcons: { key: string; label: string; name: string; mode: string }[] = [
  // App icons (by path)
  { key: "appstore", label: "App Store", name: "/System/Applications/App Store.app", mode: "app" },
  { key: "books", label: "Books", name: "/System/Applications/Books.app", mode: "app" },
  { key: "mail", label: "Mail", name: "/System/Applications/Mail.app", mode: "app" },
  { key: "photos", label: "Photos", name: "/System/Applications/Photos.app", mode: "app" },
  { key: "music", label: "Music", name: "/System/Applications/Music.app", mode: "app" },
  { key: "tv", label: "TV", name: "/System/Applications/TV.app", mode: "app" },
  { key: "podcasts", label: "Podcasts", name: "/System/Applications/Podcasts.app", mode: "app" },
  { key: "messages", label: "Messages", name: "/System/Applications/Messages.app", mode: "app" },
  { key: "xcode", label: "Xcode", name: "/Applications/Xcode.app", mode: "app" },
  { key: "docker", label: "Docker", name: "/Applications/Docker.app", mode: "app" },
  { key: "garageband", label: "GarageBand", name: "/Applications/GarageBand.app", mode: "app" },
  { key: "finder", label: "Finder", name: "/System/Library/CoreServices/Finder.app", mode: "app" },
  { key: "sysprefsapp", label: "System Settings", name: "/System/Applications/System Settings.app", mode: "app" },

  // System images (NSImage named)
  { key: "folder", label: "NSFolder", name: "NSFolder", mode: "system" },
  { key: "trash_full", label: "NSTrashFull", name: "NSTrashFull", mode: "system" },
  { key: "trash_empty", label: "NSTrashEmpty", name: "NSTrashEmpty", mode: "system" },
  { key: "computer", label: "NSComputer", name: "NSComputer", mode: "system" },
  { key: "network", label: "NSNetwork", name: "NSNetwork", mode: "system" },
  { key: "user", label: "NSUser", name: "NSUser", mode: "system" },
  { key: "everyone", label: "NSEveryone", name: "NSEveryone", mode: "system" },
  { key: "user_group", label: "NSUserGroup", name: "NSUserGroup", mode: "system" },
  { key: "caution", label: "NSCaution", name: "NSCaution", mode: "system" },
  { key: "info", label: "NSInfo", name: "NSInfo", mode: "system" },
  { key: "bonjour", label: "NSBonjour", name: "NSBonjour", mode: "system" },
  { key: "advanced", label: "NSAdvanced", name: "NSAdvanced", mode: "system" },
  { key: "folder_smart", label: "NSFolderSmart", name: "NSFolderSmart", mode: "system" },

  // SF Symbols
  { key: "sf_ellipsis", label: "ellipsis.circle.fill", name: "ellipsis.circle.fill", mode: "sf" },
  { key: "sf_question", label: "questionmark.folder.fill", name: "questionmark.folder.fill", mode: "sf" },
  { key: "sf_desktop", label: "desktopcomputer", name: "desktopcomputer", mode: "sf" },
  { key: "sf_icloud", label: "icloud.fill", name: "icloud.fill", mode: "sf" },
  { key: "sf_cylinder", label: "cylinder.split.1x2.fill", name: "cylinder.split.1x2.fill", mode: "sf" },
  { key: "sf_internaldrive", label: "internaldrive.fill", name: "internaldrive.fill", mode: "sf" },
  { key: "sf_doc", label: "doc.fill", name: "doc.fill", mode: "sf" },
  { key: "sf_trash", label: "trash.fill", name: "trash.fill", mode: "sf" },
  { key: "sf_wrench", label: "wrench.and.screwdriver.fill", name: "wrench.and.screwdriver.fill", mode: "sf" },
  { key: "sf_shippingbox", label: "shippingbox.fill", name: "shippingbox.fill", mode: "sf" },
  { key: "sf_arrow_app", label: "arrow.down.app.fill", name: "arrow.down.app.fill", mode: "sf" },
  { key: "sf_book", label: "book.fill", name: "book.fill", mode: "sf" },
  { key: "sf_envelope", label: "envelope.fill", name: "envelope.fill", mode: "sf" },
  { key: "sf_photo", label: "photo.on.rectangle.angled", name: "photo.on.rectangle.angled", mode: "sf" },
  { key: "sf_play", label: "play.square.stack.fill", name: "play.square.stack.fill", mode: "sf" },
  { key: "sf_music", label: "music.note", name: "music.note", mode: "sf" },
  { key: "sf_film", label: "film", name: "film", mode: "sf" },
  { key: "sf_person2", label: "person.2.fill", name: "person.2.fill", mode: "sf" },
  { key: "sf_iphone", label: "iphone", name: "iphone", mode: "sf" },
  { key: "sf_bubble", label: "bubble.left.fill", name: "bubble.left.fill", mode: "sf" },
  { key: "sf_mic", label: "mic.fill", name: "mic.fill", mode: "sf" },
  { key: "sf_guitars", label: "guitars.fill", name: "guitars.fill", mode: "sf" },
  { key: "sf_waveform", label: "waveform", name: "waveform", mode: "sf" },
];

onMounted(async () => {
  const results: IconDef[] = [];

  // Load system image names dynamically from Swift bridge
  try {
    const sysNames = await invoke<string[]>("list_system_images");
    for (const name of sysNames) {
      testIcons.push({ key: `sys_${name}`, label: name, name, mode: "system" });
    }
  } catch { /* non-critical */ }

  for (const def of testIcons) {
    try {
      const base64 = await invoke<string>("render_sf_symbol", {
        name: def.name,
        size: 32,
        mode: def.mode,
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

    <div v-if="loading" class="text-muted" style="padding: 40px; text-align: center;">
      Loading icons...
    </div>

    <div v-else>
      <h3 style="margin: 20px 0 10px; font-size: 14px; color: var(--muted);">App Icons (by path)</h3>
      <div class="icon-grid">
        <div v-for="icon in icons.filter(i => i.mode === 'app')" :key="icon.key" class="icon-cell">
          <div class="icon-preview">
            <img v-if="icon.base64" :src="icon.base64" width="48" height="48" />
            <span v-else class="icon-missing">?</span>
          </div>
          <span class="icon-label">{{ icon.label }}</span>
          <span class="icon-path text-muted">{{ icon.name }}</span>
        </div>
      </div>

      <h3 style="margin: 20px 0 10px; font-size: 14px; color: var(--muted);">System Images (NSImage named)</h3>
      <div class="icon-grid">
        <div v-for="icon in icons.filter(i => i.mode === 'system')" :key="icon.key" class="icon-cell">
          <div class="icon-preview">
            <img v-if="icon.base64" :src="icon.base64" width="48" height="48" />
            <span v-else class="icon-missing">?</span>
          </div>
          <span class="icon-label">{{ icon.label }}</span>
        </div>
      </div>

      <h3 style="margin: 20px 0 10px; font-size: 14px; color: var(--muted);">SF Symbols</h3>
      <div class="icon-grid">
        <div v-for="icon in icons.filter(i => i.mode === 'sf')" :key="icon.key" class="icon-cell">
          <div class="icon-preview">
            <img v-if="icon.base64" :src="icon.base64" width="48" height="48" />
            <span v-else class="icon-missing">?</span>
          </div>
          <span class="icon-label">{{ icon.label }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.icon-test {
  max-width: 1440px;
}

.icon-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 12px;
  margin-bottom: 24px;
}

.icon-cell {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 12px 8px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.4);
  border: 0.5px solid rgba(255, 255, 255, 0.5);
}

.icon-preview {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.icon-preview img {
  border-radius: 10px;
}

.icon-missing {
  font-size: 24px;
  color: var(--danger);
}

.icon-label {
  font-size: 11px;
  font-weight: 500;
  color: var(--text);
  text-align: center;
}

.icon-path {
  font-size: 9px;
  text-align: center;
  word-break: break-all;
}
</style>
