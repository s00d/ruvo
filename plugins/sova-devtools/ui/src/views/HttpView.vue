<script setup lang="ts">
import { computed, ref } from "vue";
import { Network } from "@lucide/vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import SearchField from "../components/SearchField.vue";
import { useDevToolsStore } from "../stores/devtools";
import { fmtMs, statusTone } from "../types";

const store = useDevToolsStore();
const q = ref("");
const lines = computed(() => store.current?.http ?? []);
const filtered = computed(() => {
  const s = q.value.trim().toLowerCase();
  if (!s) return lines.value;
  return lines.value.filter(
    (h) =>
      h.url.toLowerCase().includes(s) ||
      h.method.toLowerCase().includes(s) ||
      String(h.status ?? "").includes(s) ||
      (h.error || "").toLowerCase().includes(s),
  );
});
</script>

<template>
  <EmptyState
    v-if="!store.current"
    title="No snapshot"
    hint="Select a request first."
    :icon="Network"
  />
  <EmptyState
    v-else-if="!lines.length"
    title="No outbound HTTP"
    hint="Client calls for this request appear here."
    :icon="Network"
  />
  <div v-else class="flex flex-col gap-3">
    <SearchField v-model="q" placeholder="Filter HTTP…" />
    <Pane
      title="Outbound"
      :icon="Network"
      :hint="`${filtered.length} · ${fmtMs(store.httpTotalMs)}`"
    >
      <div class="flex flex-col gap-2">
        <div
          v-for="(h, i) in filtered"
          :key="i"
          class="rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-bg)] p-2.5"
        >
          <div class="flex flex-wrap items-center gap-2">
            <span
              class="dt-mono rounded border border-[var(--dt-border)] px-1.5 text-[10px] text-[var(--dt-accent)]"
              >{{ h.method }}</span
            >
            <span
              v-if="h.status != null"
              class="dt-mono text-[11px]"
              :class="{
                'text-[var(--dt-ok)]': statusTone(h.status) === 'ok',
                'text-[var(--dt-warn)]': statusTone(h.status) === 'warn',
                'text-[var(--dt-err)]': statusTone(h.status) === 'err',
              }"
              >{{ h.status }}</span
            >
            <span class="dt-mono ml-auto text-[11px] text-[var(--dt-muted)]">{{
              fmtMs(h.duration_ms)
            }}</span>
          </div>
          <div class="mt-1 truncate text-[12px] text-[var(--dt-text)]" :title="h.url">
            {{ h.url }}
          </div>
          <div
            v-if="h.error"
            class="mt-1 rounded border border-[var(--dt-err)] px-2 py-1 text-[11px] text-[var(--dt-err)]"
          >
            {{ h.error }}
          </div>
        </div>
      </div>
    </Pane>
  </div>
</template>
