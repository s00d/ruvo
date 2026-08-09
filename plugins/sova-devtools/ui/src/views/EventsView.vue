<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Zap } from "@lucide/vue";
import DataTable from "../components/DataTable.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import SearchField from "../components/SearchField.vue";
import { fetchCustomEvents } from "../api";
import { useDevToolsStore } from "../stores/devtools";
import type { CustomEvent } from "../types";

const store = useDevToolsStore();
const events = ref<CustomEvent[]>([]);
const q = ref("");

async function load() {
  events.value = await fetchCustomEvents(store.mount.api);
}

onMounted(() => {
  void load();
  store.onCustomEvent = (e) => {
    events.value = [e, ...events.value].slice(0, 100);
  };
});

const filtered = computed(() => {
  const s = q.value.trim().toLowerCase();
  if (!s) return events.value;
  return events.value.filter(
    (e) =>
      e.kind.toLowerCase().includes(s) ||
      JSON.stringify(e.payload).toLowerCase().includes(s),
  );
});

const headers = ["kind", "payload", "time"];
const rows = computed(() =>
  filtered.value.map((e) => [
    e.kind,
    JSON.stringify(e.payload),
    new Date(e.ts_ms).toLocaleTimeString(),
  ]),
);
</script>

<template>
  <EmptyState
    v-if="!events.length"
    title="No custom events"
    hint="hub.emit(kind, payload) or auth/mail EventBus forwards appear here."
    :icon="Zap"
  />
  <div v-else class="flex flex-col gap-3">
    <SearchField v-model="q" placeholder="Filter kind / payload…" />
    <Pane title="Events" :icon="Zap" :hint="`${filtered.length}`">
      <DataTable :headers="headers" :rows="rows" :mono-cols="[1]" />
    </Pane>
  </div>
</template>
