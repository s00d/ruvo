<script setup lang="ts">
import { computed, ref } from "vue";
import { Server } from "@lucide/vue";
import DataTable from "../components/DataTable.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import SearchField from "../components/SearchField.vue";
import { useDevToolsStore } from "../stores/devtools";
import { fmtMs } from "../types";

const store = useDevToolsStore();
const q = ref("");
const items = computed(() => store.current?.cache ?? []);

const filtered = computed(() => {
  const s = q.value.trim().toLowerCase();
  if (!s) return items.value;
  return items.value.filter(
    (x) =>
      x.op.toLowerCase().includes(s) ||
      x.key.toLowerCase().includes(s) ||
      x.backend.toLowerCase().includes(s),
  );
});

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
  <EmptyState
    v-if="!store.current"
    title="No snapshot"
    hint="Select a request first."
    :icon="Server"
  />
  <EmptyState
    v-else-if="!items.length"
    title="No cache / KV ops"
    hint="Store, Redis, and Cache calls appear when plugins emit sova.store / sova.redis traces."
    :icon="Server"
  />
  <div v-else class="flex flex-col gap-3">
    <SearchField v-model="q" placeholder="Filter op / key / backend…" />
    <Pane
      title="Cache & KV"
      :icon="Server"
      :hint="`${filtered.length} · ${fmtMs(store.cacheTotalMs)}`"
    >
      <DataTable :headers="headers" :rows="rows" :mono-cols="[2]" />
    </Pane>
  </div>
</template>
