<script setup lang="ts">
import { computed } from "vue";
import { ExternalLink } from "@lucide/vue";
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

function onKey(e: KeyboardEvent) {
  if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    store.toggle();
  }
}

function openInTab(e: Event) {
  e.stopPropagation();
  window.open(SHELL_URL, "_blank", "noopener,noreferrer");
}
</script>

<template>
  <div
    class="fixed inset-x-0 bottom-0 z-[var(--dt-z)] flex h-[var(--dt-bar-h)] cursor-pointer select-none items-center gap-3 border-t border-[var(--dt-border)] bg-[var(--dt-bg)] px-3 text-[var(--dt-text)]"
    role="button"
    tabindex="0"
    title="Toggle Sova DevTools"
    @click="store.toggle()"
    @keydown="onKey"
  >
    <span class="text-[13px] font-bold tracking-wide text-[var(--dt-accent)]"
      >Sova</span
    >
    <span
      class="dt-mono rounded-sm border border-[var(--dt-border)] px-1.5 py-0.5 text-[11px] font-medium"
      :class="{
        'text-[var(--dt-ok)]': tone === 'ok',
        'text-[var(--dt-warn)]': tone === 'warn',
        'text-[var(--dt-err)]': tone === 'err',
      }"
      >{{ store.mount.status || "—" }}</span
    >
    <span class="dt-mono text-[var(--dt-muted)]"
      >{{ store.mount.ms.toFixed(1) }}ms</span
    >
    <span class="dt-mono text-[var(--dt-muted)]"
      >SQL {{ store.mount.sql }}</span
    >
    <span
      class="dt-mono"
      :class="
        store.mount.errors ? 'text-[var(--dt-err)]' : 'text-[var(--dt-muted)]'
      "
      >ERR {{ store.mount.errors }}</span
    >
    <Sparkline
      class="ml-1 hidden sm:block"
      :values="store.sparkFromTimeline"
      :width="88"
      :height="18"
    />
    <span class="ml-auto flex items-center gap-2">
      <button
        v-if="!store.isShell"
        type="button"
        class="inline-flex cursor-pointer items-center gap-1 rounded-sm border border-[var(--dt-border)] bg-[var(--dt-surface)] px-2 py-0.5 text-[11px] text-[var(--dt-muted)] transition-colors hover:border-[var(--dt-border-strong)] hover:text-[var(--dt-text)]"
        title="Open DevTools in a new browser tab"
        @click="openInTab"
      >
        <Icon :icon="ExternalLink" :size="12" />
        New tab
      </button>
      <span class="text-[11px] text-[var(--dt-faint)]">DevTools</span>
    </span>
  </div>
</template>
