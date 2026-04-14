<template>
  <div
    class="nsfw-card card"
    :class="{ 'nsfw-card--selected': selected, 'nsfw-card--dimmed': dimmed }"
    @click="$emit('toggle', $event)"
  >
    <div class="nsfw-card-thumb">
      <img
        v-if="file.thumbnail"
        :src="thumbnailSrc"
        alt=""
        loading="lazy"
        decoding="async"
      />
      <div v-else class="nsfw-card-thumb-placeholder">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
          <circle cx="8.5" cy="8.5" r="1.5"/>
          <polyline points="21 15 16 10 5 21"/>
        </svg>
      </div>
      <div v-if="blurred && file.thumbnail" class="nsfw-card-scrim">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/>
        </svg>
      </div>
    </div>
    <div class="nsfw-card-body">
      <div class="nsfw-card-row">
        <span
          class="nsfw-card-check"
          :class="{ 'nsfw-card-check--on': selected }"
          @click.stop="$emit('toggle', $event)"
        >
          <svg v-if="selected" width="10" height="8" viewBox="0 0 12 10">
            <polyline points="1.5 6 4.5 9 10.5 1" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </span>
        <span class="nsfw-card-name" :title="file.name">{{ file.name }}</span>
        <span
          class="confidence-badge"
          :class="confidenceClass"
        >{{ Math.round(file.score * 100) }}%</span>
      </div>
      <div class="nsfw-card-row nsfw-card-sub">
        <span class="nsfw-card-size">{{ formatSize(file.size) }}</span>
        <span v-if="showPath" class="nsfw-card-path" :title="file.parent_dir">{{ shortPath(file.parent_dir) }}</span>
      </div>
      <div v-if="file.detected_labels?.length" class="nsfw-card-labels">
        <span v-for="label in shortLabels" :key="label" class="nsfw-label-tag">{{ label }}</span>
        <span
          class="nsfw-info-icon"
          @click.stop="$emit('show-info', file)"
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 1a7 7 0 100 14A7 7 0 008 1zm0 2.5a1 1 0 110 2 1 1 0 010-2zM6.5 7h2v5h-2V7z" fill-rule="evenodd"/>
          </svg>
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { formatSize } from "../utils";
import { getLabelWeight } from "../stores/nsfwStore";
import type { NsfwFile } from "../types";

const EXPOSED = new Set([
  "NSFW_SCORE",
  "FEMALE_BREAST_EXPOSED", "BUTTOCKS_EXPOSED", "FEMALE_GENITALIA_EXPOSED",
  "MALE_GENITALIA_EXPOSED", "ANUS_EXPOSED", "MALE_BREAST_EXPOSED",
  "BELLY_EXPOSED", "ARMPITS_EXPOSED",
]);

const props = withDefaults(defineProps<{
  file: NsfwFile;
  selected?: boolean;
  blurred?: boolean;
  showPath?: boolean;
  confidenceFloor?: number;
  dimmed?: boolean;
}>(), {
  selected: false,
  blurred: true,
  showPath: false,
  confidenceFloor: 0.5,
  dimmed: false,
});

defineEmits<{ toggle: [event: MouseEvent]; "show-info": [file: NsfwFile] }>();

const thumbnailSrc = computed(() => {
  const t = props.file.thumbnail;
  if (!t) return "";
  if (t.startsWith("data:")) return t;
  return `data:image/jpeg;base64,${t}`;
});

const confidenceClass = computed(() => {
  if (props.file.score >= 0.9) return "confidence--high";
  if (props.file.score >= 0.7) return "confidence--med";
  return "confidence--low";
});

function shortPath(p: string): string {
  const home = p.match(/^\/Users\/[^/]+/)?.[0];
  if (home) return p.replace(home, "~");
  return p;
}

const shortLabels = computed(() => {
  if (!props.file.detected_labels) return [];
  return props.file.detected_labels
    .map((raw: any) => typeof raw === "string" ? { label: raw, confidence: 0 } : raw)
    .filter((d: any) => {
      const effective = (d.confidence ?? 0) * getLabelWeight(d.label);
      return EXPOSED.has(d.label) && effective >= props.confidenceFloor;
    })
    .sort((a: any, b: any) => (b.confidence ?? 0) - (a.confidence ?? 0))
    .slice(0, 3)
    .map((d: any) => formatLabel(d.label));
});

function formatLabel(l: string): string {
  if (l === "NSFW_SCORE") return "general";
  return l.replace(/_EXPOSED$/, "").replace(/_COVERED$/, "").replace(/_/g, " ").toLowerCase();
}
</script>

<style scoped>
.nsfw-card {
  cursor: pointer;
  overflow: hidden;
  padding: 0;
  contain: layout style paint;
  content-visibility: auto;
  contain-intrinsic-size: 180px 260px;
}

.nsfw-card:hover {
  border-color: var(--accent-light);
}

.nsfw-card--selected {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-light);
}

.nsfw-card--dimmed {
  opacity: 0.35;
  filter: grayscale(0.6);
  transition: opacity 0.2s, filter 0.2s;
}

.nsfw-card--dimmed:hover {
  opacity: 0.7;
  filter: grayscale(0.2);
}

/* Thumbnail */
.nsfw-card-thumb {
  position: relative;
  width: 100%;
  aspect-ratio: 1;
  overflow: hidden;
  background: rgba(0, 0, 0, 0.04);
}

.nsfw-card-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.nsfw-card-scrim {
  position: absolute;
  inset: 0;
  backdrop-filter: blur(20px) brightness(0.7);
  -webkit-backdrop-filter: blur(20px) brightness(0.7);
  background: rgba(40, 40, 40, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.35);
  transition: opacity 0.2s ease;
  z-index: 1;
}

.nsfw-card-scrim svg {
  opacity: 0;
  transition: opacity 0.15s 0s;
  pointer-events: none;
}

.nsfw-card-scrim:hover svg {
  opacity: 1;
  transition: opacity 0.1s 0s;
}

.nsfw-card-scrim:hover {
  opacity: 0;
  transition: opacity 0.3s 0.8s;
}

.nsfw-card-thumb-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--muted);
  opacity: 0.3;
}

/* Body */
.nsfw-card-body {
  padding: 8px 10px;
}

.nsfw-card-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.nsfw-card-sub {
  margin-top: 3px;
}

/* Inline checkbox — no component overhead */
.nsfw-card-check {
  width: 16px;
  height: 16px;
  border-radius: 4px;
  flex-shrink: 0;
  background: rgba(0, 0, 0, 0.08);
  display: flex;
  align-items: center;
  justify-content: center;
}

.nsfw-card-check--on {
  background: var(--accent);
}

.nsfw-card-name {
  font-size: 12px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}

.confidence-badge {
  flex-shrink: 0;
  padding: 1px 7px;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 600;
  color: white;
}

.confidence--high { background: var(--danger); }
.confidence--med  { background: var(--warning); }
.confidence--low  { background: var(--yellow); }

.nsfw-card-size {
  font-size: 11px;
  color: var(--muted);
}

.nsfw-card-path {
  font-size: 10px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}

.nsfw-card-labels {
  display: flex;
  flex-wrap: wrap;
  gap: 3px;
  margin-top: 4px;
}

.nsfw-label-tag {
  font-size: 9px;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  padding: 1px 5px;
  border-radius: 3px;
  background: rgba(0, 0, 0, 0.05);
  color: var(--text-secondary);
  white-space: nowrap;
}

.nsfw-info-icon {
  width: 16px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  color: var(--muted);
  cursor: pointer;
  flex-shrink: 0;
  transition: color 0.15s;
}

.nsfw-info-icon:hover {
  color: var(--accent);
}
</style>
