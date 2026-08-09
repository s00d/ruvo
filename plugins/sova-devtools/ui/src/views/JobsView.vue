<script setup lang="ts">
import { computed } from "vue";
import { HardDrive } from "@lucide/vue";
import DefList from "../components/DefList.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import { useDevToolsStore } from "../stores/devtools";

const store = useDevToolsStore();
const items = computed(() => store.current?.jobs ?? []);

function paneTone(status: string): "default" | "warn" | "err" {
  const s = status.toLowerCase();
  if (s.includes("fail") || s.includes("error")) return "err";
  if (s.includes("run") || s.includes("pend") || s.includes("queue")) return "warn";
  return "default";
}
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
    hint="Background tasks for this request list here."
    :icon="HardDrive"
  />
  <div v-else class="flex flex-col gap-3">
    <Pane
      v-for="(j, i) in items"
      :key="i"
      :title="j.name"
      :icon="HardDrive"
      :hint="j.status"
      :tone="paneTone(j.status)"
    >
      <DefList
        :items="[
          { label: 'status', value: j.status },
          { label: 'detail', value: j.detail || '—', mono: false },
        ]"
      />
    </Pane>
  </div>
</template>
