<script setup lang="ts">
import { computed, ref } from "vue";
import { Boxes } from "@lucide/vue";
import DataTable from "../components/DataTable.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import RedisConsole from "../components/RedisConsole.vue";
import SearchField from "../components/SearchField.vue";
import { useDevToolsStore } from "../stores/devtools";
import { fmtMs, sumMs } from "../types";

const store = useDevToolsStore();
const q = ref("");

const hasConsole = computed(() => {
  const cfg = store.config as { features?: string[] } | null;
  return cfg?.features?.includes("console-redis") ?? false;
});

const lines = computed(
  () => store.current?.cache?.filter((x) => x.backend === "redis") ?? [],
);

const filtered = computed(() => {
  const s = q.value.trim().toLowerCase();
  if (!s) return lines.value;
  return lines.value.filter(
    (x) =>
      x.op.toLowerCase().includes(s) ||
      x.key.toLowerCase().includes(s),
  );
});

const totalMs = computed(() => sumMs(lines.value));

const headers = ["op", "key", "bytes", "ms", "ok"];
const rows = computed(() =>
  filtered.value.map((c) => [
    c.op,
    c.key,
    c.bytes == null ? "—" : String(c.bytes),
    fmtMs(c.duration_ms),
    c.ok == null ? "—" : c.ok ? "ok" : "err",
  ]),
);
</script>

<template>
  <div class="flex flex-col gap-3">
    <Pane v-if="hasConsole" title="Redis console" :icon="Boxes">
      <RedisConsole class="min-h-0 flex-1" />
    </Pane>

    <EmptyState
      v-if="!store.current"
      title="No snapshot"
      hint="Select a request to see Redis traces."
      :icon="Boxes"
    />
    <EmptyState
      v-else-if="!lines.length"
      title="No Redis ops on this request"
      hint="sova.redis tracing appears when the Redis plugin is installed."
      :icon="Boxes"
    />
    <template v-else>
      <SearchField v-model="q" placeholder="Filter op / key…" />
      <Pane
        title="Traces"
        :icon="Boxes"
        :hint="`${filtered.length} · ${fmtMs(totalMs)}`"
      >
        <DataTable :headers="headers" :rows="rows" :mono-cols="[1]" />
      </Pane>
    </template>
  </div>
</template>
