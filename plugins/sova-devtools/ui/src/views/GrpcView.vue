<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  Check,
  Copy,
  Play,
  Radio,
  RotateCw,
  Search,
} from "@lucide/vue";
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
const method = ref("hello.Greeter/SayHello");
const body = ref('{\n  "name": "devtools"\n}');
const loading = ref(false);
const error = ref<string | null>(null);
const result = ref("");
const durationMs = ref<number | null>(null);
const copied = ref(false);

const hasConsole = computed(() => {
  const cfg = store.config as { features?: string[] } | null;
  return cfg?.features?.includes("console-grpc") ?? false;
});

const grpcMount = computed(() => {
  const mounts = (store.config as { mounts?: Record<string, unknown> } | null)
    ?.mounts;
  return mounts?.grpc as
    | { client_base?: string; methods?: string[]; bind?: string }
    | undefined;
});

const clientBase = computed(() => grpcMount.value?.client_base ?? "—");

const methodSuggestions = computed(() => {
  const out = new Set<string>();
  for (const m of grpcMount.value?.methods ?? []) out.add(m);
  for (const line of store.current?.grpc ?? []) out.add(line.method);
  if (method.value) out.add(method.value);
  return [...out];
});

const lines = computed(() => store.current?.grpc ?? []);
const filtered = computed(() => {
  const s = q.value.trim().toLowerCase();
  if (!s) return lines.value;
  return lines.value.filter(
    (g) =>
      g.method.toLowerCase().includes(s) ||
      g.base.toLowerCase().includes(s) ||
      g.direction.toLowerCase().includes(s) ||
      String(g.error ?? "").toLowerCase().includes(s),
  );
});
const totalMs = computed(() =>
  lines.value.reduce((a, g) => a + (g.duration_ms ?? 0), 0),
);

watch(
  () => grpcMount.value?.methods?.[0],
  (first) => {
    if (first && method.value === "hello.Greeter/SayHello") {
      method.value = first;
    }
  },
  { immediate: true },
);

function formatBody() {
  try {
    body.value = JSON.stringify(JSON.parse(body.value), null, 2);
    error.value = null;
  } catch {
    error.value = "Body must be valid JSON";
  }
}

async function run() {
  loading.value = true;
  error.value = null;
  result.value = "";
  durationMs.value = null;
  let payload: unknown = {};
  if (body.value.trim()) {
    try {
      payload = JSON.parse(body.value);
    } catch {
      error.value = "Body must be valid JSON";
      loading.value = false;
      return;
    }
  }
  try {
    const res = await postAction(store.api, "grpc", {
      method: method.value,
      body: payload,
    });
    durationMs.value = res.duration_ms ?? null;
    if (!res.ok) {
      error.value = res.error ?? "RPC failed";
    } else {
      result.value = JSON.stringify(res.result, null, 2);
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function prefillFromTrace(line: (typeof lines.value)[number]) {
  method.value = line.method;
  if (!hasConsole.value) return;
}

async function copyResult() {
  if (!result.value) return;
  try {
    await navigator.clipboard.writeText(result.value);
    copied.value = true;
    setTimeout(() => {
      copied.value = false;
    }, 1200);
  } catch {
    /* ignore */
  }
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <Pane
      v-if="hasConsole"
      title="gRPC Playground"
      :icon="Radio"
      :hint="clientBase"
    >
      <div
        class="grid min-h-[280px] grid-cols-1 gap-3 lg:grid-cols-2 lg:gap-0 lg:divide-x lg:divide-[var(--dt-border)]"
      >
        <div class="flex min-h-0 flex-col gap-2 lg:pr-3">
          <label class="text-[10px] font-medium uppercase tracking-wide text-[var(--dt-faint)]"
            >Method</label
          >
          <input
            v-model="method"
            list="dt-grpc-methods"
            type="text"
            placeholder="package.Service/Method"
            class="w-full rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-3 py-2 font-mono text-[12px] text-[var(--dt-text)]"
          />
          <datalist id="dt-grpc-methods">
            <option v-for="m in methodSuggestions" :key="m" :value="m" />
          </datalist>
          <div class="flex items-center justify-between gap-2">
            <span class="text-[10px] text-[var(--dt-muted)]">JSON request body</span>
            <button
              type="button"
              class="inline-flex items-center gap-1 rounded border border-[var(--dt-border)] px-2 py-0.5 text-[10px] text-[var(--dt-muted)] hover:text-[var(--dt-text)]"
              @click="formatBody"
            >
              <RotateCw class="size-3" /> Format
            </button>
          </div>
          <textarea
            v-model="body"
            rows="8"
            spellcheck="false"
            class="min-h-[140px] flex-1 resize-y rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] p-2 font-mono text-[11px] leading-relaxed"
          />
          <button
            type="button"
            class="inline-flex items-center gap-1.5 self-start rounded bg-[var(--dt-accent)] px-4 py-1.5 text-[11px] font-semibold text-white disabled:opacity-50"
            :disabled="loading"
            @click="run"
          >
            <Play class="size-3.5" />
            {{ loading ? "Calling…" : "Invoke" }}
          </button>
          <p v-if="error" class="text-[11px] text-[var(--dt-err)]">{{ error }}</p>
        </div>

        <div class="flex min-h-0 flex-col gap-2 lg:pl-3">
          <div class="flex flex-wrap items-center gap-2">
            <span class="text-[10px] font-medium uppercase tracking-wide text-[var(--dt-faint)]"
              >Response</span
            >
            <Chip
              v-if="durationMs != null"
              label="duration"
              :value="fmtMs(durationMs)"
              tone="info"
            />
            <button
              v-if="result"
              type="button"
              class="ml-auto inline-flex items-center gap-1 rounded border border-[var(--dt-border)] px-2 py-0.5 text-[10px] text-[var(--dt-muted)] hover:text-[var(--dt-text)]"
              @click="copyResult"
            >
              <Check v-if="copied" class="size-3 text-[var(--dt-ok)]" />
              <Copy v-else class="size-3" />
              {{ copied ? "Copied" : "Copy" }}
            </button>
          </div>
          <div
            v-if="!result && !loading"
            class="flex flex-1 items-center justify-center rounded border border-dashed border-[var(--dt-border)] bg-[var(--dt-bg)] p-6 text-center text-[11px] text-[var(--dt-muted)]"
          >
            Connect-JSON response appears here
          </div>
          <CodeBlock
            v-else-if="result"
            :code="result"
            title="result"
            language="json"
            class="min-h-0 flex-1"
          />
        </div>
      </div>
    </Pane>

    <EmptyState
      v-else-if="grpcMount"
      title="gRPC client installed"
      hint="Enable devtools-console-grpc to invoke RPC from DevTools."
      :icon="Radio"
    />

    <EmptyState
      v-if="!store.current"
      title="No snapshot"
      hint="Select a request to see traced RPC calls."
      :icon="Radio"
    />
    <EmptyState
      v-else-if="!lines.length"
      title="No gRPC calls on this request"
      hint="Outbound req.grpc().call() and mounted unary handlers emit sova.grpc traces."
      :icon="Radio"
    />
    <template v-else>
      <SearchField v-model="q" placeholder="Filter method / base / direction…" />
      <Pane
        title="RPC traces"
        :icon="Search"
        :hint="`${filtered.length} · ${fmtMs(totalMs)}`"
      >
        <div class="flex flex-col gap-2">
          <button
            v-for="(g, i) in filtered"
            :key="i"
            type="button"
            class="rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-bg)] p-2.5 text-left transition-colors hover:border-[var(--dt-accent)]"
            @click="prefillFromTrace(g)"
          >
            <div class="flex flex-wrap items-center gap-2">
              <span
                class="dt-mono rounded border px-1.5 text-[10px] uppercase"
                :class="
                  g.direction === 'server'
                    ? 'border-[var(--dt-info)] text-[var(--dt-info)]'
                    : 'border-[var(--dt-accent)] text-[var(--dt-accent)]'
                "
                >{{ g.direction }}</span
              >
              <span
                class="truncate font-mono text-[12px] font-medium text-[var(--dt-text)]"
                >{{ g.method }}</span
              >
              <span class="dt-mono ml-auto text-[11px] text-[var(--dt-muted)]">{{
                fmtMs(g.duration_ms)
              }}</span>
            </div>
            <div class="mt-2 flex flex-wrap gap-2">
              <Chip label="base" :value="g.base" tone="default" />
              <Chip
                v-if="g.status != null"
                label="status"
                :value="String(g.status)"
                :tone="g.ok ? 'ok' : 'err'"
              />
              <Chip
                v-if="g.bytes_in != null"
                label="in"
                :value="`${g.bytes_in} B`"
                tone="info"
              />
              <Chip
                v-if="g.bytes_out != null"
                label="out"
                :value="`${g.bytes_out} B`"
                tone="info"
              />
              <Chip v-if="!g.ok" label="error" :value="g.error ?? 'failed'" tone="err" />
            </div>
          </button>
        </div>
      </Pane>
    </template>
  </div>
</template>
