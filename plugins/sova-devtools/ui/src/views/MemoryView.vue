<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Cpu, Pause, Play } from "@lucide/vue";
import DataTable from "../components/DataTable.vue";
import EmptyState from "../components/EmptyState.vue";
import MetricPill from "../components/MetricPill.vue";
import Pane from "../components/Pane.vue";
import Sparkline from "../components/Sparkline.vue";
import { fetchMemory } from "../api";
import { useDevToolsStore } from "../stores/devtools";
import type { MemorySample } from "../types";

const store = useDevToolsStore();
const samples = ref<MemorySample[]>([]);
const summaryCurrent = ref<number | null>(null);
const summaryPeak = ref<number | null>(null);
const summaryMin = ref<number | null>(null);
const paused = ref(false);

function fmtMiB(bytes: number | null | undefined): string {
  if (bytes == null) return "—";
  return `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
}

function applySummary(s: {
  samples: MemorySample[];
  current: number | null;
  peak: number | null;
  min: number | null;
}) {
  samples.value = s.samples;
  summaryCurrent.value = s.current;
  summaryPeak.value = s.peak;
  summaryMin.value = s.min;
}

function ingestLive(s: MemorySample) {
  if (paused.value) return;
  samples.value = [s, ...samples.value].slice(0, 120);
  if (s.rss_bytes != null) {
    summaryCurrent.value = s.rss_bytes;
    summaryPeak.value =
      summaryPeak.value == null
        ? s.rss_bytes
        : Math.max(summaryPeak.value, s.rss_bytes);
    summaryMin.value =
      summaryMin.value == null
        ? s.rss_bytes
        : Math.min(summaryMin.value, s.rss_bytes);
  }
  if (s.rss_peak_bytes != null) {
    summaryPeak.value =
      summaryPeak.value == null
        ? s.rss_peak_bytes
        : Math.max(summaryPeak.value, s.rss_peak_bytes);
  }
}

async function load() {
  applySummary(await fetchMemory(store.mount.api));
}

onMounted(() => {
  void load();
  store.onMemorySample = ingestLive;
});

onUnmounted(() => {
  if (store.onMemorySample === ingestLive) {
    store.onMemorySample = null;
  }
});

const current = computed(() => summaryCurrent.value);
const peak = computed(() => summaryPeak.value);
const min = computed(() => summaryMin.value);
const delta = computed(() => {
  const c = current.value;
  const m = min.value;
  if (c == null || m == null) return null;
  return c - m;
});

const deltaLabel = computed(() => {
  if (delta.value == null) return "—";
  const sign = delta.value >= 0 ? "+" : "-";
  return `${sign}${(Math.abs(delta.value) / 1024 / 1024).toFixed(1)} MiB`;
});

const sparkValues = computed(() =>
  samples.value
    .slice()
    .reverse()
    .map((s) => s.rss_bytes)
    .filter((v): v is number => v != null)
    .map((v) => v / 1024 / 1024),
);

const headers = ["time", "rss", "peak"];
const rows = computed(() =>
  samples.value.map((s) => [
    new Date(s.ts_ms).toLocaleTimeString(),
    fmtMiB(s.rss_bytes),
    fmtMiB(s.rss_peak_bytes ?? null),
  ]),
);
</script>

<template>
  <EmptyState
    v-if="!samples.length"
    title="No memory samples yet"
    hint="RSS samples arrive every ~2s (Linux / macOS). Pause only freezes the live UI feed."
    :icon="Cpu"
  />
  <div v-else class="flex flex-col gap-3">
    <div class="flex flex-wrap items-center gap-2">
      <button
        type="button"
        class="inline-flex min-h-[var(--dt-touch)] cursor-pointer items-center gap-1.5 rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-surface)] px-2.5 text-[11px] text-[var(--dt-muted)] hover:border-[var(--dt-border-strong)] hover:text-[var(--dt-text)]"
        @click="paused = !paused"
      >
        <component :is="paused ? Play : Pause" :size="13" />
        {{ paused ? "Resume" : "Pause" }}
      </button>
      <span v-if="paused" class="text-[10px] text-[var(--dt-faint)]"
        >Live updates paused · server sampler still running</span
      >
    </div>

    <div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
      <MetricPill label="Current" :value="fmtMiB(current)" :icon="Cpu" tone="info" />
      <MetricPill label="Peak" :value="fmtMiB(peak)" tone="warn" />
      <MetricPill label="Min" :value="fmtMiB(min)" />
      <MetricPill
        label="Delta"
        :value="deltaLabel"
        :tone="delta != null && delta > 0 ? 'warn' : 'ok'"
        hint="vs min in window"
      />
    </div>

    <Pane v-if="sparkValues.length >= 2" title="RSS trend" :icon="Cpu">
      <Sparkline :values="sparkValues" :width="320" :height="48" />
    </Pane>

    <Pane title="Memory" :icon="Cpu" :hint="`${samples.length} samples`">
      <DataTable :headers="headers" :rows="rows" />
    </Pane>
  </div>
</template>
