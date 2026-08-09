<script setup lang="ts">
import { computed, ref } from "vue";
import { ScrollText } from "@lucide/vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import SearchField from "../components/SearchField.vue";
import { useDevToolsStore } from "../stores/devtools";
import type { LogLine } from "../types";

const store = useDevToolsStore();
const q = ref("");
const level = ref<"all" | "error" | "warn" | "info" | "debug">("all");

const logs = computed<LogLine[]>(() => {
  const fromSnap = store.current?.logs;
  return fromSnap?.length ? fromSnap : store.globalLogs;
});

const filtered = computed(() => {
  let list = logs.value;
  if (level.value !== "all") {
    list = list.filter((l) =>
      String(l.level).toLowerCase().includes(level.value),
    );
  }
  const s = q.value.trim().toLowerCase();
  if (s) {
    list = list.filter(
      (l) =>
        l.message.toLowerCase().includes(s) ||
        l.target.toLowerCase().includes(s),
    );
  }
  return list;
});

const chips = [
  { id: "all" as const, label: "All" },
  { id: "error" as const, label: "Error" },
  { id: "warn" as const, label: "Warn" },
  { id: "info" as const, label: "Info" },
  { id: "debug" as const, label: "Debug" },
];
</script>

<template>
  <div class="flex flex-col gap-3">
    <div class="flex flex-col gap-2 sm:flex-row sm:items-center">
      <SearchField v-model="q" placeholder="Filter logs…" class="flex-1" />
      <div class="flex flex-wrap gap-1">
        <button
          v-for="c in chips"
          :key="c.id"
          type="button"
          class="min-h-8 cursor-pointer rounded border px-2 text-[10px] uppercase"
          :class="
            level === c.id
              ? 'border-[var(--dt-accent)] bg-[var(--dt-accent-dim)] text-[var(--dt-accent)]'
              : 'border-[var(--dt-border)] text-[var(--dt-faint)]'
          "
          @click="level = c.id"
        >
          {{ c.label }}
        </button>
      </div>
    </div>

    <EmptyState
      v-if="!logs.length"
      title="No log lines"
      hint="Tracing output appears here."
      :icon="ScrollText"
    />
    <Pane
      v-else
      title="Logs"
      :icon="ScrollText"
      :hint="`${filtered.length} / ${logs.length}`"
    >
      <div class="flex flex-col gap-2">
        <div
          v-for="(l, i) in filtered"
          :key="i"
          class="rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2.5 py-2"
        >
          <div class="flex flex-wrap items-center gap-2">
            <span
              class="rounded border px-1.5 text-[10px] font-semibold uppercase"
              :class="{
                'border-[var(--dt-err)] text-[var(--dt-err)]': String(l.level)
                  .toUpperCase()
                  .includes('ERROR'),
                'border-[var(--dt-warn)] text-[var(--dt-warn)]': String(l.level)
                  .toUpperCase()
                  .includes('WARN'),
                'border-[var(--dt-info)] text-[var(--dt-info)]': String(l.level)
                  .toUpperCase()
                  .includes('INFO'),
                'border-[var(--dt-border)] text-[var(--dt-muted)]': String(l.level)
                  .toUpperCase()
                  .includes('DEBUG'),
              }"
              >{{ l.level }}</span
            >
            <span class="dt-mono text-[10px] text-[var(--dt-faint)]">{{
              l.target
            }}</span>
            <span
              v-if="l.at_ms != null"
              class="dt-mono ml-auto text-[10px] text-[var(--dt-faint)]"
              >{{ l.at_ms }}</span
            >
          </div>
          <p class="m-0 mt-1 text-[12px] text-[var(--dt-text)]">{{ l.message }}</p>
        </div>
      </div>
    </Pane>
  </div>
</template>
