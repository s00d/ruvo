<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Send, Zap } from "@lucide/vue";
import DataTable from "../components/DataTable.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import SearchField from "../components/SearchField.vue";
import { fetchCustomEvents, postAction } from "../api";
import { useDevToolsStore } from "../stores/devtools";
import type { CustomEvent } from "../types";

const store = useDevToolsStore();
const events = ref<CustomEvent[]>([]);
const q = ref("");
const kind = ref("demo.event");
const payload = ref('{"hello":true}');
const loading = ref(false);
const emitError = ref<string | null>(null);

const hasEvents = computed(() => {
  const cfg = store.config as { features?: string[] } | null;
  return cfg?.features?.includes("console-events") ?? false;
});

async function load() {
  events.value = await fetchCustomEvents(store.mount.api);
}

onMounted(() => {
  void load();
  store.onCustomEvent = (e) => {
    events.value = [e, ...events.value].slice(0, 100);
  };
});

async function emitEvent() {
  loading.value = true;
  emitError.value = null;
  let pl: unknown = {};
  try {
    pl = JSON.parse(payload.value || "{}");
  } catch {
    emitError.value = "Payload must be valid JSON";
    loading.value = false;
    return;
  }
  try {
    const res = await postAction(store.api, "events", {
      kind: kind.value,
      payload: pl,
    });
    if (!res.ok) emitError.value = res.error ?? "emit failed";
  } catch (e) {
    emitError.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

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
  <div class="flex flex-col gap-3">
    <Pane v-if="hasEvents" title="Emit event" :icon="Zap">
      <div class="flex flex-wrap items-end gap-2">
        <input
          v-model="kind"
          type="text"
          placeholder="kind"
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
          @click="emitEvent"
        >
          <Send class="size-3.5" /> Emit
        </button>
      </div>
      <p v-if="emitError" class="mt-2 text-[11px] text-[var(--dt-err)]">
        {{ emitError }}
      </p>
    </Pane>

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
  </div>
</template>
