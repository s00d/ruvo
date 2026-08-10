<script setup lang="ts">
import { computed } from "vue";
import { useConsoleStore } from "../stores/console";
import { useDevToolsStore } from "../stores/devtools";

const consoleStore = useConsoleStore();
const store = useDevToolsStore();

const hasRedis = computed(() => {
  const cfg = store.config as { features?: string[] } | null;
  return cfg?.features?.includes("console-redis") ?? false;
});

const needsValue = computed(
  () =>
    consoleStore.redisOp === "set" || consoleStore.redisOp === "publish",
);

const resultText = computed(() => {
  if (!consoleStore.last?.result) return "";
  return JSON.stringify(consoleStore.last.result, null, 2);
});
</script>

<template>
  <div v-if="!hasRedis" class="text-[12px] text-[var(--dt-muted)]">
    Redis console requires <code class="dt-mono">devtools-console-redis</code>.
  </div>
  <div
    v-else
    class="flex min-h-0 flex-1 flex-col gap-3 rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-surface)] p-3"
  >
    <div class="flex flex-wrap items-center gap-2">
      <label class="text-[10px] uppercase tracking-wide text-[var(--dt-faint)]"
        >DB</label
      >
      <select
        :value="consoleStore.redisDb"
        class="dt-mono rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1.5 text-[11px]"
        @change="
          consoleStore.setRedisDb(Number(($event.target as HTMLSelectElement).value))
        "
      >
        <option v-for="n in 16" :key="n - 1" :value="n - 1">{{ n - 1 }}</option>
      </select>
      <select
        v-model="consoleStore.redisOp"
        class="rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1.5 text-[11px] font-medium"
      >
        <option value="get">GET</option>
        <option value="set">SET</option>
        <option value="del">DEL</option>
        <option value="ttl">TTL</option>
        <option value="type">TYPE</option>
        <option value="scan">SCAN</option>
        <option value="publish">PUBLISH</option>
      </select>
      <button
        type="button"
        class="ml-auto rounded bg-[var(--dt-accent)] px-4 py-1.5 text-[11px] font-semibold text-white disabled:opacity-50"
        :disabled="consoleStore.loading"
        @click="consoleStore.runRedis(store.api)"
      >
        {{ consoleStore.loading ? "…" : "Run" }}
      </button>
    </div>

    <div v-if="consoleStore.redisOp === 'scan'" class="flex flex-wrap gap-2">
      <input
        v-model="consoleStore.redisPattern"
        type="text"
        placeholder="pattern *"
        class="min-w-0 flex-1 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1.5 font-mono text-[11px]"
      />
      <input
        v-model.number="consoleStore.redisCursor"
        type="number"
        min="0"
        placeholder="cursor"
        class="w-24 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1.5 font-mono text-[11px]"
      />
    </div>

    <template v-else-if="consoleStore.redisOp !== 'publish'">
      <input
        v-model="consoleStore.redisKey"
        type="text"
        placeholder="key"
        class="w-full rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-3 py-2 font-mono text-[12px]"
      />
    </template>

    <template v-else>
      <input
        v-model="consoleStore.redisChannel"
        type="text"
        placeholder="channel"
        class="w-full rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-3 py-2 font-mono text-[12px]"
      />
    </template>

    <div v-if="needsValue" class="flex flex-wrap gap-2">
      <input
        v-model="consoleStore.redisValue"
        type="text"
        placeholder="value / message"
        class="min-w-0 flex-1 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-3 py-2 font-mono text-[12px]"
      />
      <input
        v-if="consoleStore.redisOp === 'set'"
        v-model.number="consoleStore.redisTtl"
        type="number"
        min="0"
        placeholder="TTL sec"
        class="w-28 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-2 font-mono text-[11px]"
      />
    </div>

    <div
      v-if="consoleStore.redisOp === 'publish'"
      class="flex items-center gap-2 border-t border-[var(--dt-border)] pt-2"
    >
      <button
        type="button"
        class="rounded border border-[var(--dt-border)] px-3 py-1 text-[11px]"
        @click="consoleStore.toggleRedisListen(store.api)"
      >
        {{ consoleStore.redisListening ? "Stop listen" : "Listen (SSE)" }}
      </button>
    </div>

    <ul
      v-if="consoleStore.redisMessages.length"
      class="max-h-32 overflow-auto rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] p-2 text-[11px]"
    >
      <li
        v-for="(m, i) in consoleStore.redisMessages"
        :key="i"
        class="border-b border-[var(--dt-border)] py-1 last:border-0"
      >
        <span class="font-mono text-[var(--dt-accent)]">{{ m.channel }}</span>:
        {{ m.payload }}
      </li>
    </ul>

    <p v-if="consoleStore.error" class="text-[12px] text-[var(--dt-err)]">
      {{ consoleStore.error }}
    </p>
    <pre
      v-if="resultText"
      class="min-h-[80px] flex-1 overflow-auto rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] p-3 font-mono text-[11px] leading-relaxed"
      >{{ resultText }}</pre
    >
  </div>
</template>
