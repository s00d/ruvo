<script setup lang="ts">
import { computed } from "vue";
import { Activity, Clock, Database, Network, TriangleAlert } from "@lucide/vue";
import Chip from "../components/Chip.vue";
import DefList from "../components/DefList.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import Waterfall from "../components/Waterfall.vue";
import { useDevToolsStore } from "../stores/devtools";
import { fmtMs, statusTone } from "../types";

const store = useDevToolsStore();
const c = computed(() => store.current);
const tone = computed(() => (c.value ? statusTone(c.value.status) : "ok"));

const otherMs = computed(() => {
  if (!c.value) return 0;
  return Math.max(0, c.value.duration_ms - store.sqlTotalMs - store.httpTotalMs);
});

const segments = computed(() => [
  { label: "SQL", ms: store.sqlTotalMs, color: "var(--dt-info)" },
  { label: "HTTP", ms: store.httpTotalMs, color: "var(--dt-warn)" },
  { label: "App / other", ms: otherMs.value, color: "var(--dt-accent)" },
]);

const defs = computed(() => {
  if (!c.value) return [];
  return [
    { label: "request_id", value: c.value.request_id },
    { label: "snap_id", value: c.value.id },
    { label: "path", value: c.value.path },
    { label: "at_ms", value: String(c.value.at_ms) },
  ];
});
</script>

<template>
  <EmptyState
    v-if="!c"
    title="No request selected"
    hint="Pick a request from the list on the left."
    :icon="Activity"
  />
  <div v-else class="flex flex-col gap-3">
    <Pane title="Overview" :icon="Activity">
      <div class="mb-3 flex flex-wrap gap-2">
        <Chip :value="c.method" tone="info" label="method" />
        <Chip :value="String(c.status)" :tone="tone" label="status" />
        <Chip :value="fmtMs(c.duration_ms)" label="duration" :icon="Clock" />
      </div>
      <DefList :items="defs" />
    </Pane>

    <div class="grid grid-cols-2 gap-2 lg:grid-cols-4">
      <Chip
        class="!flex w-full justify-start"
        label="duration"
        :value="fmtMs(c.duration_ms)"
        :icon="Clock"
        :tone="c.duration_ms > 500 ? 'warn' : 'default'"
      />
      <Chip
        class="!flex w-full justify-start"
        label="sql"
        :value="`${c.queries.length} · ${fmtMs(store.sqlTotalMs)}`"
        :icon="Database"
        tone="info"
      />
      <Chip
        class="!flex w-full justify-start"
        label="http"
        :value="`${c.http.length} · ${fmtMs(store.httpTotalMs)}`"
        :icon="Network"
      />
      <Chip
        class="!flex w-full justify-start"
        label="errors"
        :value="String(store.logErrorCount)"
        :icon="TriangleAlert"
        :tone="store.logErrorCount ? 'err' : 'ok'"
      />
    </div>

    <Pane title="Timing waterfall" :icon="Clock" hint="measured spans">
      <Waterfall :total-ms="c.duration_ms" :segments="segments" />
    </Pane>
  </div>
</template>
