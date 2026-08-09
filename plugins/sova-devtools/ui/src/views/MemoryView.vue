<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Cpu } from "@lucide/vue";
import DataTable from "../components/DataTable.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import { fetchMemory } from "../api";
import { useDevToolsStore } from "../stores/devtools";
import type { MemorySample } from "../types";

const store = useDevToolsStore();
const samples = ref<MemorySample[]>([]);

async function load() {
  samples.value = await fetchMemory(store.mount.api);
}

onMounted(() => {
  void load();
  store.onMemorySample = (s) => {
    samples.value = [s, ...samples.value].slice(0, 120);
  };
});

const headers = ["time", "rss"];
const rows = computed(() =>
  samples.value.map((s) => [
    new Date(s.ts_ms).toLocaleTimeString(),
    s.rss_bytes == null ? "—" : `${(s.rss_bytes / 1024 / 1024).toFixed(1)} MiB`,
  ]),
);

const sparkPoints = computed(() => {
  const vals = samples.value
    .slice()
    .reverse()
    .map((s) => s.rss_bytes)
    .filter((v): v is number => v != null);
  if (vals.length < 2) return "";
  const min = Math.min(...vals);
  const max = Math.max(...vals);
  const span = Math.max(max - min, 1);
  const w = 240;
  const h = 40;
  return vals
    .map((v, i) => {
      const x = (i / (vals.length - 1)) * w;
      const y = h - ((v - min) / span) * (h - 4) - 2;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
});
</script>

<template>
  <EmptyState
    v-if="!samples.length"
    title="No memory samples yet"
    hint="RSS samples arrive every ~2s (Linux). Custom sampler keeps the feed alive on other OS."
    :icon="Cpu"
  />
  <div v-else class="flex flex-col gap-3">
    <Pane v-if="sparkPoints" title="RSS trend" :icon="Cpu">
      <svg viewBox="0 0 240 40" class="h-10 w-full max-w-sm text-sky-500" aria-hidden="true">
        <polyline
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          :points="sparkPoints"
        />
      </svg>
    </Pane>
    <Pane title="Memory" :icon="Cpu" :hint="`${samples.length} samples`">
      <DataTable :headers="headers" :rows="rows" />
    </Pane>
  </div>
</template>
