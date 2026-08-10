<script setup lang="ts">
import { computed, ref } from "vue";
import { postAction } from "../api";
import { useDevToolsStore } from "../stores/devtools";

const store = useDevToolsStore();
const hasStore = computed(() => {
  const cfg = store.config as { features?: string[] } | null;
  return cfg?.features?.includes("console-store") ?? false;
});
const namespace = ref("default");
const op = ref<"get" | "set" | "del" | "incr">("get");
const key = ref("");
const value = ref("");
const ttl = ref(0);
const loading = ref(false);
const error = ref<string | null>(null);
const result = ref<string>("");

async function run() {
  loading.value = true;
  error.value = null;
  const payload: Record<string, unknown> = {
    namespace: namespace.value,
    op: op.value,
  };
  if (key.value) payload.key = key.value;
  if (value.value) payload.value = value.value;
  if (ttl.value > 0) payload.ttl_secs = ttl.value;
  try {
    const res = await postAction(store.api, "store", payload);
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
  <div
    v-if="hasStore"
    class="flex min-h-0 flex-1 flex-col gap-3 rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-surface)] p-3"
  >
    <div class="flex flex-wrap items-center gap-2">
      <input
        v-model="namespace"
        type="text"
        placeholder="namespace"
        class="w-28 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1.5 font-mono text-[11px]"
      />
      <select
        v-model="op"
        class="rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1.5 text-[11px]"
      >
        <option value="get">GET</option>
        <option value="set">SET</option>
        <option value="del">DEL</option>
        <option value="incr">INCR</option>
      </select>
      <button
        type="button"
        class="ml-auto rounded bg-[var(--dt-accent)] px-4 py-1.5 text-[11px] font-semibold text-white disabled:opacity-50"
        :disabled="loading"
        @click="run"
      >
        Run
      </button>
    </div>
    <input
      v-model="key"
      type="text"
      placeholder="key"
      class="w-full rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-3 py-2 font-mono text-[12px]"
    />
    <div v-if="op === 'set' || op === 'incr'" class="flex gap-2">
      <input
        v-model="value"
        type="text"
        placeholder="value"
        class="min-w-0 flex-1 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-3 py-2 font-mono text-[12px]"
      />
      <input
        v-if="op === 'set'"
        v-model.number="ttl"
        type="number"
        min="0"
        placeholder="TTL"
        class="w-24 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-2 font-mono text-[11px]"
      />
    </div>
    <p v-if="error" class="text-[12px] text-[var(--dt-err)]">{{ error }}</p>
    <pre
      v-if="result"
      class="min-h-[80px] flex-1 overflow-auto rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] p-3 font-mono text-[11px]"
      >{{ result }}</pre
    >
  </div>
</template>
