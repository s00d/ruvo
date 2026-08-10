<script setup lang="ts">
import { computed, ref } from "vue";
import { HardDrive, Play } from "@lucide/vue";
import DataTable from "../components/DataTable.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import { postAction } from "../api";
import { useDevToolsStore } from "../stores/devtools";
import { fmtMs } from "../types";

const store = useDevToolsStore();
const jobName = ref("ping");
const payload = ref("{}");
const loading = ref(false);
const error = ref<string | null>(null);
const lastId = ref<string | null>(null);

const hasTasks = computed(() => {
  const cfg = store.config as { features?: string[] } | null;
  return cfg?.features?.includes("console-tasks") ?? false;
});

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

async function enqueue() {
  loading.value = true;
  error.value = null;
  let data: unknown = {};
  try {
    data = JSON.parse(payload.value || "{}");
  } catch {
    error.value = "Payload must be valid JSON";
    loading.value = false;
    return;
  }
  try {
    const res = await postAction(store.api, "tasks", {
      name: jobName.value,
      payload: data,
    });
    if (!res.ok) error.value = res.error ?? "enqueue failed";
    else {
      const r = res.result as { id?: string };
      lastId.value = r.id ?? null;
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="flex flex-col gap-3">
    <Pane v-if="hasTasks" title="Enqueue job" :icon="HardDrive">
      <div class="flex flex-wrap items-end gap-2">
        <input
          v-model="jobName"
          type="text"
          placeholder="job name"
          class="w-40 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1.5 font-mono text-[11px]"
        />
        <input
          v-model="payload"
          type="text"
          placeholder='{"key":"value"}'
          class="min-w-0 flex-1 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1.5 font-mono text-[11px]"
        />
        <button
          type="button"
          class="inline-flex items-center gap-1 rounded bg-[var(--dt-accent)] px-3 py-1.5 text-[11px] font-semibold text-white disabled:opacity-50"
          :disabled="loading"
          @click="enqueue"
        >
          <Play class="size-3.5" /> Enqueue
        </button>
      </div>
      <p v-if="error" class="mt-2 text-[11px] text-[var(--dt-err)]">{{ error }}</p>
      <p v-if="lastId" class="mt-1 font-mono text-[11px] text-[var(--dt-ok)]">
        id: {{ lastId }}
      </p>
    </Pane>

    <EmptyState
      v-if="!store.current"
      title="No snapshot"
      hint="Select a request first."
      :icon="HardDrive"
    />
    <EmptyState
      v-else-if="!items.length"
      title="No jobs on this request"
      hint="Task enqueue / worker events (sova.tasks) appear here."
      :icon="HardDrive"
    />
    <Pane v-else title="Jobs" :icon="HardDrive" :hint="`${items.length}`">
      <DataTable :headers="headers" :rows="rows" :mono-cols="[2]" />
    </Pane>
  </div>
</template>
