<script setup lang="ts">
import { computed, ref } from "vue";
import { Inbox, Play, Send } from "@lucide/vue";
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
const mode = ref<"publish" | "consume">("publish");
const exchange = ref("amq.direct");
const routingKey = ref("devtools.ping");
const queue = ref("devtools.queue");
const body = ref('{"hello":"rabbit"}');
const loading = ref(false);
const error = ref<string | null>(null);
const result = ref("");

const hasConsole = computed(() => {
  const cfg = store.config as { features?: string[] } | null;
  return cfg?.features?.includes("console-rabbit") ?? false;
});

const rabbitMount = computed(() => {
  const mounts = (store.config as { mounts?: Record<string, unknown> } | null)
    ?.mounts;
  return mounts?.rabbit as { mode?: string } | undefined;
});

const lines = computed(() => store.current?.rabbit ?? []);
const filtered = computed(() => {
  const s = q.value.trim().toLowerCase();
  if (!s) return lines.value;
  return lines.value.filter(
    (r) =>
      r.op.toLowerCase().includes(s) ||
      String(r.exchange ?? "").toLowerCase().includes(s) ||
      String(r.routing_key ?? "").toLowerCase().includes(s) ||
      String(r.queue ?? "").toLowerCase().includes(s),
  );
});
const totalMs = computed(() =>
  lines.value.reduce((a, r) => a + (r.duration_ms ?? 0), 0),
);

async function run() {
  loading.value = true;
  error.value = null;
  result.value = "";
  const payload: Record<string, unknown> = { op: mode.value };
  if (mode.value === "publish") {
    payload.exchange = exchange.value;
    payload.routing_key = routingKey.value;
    payload.body = body.value;
  } else {
    payload.queue = queue.value;
  }
  try {
    const res = await postAction(store.api, "rabbit", payload);
    if (!res.ok) error.value = res.error ?? "failed";
    else result.value = JSON.stringify(res.result, null, 2);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function prefillFromTrace(line: (typeof lines.value)[number]) {
  if (line.op === "consume" && line.queue) {
    mode.value = "consume";
    queue.value = line.queue;
  } else if (line.op === "publish") {
    mode.value = "publish";
    if (line.exchange) exchange.value = line.exchange;
    if (line.routing_key) routingKey.value = line.routing_key;
  }
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <Pane
      v-if="hasConsole"
      title="RabbitMQ Console"
      :icon="Inbox"
      :hint="rabbitMount?.mode ?? 'broker'"
    >
      <div
        class="inline-flex shrink-0 self-start rounded-[var(--dt-radius)] border border-[var(--dt-border)] p-0.5"
      >
        <button
          type="button"
          class="rounded px-3 py-1 text-[11px] font-medium transition-colors"
          :class="
            mode === 'publish'
              ? 'bg-[var(--dt-accent-dim)] text-[var(--dt-accent)]'
              : 'text-[var(--dt-muted)] hover:bg-[var(--dt-surface-2)]'
          "
          @click="mode = 'publish'"
        >
          Publish
        </button>
        <button
          type="button"
          class="rounded px-3 py-1 text-[11px] font-medium transition-colors"
          :class="
            mode === 'consume'
              ? 'bg-[var(--dt-accent-dim)] text-[var(--dt-accent)]'
              : 'text-[var(--dt-muted)] hover:bg-[var(--dt-surface-2)]'
          "
          @click="mode = 'consume'"
        >
          Consume one
        </button>
      </div>

      <div v-if="mode === 'publish'" class="mt-3 flex flex-col gap-2">
        <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
          <input
            v-model="exchange"
            type="text"
            placeholder="exchange"
            class="rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-3 py-2 font-mono text-[11px]"
          />
          <input
            v-model="routingKey"
            type="text"
            placeholder="routing key"
            class="rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-3 py-2 font-mono text-[11px]"
          />
        </div>
        <textarea
          v-model="body"
          rows="4"
          spellcheck="false"
          class="rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] p-2 font-mono text-[11px]"
          placeholder="message body"
        />
      </div>
      <div v-else class="mt-3">
        <input
          v-model="queue"
          type="text"
          placeholder="queue name"
          class="w-full rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-3 py-2 font-mono text-[11px]"
        />
      </div>

      <button
        type="button"
        class="mt-3 inline-flex items-center gap-1.5 self-start rounded bg-[var(--dt-accent)] px-4 py-1.5 text-[11px] font-semibold text-white disabled:opacity-50"
        :disabled="loading"
        @click="run"
      >
        <Play class="size-3.5" />
        {{ loading ? "Running…" : mode === "publish" ? "Publish" : "Consume" }}
      </button>
      <p v-if="error" class="mt-2 text-[11px] text-[var(--dt-err)]">{{ error }}</p>
      <CodeBlock v-if="result" :code="result" title="result" language="json" class="mt-2" />
    </Pane>

    <EmptyState
      v-else-if="rabbitMount"
      title="RabbitMQ installed"
      hint="Enable devtools-console-rabbit to publish/consume from DevTools."
      :icon="Inbox"
    />

    <EmptyState
      v-if="!store.current"
      title="No snapshot"
      hint="Select a request to see AMQP traces."
      :icon="Inbox"
    />
    <EmptyState
      v-else-if="!lines.length"
      title="No RabbitMQ ops on this request"
      hint="req.rabbit().publish() / consume_one() emit sova.rabbit traces."
      :icon="Send"
    />
    <template v-else>
      <SearchField v-model="q" placeholder="Filter op / exchange / queue…" />
      <Pane
        title="AMQP traces"
        :icon="Inbox"
        :hint="`${filtered.length} · ${fmtMs(totalMs)}`"
      >
        <div class="flex flex-col gap-2">
          <button
            v-for="(r, i) in filtered"
            :key="i"
            type="button"
            class="rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-bg)] p-2.5 text-left transition-colors hover:border-[var(--dt-accent)]"
            @click="prefillFromTrace(r)"
          >
            <div class="flex flex-wrap items-center gap-2">
              <span
                class="dt-mono rounded border px-1.5 text-[10px] uppercase"
                :class="
                  r.ok
                    ? 'border-[var(--dt-ok)] text-[var(--dt-ok)]'
                    : 'border-[var(--dt-err)] text-[var(--dt-err)]'
                "
                >{{ r.op }}</span
              >
              <span class="truncate font-mono text-[12px] text-[var(--dt-text)]">
                {{
                  r.op === "publish"
                    ? `${r.exchange ?? "—"} → ${r.routing_key ?? "—"}`
                    : r.queue ?? "—"
                }}
              </span>
              <span class="dt-mono ml-auto text-[11px] text-[var(--dt-muted)]">{{
                fmtMs(r.duration_ms)
              }}</span>
            </div>
            <div v-if="r.bytes != null || r.error" class="mt-2 flex flex-wrap gap-2">
              <Chip
                v-if="r.bytes != null"
                label="bytes"
                :value="String(r.bytes)"
                tone="info"
              />
              <Chip v-if="r.error" label="error" :value="r.error" tone="err" />
            </div>
          </button>
        </div>
      </Pane>
    </template>
  </div>
</template>
