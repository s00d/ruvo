<script setup lang="ts">
import { computed, ref } from "vue";
import SearchField from "./SearchField.vue";
import { useDevToolsStore } from "../stores/devtools";
import { usePlaygroundStore } from "../stores/playground";
import { statusTone } from "../types";

const store = useDevToolsStore();
const playground = usePlaygroundStore();
const q = ref("");

const maxMs = computed(() =>
  Math.max(...store.timeline.map((m) => m.duration_ms), 1),
);

const filtered = computed(() => {
  const s = q.value.trim().toLowerCase();
  if (!s) return store.timeline;
  return store.timeline.filter(
    (m) =>
      m.path.toLowerCase().includes(s) ||
      m.method.toLowerCase().includes(s) ||
      String(m.status).includes(s) ||
      m.id.toLowerCase().includes(s),
  );
});
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="shrink-0 border-b border-[var(--dt-border)] p-2">
      <SearchField v-model="q" placeholder="Filter requests…" />
    </div>
    <div class="dt-scroll flex-1 overflow-auto" role="listbox" aria-label="Requests">
      <p
        v-if="!filtered.length"
        class="m-0 p-4 text-center text-[11px] text-[var(--dt-faint)]"
      >
        No requests
      </p>
      <button
        v-for="m in filtered"
        :key="m.id"
        type="button"
        role="option"
        :aria-selected="store.snapId === m.id"
        class="flex w-full min-h-[var(--dt-touch)] cursor-pointer flex-col gap-1 border-0 border-b border-[var(--dt-border)] border-l-[3px] bg-transparent px-2.5 py-2 text-left transition-colors"
        :class="
          store.snapId === m.id
            ? 'border-l-[var(--dt-accent)] bg-[var(--dt-accent-dim)]'
            : 'border-l-transparent hover:bg-[var(--dt-surface-2)]'
        "
        @click="store.openSnap(m.id)"
        @dblclick.prevent="
          playground.prefill({ method: m.method, path: m.path });
          store.setTab('http');
        "
      >
        <div class="flex items-center gap-2">
          <span
            class="dt-mono text-[11px] font-semibold"
            :class="{
              'text-[var(--dt-ok)]': statusTone(m.status) === 'ok',
              'text-[var(--dt-warn)]': statusTone(m.status) === 'warn',
              'text-[var(--dt-err)]': statusTone(m.status) === 'err',
            }"
            >{{ m.status }}</span
          >
          <span class="dt-mono text-[10px] text-[var(--dt-accent)]">{{
            m.method
          }}</span>
          <span class="dt-mono ml-auto text-[10px] text-[var(--dt-muted)]"
            >{{ m.duration_ms.toFixed(0) }}ms</span
          >
        </div>
        <div
          class="truncate text-[11px] text-[var(--dt-text)]"
          :title="m.path"
        >
          {{ m.path }}
        </div>
        <div class="h-0.5 overflow-hidden rounded-sm bg-[var(--dt-bg)]">
          <div
            class="h-full rounded-sm bg-[var(--dt-accent)] opacity-70"
            :style="{
              width: Math.min(100, (m.duration_ms / maxMs) * 100) + '%',
            }"
          />
        </div>
      </button>
    </div>
  </div>
</template>
