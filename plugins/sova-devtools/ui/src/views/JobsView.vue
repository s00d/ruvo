<script setup lang="ts">
import { computed } from "vue";
import { HardDrive } from "@lucide/vue";
import DataTable from "../components/DataTable.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import { useDevToolsStore } from "../stores/devtools";
import { fmtMs } from "../types";

const store = useDevToolsStore();
const items = computed(() => store.current?.jobs ?? []);

const headers = ["name", "status", "detail", "ms"];
const rows = computed(() =>
  items.value.map((j) => [
    j.name,
    j.status,
    j.detail || "—",
    fmtMs(j.duration_ms),
  ]),
);
</script>

<template>
  <EmptyState
    v-if="!store.current"
    title="No snapshot"
    hint="Select a request first."
    :icon="HardDrive"
  />
  <EmptyState
    v-else-if="!items.length"
    title="No jobs"
    hint="Task enqueue / worker events (sova.tasks) for this request."
    :icon="HardDrive"
  />
  <div v-else class="flex flex-col gap-3">
    <Pane title="Jobs" :icon="HardDrive" :hint="`${items.length}`">
      <DataTable :headers="headers" :rows="rows" :mono-cols="[2]" />
    </Pane>
  </div>
</template>
