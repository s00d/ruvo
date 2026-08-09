<script setup lang="ts">
import { computed } from "vue";
import {
  Activity,
  Clock,
  Database,
  Network,
  Server,
  TriangleAlert,
} from "@lucide/vue";
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
  return Math.max(
    0,
    c.value.duration_ms -
      store.sqlTotalMs -
      store.httpTotalMs -
      store.cacheTotalMs,
  );
});

const segments = computed(() => [
  { label: "SQL", ms: store.sqlTotalMs, color: "var(--dt-info)" },
  { label: "HTTP", ms: store.httpTotalMs, color: "var(--dt-warn)" },
  { label: "Cache", ms: store.cacheTotalMs, color: "var(--dt-ok)" },
  { label: "App / other", ms: otherMs.value, color: "var(--dt-accent)" },
]);

const defs = computed(() => {
  if (!c.value) return [];
  const route = c.value.route;
  const rl = c.value.rate_limit;
  const items = [
    { label: "request_id", value: c.value.request_id },
    { label: "snap_id", value: c.value.id },
    { label: "path", value: c.value.path },
    {
      label: "route",
      value: route?.pattern || route?.path || "—",
    },
    { label: "locale", value: c.value.locale || "—" },
    {
      label: "csrf",
      value:
        c.value.csrf == null ? "—" : c.value.csrf ? "present" : "missing",
    },
    { label: "encoding", value: c.value.encoding || "—" },
    {
      label: "rate_limit",
      value: rl
        ? `${rl.remaining ?? "?"}/${rl.limit ?? "?"}${rl.reset != null ? ` reset=${rl.reset}` : ""}`
        : "—",
    },
    { label: "at_ms", value: String(c.value.at_ms) },
  ];
  return items;
});

const captures = computed(() => {
  const caps = c.value?.route?.captures ?? [];
  return caps.map(([k, v]) => ({ label: k, value: v }));
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

    <Pane v-if="captures.length" title="Route captures" hint="MatchedRoute">
      <DefList :items="captures" />
    </Pane>

    <div class="grid grid-cols-2 gap-2 lg:grid-cols-5">
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
        label="cache"
        :value="`${(c.cache ?? []).length} · ${fmtMs(store.cacheTotalMs)}`"
        :icon="Server"
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
