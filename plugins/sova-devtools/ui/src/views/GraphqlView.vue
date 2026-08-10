<script setup lang="ts">
import { computed, ref } from "vue";
import { Braces, Play } from "@lucide/vue";
import Chip from "../components/Chip.vue";
import CodeBlock from "../components/CodeBlock.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import SearchField from "../components/SearchField.vue";
import { postAction } from "../api";
import { useDevToolsStore } from "../stores/devtools";
import { fmtMs } from "../types";

const store = useDevToolsStore();
const q = ref("");
const query = ref("query { counter }");
const variables = ref("");
const loading = ref(false);
const error = ref<string | null>(null);
const result = ref("");

const hasGraphql = computed(() => {
  const cfg = store.config as { features?: string[] } | null;
  return cfg?.features?.includes("console-graphql") ?? false;
});

const lines = computed(() => store.current?.graphql ?? []);
const filtered = computed(() => {
  const s = q.value.trim().toLowerCase();
  if (!s) return lines.value;
  return lines.value.filter(
    (g) =>
      g.operation.toLowerCase().includes(s) ||
      g.kind.toLowerCase().includes(s) ||
      String(g.errors).includes(s),
  );
});
const totalMs = computed(() =>
  lines.value.reduce((a, g) => a + (g.duration_ms ?? 0), 0),
);

async function run() {
  loading.value = true;
  error.value = null;
  let vars: unknown = undefined;
  if (variables.value.trim()) {
    try {
      vars = JSON.parse(variables.value);
    } catch {
      error.value = "Variables must be valid JSON";
      loading.value = false;
      return;
    }
  }
  try {
    const res = await postAction(store.api, "graphql", {
      query: query.value,
      variables: vars,
    });
    if (!res.ok) error.value = res.error ?? "failed";
    else result.value = JSON.stringify(res.result, null, 2);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="flex flex-col gap-3">
    <Pane v-if="hasGraphql" title="GraphQL Playground" :icon="Braces">
      <div class="flex flex-col gap-2">
        <textarea
          v-model="query"
          rows="4"
          class="w-full rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] p-2 font-mono text-[11px]"
          placeholder="query { ... }"
        />
        <textarea
          v-model="variables"
          rows="2"
          class="w-full rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] p-2 font-mono text-[10px]"
          placeholder='Variables JSON (optional)'
        />
        <button
          type="button"
          class="inline-flex items-center gap-1 self-start rounded bg-[var(--dt-accent)] px-3 py-1.5 text-[11px] font-semibold text-white disabled:opacity-50"
          :disabled="loading"
          @click="run"
        >
          <Play class="size-3.5" /> Run
        </button>
        <p v-if="error" class="text-[11px] text-[var(--dt-err)]">{{ error }}</p>
        <CodeBlock v-if="result" :code="result" title="response" language="json" />
      </div>
    </Pane>

    <EmptyState
      v-if="!store.current"
      title="No snapshot"
      hint="Select a request to see traced operations."
      :icon="Braces"
    />
    <EmptyState
      v-else-if="!lines.length"
      title="No GraphQL operations on this request"
      hint="POST /graphql queries appear in traces when the server plugin is installed."
      :icon="Braces"
    />
    <template v-else>
      <SearchField v-model="q" placeholder="Filter operations…" />
      <Pane
        title="Operations"
        :icon="Braces"
        :hint="`${filtered.length} · ${fmtMs(totalMs)}`"
      >
        <div class="flex flex-col gap-2">
          <div
            v-for="(g, i) in filtered"
            :key="i"
            class="rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-bg)] p-2.5"
          >
            <div class="flex flex-wrap items-center gap-2">
              <span
                class="dt-mono rounded border border-[var(--dt-border)] px-1.5 text-[10px] uppercase text-[var(--dt-accent)]"
                >{{ g.kind }}</span
              >
              <span class="truncate text-[12px] font-medium text-[var(--dt-text)]">{{
                g.operation
              }}</span>
              <span class="dt-mono ml-auto text-[11px] text-[var(--dt-muted)]">{{
                fmtMs(g.duration_ms)
              }}</span>
            </div>
            <div class="mt-2 flex flex-wrap gap-2">
              <Chip
                v-if="g.errors > 0"
                label="errors"
                :value="String(g.errors)"
                tone="err"
              />
              <Chip
                v-if="g.auth != null"
                label="auth"
                :value="g.auth ? 'header' : 'none'"
                :tone="g.auth ? 'ok' : 'default'"
              />
            </div>
          </div>
        </div>
      </Pane>
    </template>
  </div>
</template>
