<script setup lang="ts">
import { computed } from "vue";
import {
  Database,
  ExternalLink,
  Maximize2,
  Minimize2,
  TriangleAlert,
} from "@lucide/vue";
import Chip from "./Chip.vue";
import Icon from "./Icon.vue";
import Sparkline from "./Sparkline.vue";
import { useDevToolsStore } from "../stores/devtools";
import { statusTone } from "../types";

const store = useDevToolsStore();
const SHELL_URL = "/_devtools/app";

const tone = computed(() => {
  const m = store.mount;
  return m.status ? statusTone(m.status) : m.statusClass;
});

function onBarClick() {
  store.toggle();
}

function openInTab(e: Event) {
  e.stopPropagation();
  window.open(SHELL_URL, "_blank", "noopener,noreferrer");
}

function toggleFs(e: Event) {
  e.stopPropagation();
  store.toggleFullscreen();
}
</script>

<template>
  <div
    class="fixed inset-x-0 bottom-0 z-[var(--dt-z)] flex h-[var(--dt-bar-h)] select-none items-stretch gap-1.5 border-t border-[var(--dt-border-strong)] bg-[var(--dt-bg)] px-2 py-1 text-[var(--dt-text)]"
    role="toolbar"
    aria-label="Sova DevTools"
  >
    <button
      type="button"
      class="inline-flex min-h-[var(--dt-touch)] shrink-0 cursor-pointer items-center rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-surface)] px-2.5 text-[13px] font-bold tracking-wide text-[var(--dt-accent)] hover:border-[var(--dt-border-strong)]"
      title="Toggle panel"
      @click="onBarClick"
    >
      Sova
    </button>

    <div class="dt-scroll flex min-w-0 flex-1 items-center gap-1.5 overflow-x-auto">
      <Chip
        label="status"
        :value="String(store.mount.status || '—')"
        :tone="tone"
        compact
        @click="onBarClick"
      />
      <Chip
        label="time"
        :value="store.mount.ms.toFixed(1) + 'ms'"
        compact
        @click="onBarClick"
      />
      <Chip
        label="sql"
        :value="String(store.mount.sql)"
        :icon="Database"
        tone="info"
        compact
        @click="onBarClick"
      />
      <Chip
        label="err"
        :value="String(store.mount.errors)"
        :icon="TriangleAlert"
        :tone="store.mount.errors ? 'err' : 'default'"
        compact
        @click="onBarClick"
      />
      <div class="ml-1 hidden items-center sm:flex">
        <Sparkline :values="store.sparkFromTimeline" :width="72" :height="16" />
      </div>
    </div>

    <button
      v-if="!store.isShell"
      type="button"
      class="hidden min-h-[var(--dt-touch)] cursor-pointer items-center gap-1 rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-surface)] px-2 text-[11px] text-[var(--dt-muted)] hover:text-[var(--dt-text)] sm:inline-flex"
      title="Open in new tab"
      @click="openInTab"
    >
      <Icon :icon="ExternalLink" :size="12" />
      Tab
    </button>
    <button
      type="button"
      class="inline-flex min-h-[var(--dt-touch)] cursor-pointer items-center rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-surface)] px-2 text-[var(--dt-muted)] hover:text-[var(--dt-text)]"
      :title="store.fullscreen ? 'Exit fullscreen' : 'Fullscreen panel'"
      @click="toggleFs"
    >
      <Icon :icon="store.fullscreen ? Minimize2 : Maximize2" :size="14" />
    </button>
  </div>
</template>
