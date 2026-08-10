<script setup lang="ts">
import { computed, ref } from "vue";
import { Check, Copy, History, Network, Play, Plus } from "@lucide/vue";
import Chip from "../components/Chip.vue";
import CodeBlock from "../components/CodeBlock.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import SearchField from "../components/SearchField.vue";
import { useDevToolsStore } from "../stores/devtools";
import { usePlaygroundStore } from "../stores/playground";
import { fmtMs, statusTone } from "../types";

const store = useDevToolsStore();
const pg = usePlaygroundStore();
const q = ref("");

const features = computed(
  () =>
    (store.config as { features?: string[] } | null)?.features ?? [],
);

const hasConsole = computed(() => features.value.includes("console"));
const hasHttpTraces = computed(() => features.value.includes("http"));

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

const pathSuggestions = computed(() => {
  const mounts = (store.config as { mounts?: Record<string, unknown> } | null)
    ?.mounts;
  const out = new Set<string>(["/", "/ping", "/graphql", "/graphiql"]);
  if (mounts?.graphql && typeof mounts.graphql === "object") {
    const g = mounts.graphql as Record<string, string>;
    for (const v of Object.values(g)) {
      if (typeof v === "string" && v.startsWith("/")) out.add(v);
    }
  }
  for (const h of pg.history) {
    const p = h.path.split("?")[0];
    if (p) out.add(p);
  }
  return [...out];
});

const responseHeaders = computed(() => {
  if (!pg.response?.headers) return [];
  return Object.entries(pg.response.headers);
});

const copied = ref(false);

async function copyResponse() {
  if (!pg.responseBodyPretty) return;
  try {
    await navigator.clipboard.writeText(pg.responseBodyPretty);
    copied.value = true;
    setTimeout(() => {
      copied.value = false;
    }, 1200);
  } catch {
    /* ignore */
  }
}

function replayTrace(h: { method: string; url: string }) {
  pg.prefill({
    target: "external",
    method: h.method,
    path: h.url,
  });
}
</script>

<template>
  <div class="flex flex-col gap-3">
    <Pane v-if="hasConsole" title="HTTP Client" :icon="Network">
      <div class="flex min-h-[320px] flex-col gap-3">
        <div
          class="flex flex-wrap items-center gap-2 rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-surface)] p-2"
        >
          <select
            v-model="pg.target"
            class="rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1.5 text-[11px]"
            @change="pg.persist()"
          >
            <option value="app">App</option>
            <option value="external">External</option>
          </select>
          <select
            v-model="pg.method"
            class="dt-mono w-24 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1.5 text-[12px] font-semibold text-[var(--dt-accent)]"
            @change="pg.persist()"
          >
            <option v-for="m in pg.methods" :key="m" :value="m">{{ m }}</option>
          </select>
          <input
            v-model="pg.path"
            list="dt-path-suggestions"
            type="text"
            placeholder="/path"
            class="min-w-0 flex-1 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-3 py-1.5 font-mono text-[12px]"
            @change="pg.persist()"
          />
          <datalist id="dt-path-suggestions">
            <option v-for="p in pathSuggestions" :key="p" :value="p" />
          </datalist>
          <button
            type="button"
            class="inline-flex items-center gap-1.5 rounded bg-[var(--dt-accent)] px-4 py-1.5 text-[12px] font-semibold text-white disabled:opacity-50"
            :disabled="pg.loading"
            @click="pg.send(store.api)"
          >
            <Play class="size-3.5" />
            {{ pg.loading ? "Sending…" : "Send" }}
          </button>
        </div>

        <div class="flex min-h-0 flex-1 flex-col gap-3 lg:flex-row">
          <section
            class="flex min-h-0 min-w-0 flex-1 flex-col rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-surface)]"
          >
            <div
              class="flex shrink-0 gap-1 border-b border-[var(--dt-border)] px-2 py-1"
            >
              <button
                v-for="tab in (['query', 'headers', 'body'] as const)"
                :key="tab"
                type="button"
                class="rounded px-2 py-1 text-[11px] capitalize transition-colors"
                :class="
                  pg.section === tab
                    ? 'bg-[var(--dt-accent-dim)] text-[var(--dt-accent)]'
                    : 'text-[var(--dt-muted)] hover:bg-[var(--dt-surface-2)]'
                "
                @click="pg.section = tab"
              >
                {{ tab }}
              </button>
            </div>

            <div class="dt-scroll min-h-0 flex-1 overflow-auto p-2">
              <div v-if="pg.section === 'query'" class="flex flex-col gap-2">
                <div
                  v-for="(row, i) in pg.queryParams"
                  :key="i"
                  class="flex items-center gap-2"
                >
                  <input
                    v-model="row.key"
                    type="text"
                    placeholder="key"
                    class="w-28 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1 font-mono text-[11px]"
                  />
                  <input
                    v-model="row.value"
                    type="text"
                    placeholder="value"
                    class="min-w-0 flex-1 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1 font-mono text-[11px]"
                  />
                  <label
                    class="flex items-center gap-1 text-[10px] text-[var(--dt-muted)]"
                  >
                    <input v-model="row.enabled" type="checkbox" />
                  </label>
                </div>
                <button
                  type="button"
                  class="inline-flex items-center gap-1 self-start text-[11px] text-[var(--dt-accent)]"
                  @click="pg.addQueryParam()"
                >
                  <Plus class="size-3" /> Add param
                </button>
              </div>

              <div
                v-else-if="pg.section === 'headers'"
                class="flex flex-col gap-2"
              >
                <div
                  v-for="(row, i) in pg.headers"
                  :key="i"
                  class="flex items-center gap-2"
                >
                  <input
                    v-model="row.key"
                    type="text"
                    placeholder="header"
                    class="w-36 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1 font-mono text-[11px]"
                  />
                  <input
                    v-model="row.value"
                    type="text"
                    placeholder="value"
                    class="min-w-0 flex-1 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1 font-mono text-[11px]"
                  />
                  <label
                    class="flex items-center gap-1 text-[10px] text-[var(--dt-muted)]"
                  >
                    <input v-model="row.enabled" type="checkbox" />
                  </label>
                </div>
                <button
                  type="button"
                  class="inline-flex items-center gap-1 self-start text-[11px] text-[var(--dt-accent)]"
                  @click="pg.addHeader()"
                >
                  <Plus class="size-3" /> Add header
                </button>
              </div>

              <div v-else class="flex h-full min-h-[140px] flex-col gap-2">
                <div class="flex flex-wrap items-center gap-2">
                  <select
                    v-model="pg.bodyMode"
                    class="rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1 text-[11px]"
                  >
                    <option value="none">No body</option>
                    <option value="json">JSON</option>
                    <option value="raw">Raw</option>
                  </select>
                  <button
                    v-if="pg.bodyMode === 'json'"
                    type="button"
                    class="text-[10px] text-[var(--dt-accent)]"
                    @click="pg.applyJsonBodyPreset()"
                  >
                    + content-type
                  </button>
                  <button
                    v-if="pg.bodyMode === 'json'"
                    type="button"
                    class="text-[10px] text-[var(--dt-muted)]"
                    @click="pg.formatJsonBody()"
                  >
                    Format JSON
                  </button>
                </div>
                <textarea
                  v-if="pg.bodyMode !== 'none'"
                  v-model="pg.body"
                  class="min-h-[120px] flex-1 resize-y rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] p-2 font-mono text-[11px] leading-relaxed"
                  placeholder='{ "key": "value" }'
                  @change="pg.persist()"
                />
                <p v-else class="text-[11px] text-[var(--dt-faint)]">
                  No request body.
                </p>
              </div>
            </div>
          </section>

          <section
            class="flex min-h-0 min-w-0 flex-1 flex-col rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-surface)]"
          >
            <div
              class="flex shrink-0 flex-wrap items-center gap-2 border-b border-[var(--dt-border)] px-3 py-2"
            >
              <template v-if="pg.response">
                <Chip
                  :value="String(pg.response.status)"
                  :tone="pg.responseTone"
                  label="status"
                />
                <span class="text-[11px] text-[var(--dt-muted)]">{{
                  fmtMs(pg.response.duration_ms)
                }}</span>
                <span
                  v-if="pg.response.truncated"
                  class="rounded bg-[var(--dt-warn)]/15 px-1.5 py-0.5 text-[10px] text-[var(--dt-warn)]"
                  >truncated</span
                >
              </template>
              <span v-else class="text-[11px] text-[var(--dt-faint)]"
                >Response will appear here</span
              >
              <button
                v-if="pg.responseBodyPretty"
                type="button"
                class="ml-auto inline-flex items-center gap-1 text-[10px] text-[var(--dt-accent)]"
                @click="copyResponse"
              >
                <Check v-if="copied" class="size-3" />
                <Copy v-else class="size-3" />
                {{ copied ? "Copied" : "Copy" }}
              </button>
            </div>

            <div class="dt-scroll min-h-0 flex-1 overflow-auto p-2">
              <p v-if="pg.error" class="mb-2 text-[12px] text-[var(--dt-err)]">
                {{ pg.error }}
              </p>
              <details
                v-if="responseHeaders.length"
                class="mb-2 text-[11px] text-[var(--dt-muted)]"
              >
                <summary class="cursor-pointer select-none">
                  Response headers
                </summary>
                <table class="mt-1 w-full text-left text-[10px]">
                  <tbody>
                    <tr
                      v-for="[k, v] in responseHeaders"
                      :key="k"
                      class="border-t border-[var(--dt-border)]"
                    >
                      <td class="py-1 pr-2 font-mono text-[var(--dt-accent)]">
                        {{ k }}
                      </td>
                      <td class="py-1 font-mono">{{ v }}</td>
                    </tr>
                  </tbody>
                </table>
              </details>
              <CodeBlock
                v-if="pg.responseBodyPretty"
                :code="pg.responseBodyPretty"
                title="body"
                language="json"
              />
            </div>
          </section>
        </div>

        <div
          v-if="pg.history.length"
          class="shrink-0 rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-surface)] p-2"
        >
          <div
            class="mb-1 flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wide text-[var(--dt-faint)]"
          >
            <History class="size-3" /> Recent
          </div>
          <div class="flex flex-wrap gap-1">
            <button
              v-for="(h, i) in pg.history"
              :key="i"
              type="button"
              class="rounded border border-[var(--dt-border)] px-2 py-0.5 font-mono text-[10px] hover:bg-[var(--dt-surface-2)]"
              @click="pg.restoreHistory(h)"
            >
              <span class="text-[var(--dt-accent)]">{{ h.method }}</span>
              {{ h.path }}
            </button>
          </div>
        </div>
      </div>
    </Pane>

    <template v-if="hasHttpTraces">
      <EmptyState
        v-if="!store.current"
        title="No snapshot"
        hint="Select a request to see outbound HTTP traces."
        :icon="Network"
      />
      <EmptyState
        v-else-if="!lines.length"
        title="No outbound HTTP"
        hint="Client calls for this request appear here."
        :icon="Network"
      />
      <template v-else>
        <SearchField v-model="q" placeholder="Filter traces…" />
        <Pane
          title="Outbound traces"
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
                <span
                  class="dt-mono ml-auto text-[11px] text-[var(--dt-muted)]"
                  >{{ fmtMs(h.duration_ms) }}</span
                >
                <button
                  v-if="hasConsole"
                  type="button"
                  class="rounded border border-[var(--dt-border)] px-2 py-0.5 text-[10px] text-[var(--dt-accent)] hover:bg-[var(--dt-surface-2)]"
                  @click="replayTrace(h)"
                >
                  Replay
                </button>
              </div>
              <div
                class="mt-1 truncate text-[12px] text-[var(--dt-text)]"
                :title="h.url"
              >
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
      </template>
    </template>
  </div>
</template>
