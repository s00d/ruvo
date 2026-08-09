<script setup lang="ts">
import { computed, ref } from "vue";
import { Database } from "@lucide/vue";
import CodeBlock from "../components/CodeBlock.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import SearchField from "../components/SearchField.vue";
import { groupDuplicates, hashSql, SLOW_SQL_MS } from "../lib/sql";
import { useDevToolsStore } from "../stores/devtools";
import { fmtMs } from "../types";

const store = useDevToolsStore();
const q = ref("");
const queries = computed(() => store.current?.queries ?? []);
const dups = computed(() =>
  groupDuplicates(queries.value.map((x) => x.sql)),
);

const filtered = computed(() => {
  const s = q.value.trim().toLowerCase();
  if (!s) return queries.value;
  return queries.value.filter((x) => x.sql.toLowerCase().includes(s));
});
</script>

<template>
  <EmptyState
    v-if="!store.current"
    title="No snapshot"
    hint="Select a request first."
    :icon="Database"
  />
  <EmptyState
    v-else-if="!queries.length"
    title="No SQL on this request"
    hint="Queries appear when the DB plugin reports timings."
    :icon="Database"
  />
  <div v-else class="flex flex-col gap-3">
    <SearchField v-model="q" placeholder="Filter SQL…" />
    <Pane
      title="Queries"
      :icon="Database"
      :hint="`${filtered.length} · ${fmtMs(store.sqlTotalMs)}`"
    >
      <div class="flex flex-col gap-3">
        <div
          v-for="(query, i) in filtered"
          :key="i"
          class="rounded-[var(--dt-radius)] border p-2"
          :class="
            (query.duration_ms ?? 0) >= SLOW_SQL_MS
              ? 'border-[var(--dt-warn)]'
              : 'border-[var(--dt-border)]'
          "
        >
          <div class="mb-2 flex flex-wrap items-center gap-2">
            <span class="dt-mono text-[11px] text-[var(--dt-text)]">{{
              fmtMs(query.duration_ms)
            }}</span>
            <span
              v-if="query.rows != null"
              class="rounded border border-[var(--dt-border)] px-1.5 text-[10px] text-[var(--dt-faint)]"
              >{{ query.rows }} rows</span
            >
            <span
              v-if="(query.duration_ms ?? 0) >= SLOW_SQL_MS"
              class="rounded border border-[var(--dt-warn)] px-1.5 text-[10px] text-[var(--dt-warn)]"
              >slow</span
            >
            <span
              v-if="(dups.get(hashSql(query.sql)) || 0) > 1"
              class="rounded border border-[var(--dt-err)] px-1.5 text-[10px] text-[var(--dt-err)]"
              >dup ×{{ dups.get(hashSql(query.sql)) }}</span
            >
          </div>
          <CodeBlock :code="query.sql" title="sql" language="sql" />
        </div>
      </div>
    </Pane>
  </div>
</template>
