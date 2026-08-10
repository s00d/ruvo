<script setup lang="ts">
import { computed, ref } from "vue";
import { Archive } from "@lucide/vue";
import DataTable from "../components/DataTable.vue";
import EmptyState from "../components/EmptyState.vue";
import KvConsole from "../components/KvConsole.vue";
import Pane from "../components/Pane.vue";
import SearchField from "../components/SearchField.vue";
import { useDevToolsStore } from "../stores/devtools";
import { fmtMs, sumMs } from "../types";

const store = useDevToolsStore();
const q = ref("");

const hasConsole = computed(() => {
  const cfg = store.config as { features?: string[] } | null;
  return cfg?.features?.includes("console-store") ?? false;
});

const lines = computed(
  () => store.current?.cache?.filter((x) => x.backend !== "redis") ?? [],
);

const filtered = computed(() => {
  const s = q.value.trim().toLowerCase();
  if (!s) return lines.value;
  return lines.value.filter(
    (x) =>
      x.op.toLowerCase().includes(s) ||
      x.key.toLowerCase().includes(s) ||
      x.backend.toLowerCase().includes(s),
  );
});

const totalMs = computed(() => sumMs(lines.value));

const headers = ["op", "backend", "key", "hit", "bytes", "ms", "ok"];
const rows = computed(() =>
  filtered.value.map((c) => [
    c.op,
    c.backend,
    c.key,
    c.hit == null ? "—" : c.hit ? "hit" : "miss",
    c.bytes == null ? "—" : String(c.bytes),
    fmtMs(c.duration_ms),
    c.ok == null ? "—" : c.ok ? "ok" : "err",
  ]),
);
</script>

<template>
  <div class="flex flex-col gap-3">
    <Pane v-if="hasConsole" title="KV console" :icon="Archive">
      <KvConsole class="min-h-0 flex-1" />
    </Pane>

    <EmptyState
      v-if="!store.current"
      title="No snapshot"
      hint="Select a request to see cache / KV traces."
      :icon="Archive"
    />
    <EmptyState
      v-else-if="!lines.length"
      title="No cache / KV ops"
      hint="Store and Cache calls appear when plugins emit sova.store traces."
      :icon="Archive"
    />
    <template v-else>
      <SearchField v-model="q" placeholder="Filter op / key / backend…" />
      <Pane
        title="Traces"
        :icon="Archive"
        :hint="`${filtered.length} · ${fmtMs(totalMs)}`"
      >
        <DataTable :headers="headers" :rows="rows" :mono-cols="[2]" />
      </Pane>
    </template>
  </div>
</template>
