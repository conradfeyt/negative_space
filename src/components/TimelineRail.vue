<template>
  <nav ref="railEl" class="timeline-rail" aria-label="Timeline navigation">
    <div
      v-for="yg in years"
      :key="yg.year"
      class="tl-year"
      :class="{ 'tl-year--active': yg.year === activeYear }"
    >
      <button
        class="tl-year-btn"
        :class="{ 'tl-year-btn--collapsed': yg.collapsed }"
        :title="`${yg.year} (${yg.totalCount})`"
        @click="yg.collapsed ? $emit('navigate', yg.key) : undefined"
      >
        <span class="tl-year-label">{{ yg.year }}</span>
        <span class="tl-year-count">{{ yg.totalCount }}</span>
      </button>

      <div v-if="!yg.collapsed && yg.months.length > 0" class="tl-months">
        <button
          v-for="(m, mi) in yg.months"
          :key="m.key"
          class="tl-month"
          :class="monthClass(yg, mi)"
          :style="monthStyle(yg, mi)"
          :title="`${m.label} (${m.count})`"
          @click="$emit('navigate', m.key)"
        >
          <span class="tl-month-label">{{ m.label }}</span>
          <span class="tl-month-count">{{ m.count }}</span>
        </button>
      </div>
    </div>
  </nav>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from "vue";

export interface TimelineMonth {
  key: string;
  label: string;
  count: number;
}

export interface TimelineYear {
  year: string;
  key: string;
  totalCount: number;
  collapsed: boolean;
  months: TimelineMonth[];
}

const props = defineProps<{
  years: TimelineYear[];
  activeKey?: string;
}>();

defineEmits<{
  navigate: [key: string];
}>();

const railEl = ref<HTMLElement | null>(null);

// Keep active item centred — driven by scroll, not just activeKey changes
let scrollContainer: Element | null = null;
let ticking = false;

function scrollActiveIntoView() {
  const rail = railEl.value;
  if (!rail) return;
  const active = rail.querySelector(".tl-month--active, .tl-year--active .tl-year-btn") as HTMLElement | null;
  if (!active) return;
  const itemTop = active.offsetTop;
  const itemHeight = active.offsetHeight;
  const railHeight = rail.clientHeight;
  const target = itemTop - railHeight / 2 + itemHeight / 2;
  rail.scrollTop = Math.max(0, target);
}

function onContentScroll() {
  if (ticking) return;
  ticking = true;
  requestAnimationFrame(() => {
    scrollActiveIntoView();
    ticking = false;
  });
}

onMounted(() => {
  scrollContainer = document.querySelector(".content");
  if (scrollContainer) {
    scrollContainer.addEventListener("scroll", onContentScroll, { passive: true });
  }
});

onUnmounted(() => {
  if (scrollContainer) {
    scrollContainer.removeEventListener("scroll", onContentScroll);
  }
});

const activeYear = computed(() => {
  if (!props.activeKey) return "";
  if (/^\d{4}$/.test(props.activeKey)) return props.activeKey;
  const parts = props.activeKey.split(" ");
  return parts[parts.length - 1] || "";
});

function monthClass(yg: TimelineYear, mi: number) {
  const m = yg.months[mi];
  const dist = activeDistance(yg, mi);
  return {
    "tl-month--active": m.key === props.activeKey,
    "tl-month--near": dist === 1,
    "tl-month--far": dist >= 2,
    "tl-month--inactive-year": yg.year !== activeYear.value,
  };
}

function monthStyle(yg: TimelineYear, mi: number): Record<string, string> {
  if (yg.year !== activeYear.value) return {};
  const dist = activeDistance(yg, mi);
  // Fish-eye via font size: active=15, ±1=13, ±2=12, rest=11
  const sizes = [15, 13, 12, 11];
  const fs = sizes[Math.min(dist, sizes.length - 1)];
  return { "--fs": `${fs}px` };
}

function activeDistance(yg: TimelineYear, mi: number): number {
  if (!props.activeKey) return 99;
  const ai = yg.months.findIndex((m) => m.key === props.activeKey);
  if (ai === -1) return 99;
  return Math.abs(mi - ai);
}
</script>

<style scoped>
.timeline-rail {
  position: sticky;
  top: 50px;
  display: flex;
  flex-direction: column;
  gap: 0;
  padding: var(--sp-3) 0;
  min-width: 80px;
  max-width: 110px;
  flex-shrink: 0;
  align-self: flex-start;
  max-height: calc(100vh - 120px);
  overflow-y: auto;
  scrollbar-width: none;
}

.tl-rail::-webkit-scrollbar {
  display: none;
}

/* ── Year group ─────────────────────────────────────── */
.tl-year {
  padding: 0;
}

.tl-year + .tl-year {
  margin-top: 4px;
  padding-top: 4px;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
}

.tl-year-btn {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 6px;
  width: 100%;
  padding: 3px 8px;
  border: none;
  background: none;
  cursor: default;
  text-align: right;
  border-radius: var(--radius-sm);
}

.tl-year-btn--collapsed {
  cursor: pointer;
}

.tl-year-btn--collapsed:hover {
  background: rgba(0, 0, 0, 0.04);
}

.tl-year-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--muted);
  letter-spacing: 0.02em;
  transition: color 0.2s, font-size 0.2s;
}

.tl-year--active > .tl-year-btn .tl-year-label {
  font-size: 12px;
  color: var(--text);
}

.tl-year-count {
  font-size: 10px;
  color: var(--muted);
  transition: color 0.2s;
}

.tl-year--active > .tl-year-btn .tl-year-count {
  color: var(--text-secondary);
}

/* ── Month entries ──────────────────────────────────── */
.tl-months {
  display: flex;
  flex-direction: column;
  padding: 1px 0 2px 0;
}

.tl-month {
  --fs: 11px;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 6px;
  padding: 2px 8px 2px 6px;
  border: none;
  background: none;
  cursor: pointer;
  border-radius: var(--radius-sm);
  text-align: right;
  transition: background 0.12s;
}

.tl-month:hover {
  background: rgba(0, 0, 0, 0.04);
}

.tl-month--active {
  background: rgba(0, 0, 0, 0.06);
}

.tl-month-label {
  font-size: var(--fs);
  color: var(--muted);
  white-space: nowrap;
  transition: font-size 0.2s ease, color 0.2s, font-weight 0.2s;
}

/* Active month — boldest, darkest */
.tl-month--active .tl-month-label {
  font-weight: 700;
  color: var(--text);
}

/* ±1 neighbor */
.tl-month--near .tl-month-label {
  font-weight: 500;
  color: var(--text-secondary);
}

/* Months in non-active years — subdued */
.tl-month--inactive-year .tl-month-label {
  color: var(--muted);
  font-weight: 400;
}

.tl-month-count {
  font-size: 10px;
  color: var(--muted);
  transition: color 0.2s;
}

.tl-month--active .tl-month-count {
  color: var(--text-secondary);
}
</style>
