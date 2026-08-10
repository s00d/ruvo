<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Plus, RefreshCw, Trash2 } from "@lucide/vue";
import { postAction } from "../api";
import { useDevToolsStore } from "../stores/devtools";

const store = useDevToolsStore();

const hasSession = computed(() => {
  const cfg = store.config as { features?: string[] } | null;
  return cfg?.features?.includes("console-session") ?? false;
});

const loading = ref(false);
const error = ref<string | null>(null);
const sessionId = ref("");
const userId = ref("");
const rows = ref<{ key: string; value: string }[]>([]);
const newKey = ref("");
const newValue = ref("");

type SessionSnap = {
  session_id?: string;
  user_id?: string | null;
  keys?: Record<string, string>;
};

function applySnap(data: SessionSnap) {
  sessionId.value = data.session_id ?? "";
  userId.value = data.user_id ?? "";
  const keys = data.keys ?? {};
  rows.value = Object.entries(keys).map(([key, value]) => ({ key, value }));
}

async function run(op: string, extra: Record<string, unknown> = {}) {
  loading.value = true;
  error.value = null;
  try {
    const res = await postAction(store.api, "session", { op, ...extra });
    if (!res.ok) {
      error.value = res.error ?? "session action failed";
      return;
    }
    if (op === "destroy") {
      sessionId.value = "";
      userId.value = "";
      rows.value = [];
      return;
    }
    applySnap((res.result ?? {}) as SessionSnap);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function refresh() {
  await run("list");
}

async function saveRow(row: { key: string; value: string }) {
  if (!row.key.trim()) return;
  await run("set", { key: row.key.trim(), value: row.value });
}

async function addKey() {
  if (!newKey.value.trim()) return;
  await run("set", { key: newKey.value.trim(), value: newValue.value });
  newKey.value = "";
  newValue.value = "";
}

async function deleteRow(key: string) {
  await run("del", { key });
}

async function bindUser() {
  if (!userId.value.trim()) return;
  await run("bind_user", { user_id: userId.value.trim() });
}

onMounted(() => {
  if (hasSession.value) void refresh();
});
</script>

<template>
  <div
    v-if="hasSession"
    class="flex flex-col gap-3 rounded-[var(--dt-radius)] border border-[var(--dt-border)] bg-[var(--dt-surface)] p-3"
  >
    <div class="flex flex-wrap items-center gap-2">
      <span class="text-[10px] uppercase tracking-wide text-[var(--dt-faint)]"
        >Live session</span
      >
      <span
        v-if="sessionId"
        class="dt-mono truncate text-[10px] text-[var(--dt-muted)]"
        :title="sessionId"
        >{{ sessionId }}</span
      >
      <button
        type="button"
        class="ml-auto inline-flex items-center gap-1 rounded border border-[var(--dt-border)] px-2 py-1 text-[10px] text-[var(--dt-accent)] disabled:opacity-50"
        :disabled="loading"
        @click="refresh"
      >
        <RefreshCw class="size-3" /> Refresh
      </button>
    </div>

    <div class="flex flex-wrap items-end gap-2">
      <label class="flex min-w-[8rem] flex-1 flex-col gap-1 text-[10px] text-[var(--dt-faint)]">
        user_id
        <input
          v-model="userId"
          type="text"
          placeholder="user id"
          class="rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1.5 font-mono text-[11px]"
        />
      </label>
      <button
        type="button"
        class="rounded border border-[var(--dt-border)] px-2 py-1.5 text-[10px] text-[var(--dt-accent)] disabled:opacity-50"
        :disabled="loading || !userId.trim()"
        @click="bindUser"
      >
        Bind user
      </button>
      <button
        type="button"
        class="rounded border border-[var(--dt-border)] px-2 py-1.5 text-[10px] text-[var(--dt-warn)] disabled:opacity-50"
        :disabled="loading"
        @click="run('regenerate')"
      >
        Regenerate
      </button>
      <button
        type="button"
        class="rounded border border-[var(--dt-border)] px-2 py-1.5 text-[10px] text-[var(--dt-err)] disabled:opacity-50"
        :disabled="loading"
        @click="run('clear')"
      >
        Clear
      </button>
    </div>

    <div class="flex flex-col gap-2">
      <div
        v-for="(row, i) in rows"
        :key="`${row.key}-${i}`"
        class="flex items-center gap-2"
      >
        <input
          v-model="row.key"
          type="text"
          readonly
          class="w-28 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1 font-mono text-[11px] text-[var(--dt-muted)]"
        />
        <input
          v-model="row.value"
          type="text"
          class="min-w-0 flex-1 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1 font-mono text-[11px]"
          @keydown.enter="saveRow(row)"
        />
        <button
          type="button"
          class="rounded border border-[var(--dt-border)] px-2 py-1 text-[10px] text-[var(--dt-accent)] disabled:opacity-50"
          :disabled="loading"
          @click="saveRow(row)"
        >
          Save
        </button>
        <button
          type="button"
          class="rounded border border-[var(--dt-border)] p-1 text-[var(--dt-err)] disabled:opacity-50"
          :disabled="loading"
          title="Delete key"
          @click="deleteRow(row.key)"
        >
          <Trash2 class="size-3" />
        </button>
      </div>

      <div class="flex items-center gap-2 border-t border-[var(--dt-border)] pt-2">
        <input
          v-model="newKey"
          type="text"
          placeholder="key"
          class="w-28 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1 font-mono text-[11px]"
        />
        <input
          v-model="newValue"
          type="text"
          placeholder="value"
          class="min-w-0 flex-1 rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1 font-mono text-[11px]"
          @keydown.enter="addKey"
        />
        <button
          type="button"
          class="inline-flex items-center gap-1 rounded bg-[var(--dt-accent)] px-3 py-1 text-[10px] font-semibold text-white disabled:opacity-50"
          :disabled="loading || !newKey.trim()"
          @click="addKey"
        >
          <Plus class="size-3" /> Add
        </button>
      </div>
    </div>

    <p v-if="error" class="text-[11px] text-[var(--dt-err)]">{{ error }}</p>
    <p class="text-[10px] text-[var(--dt-faint)]">
      Edits apply to your browser session (cookie), not the selected snapshot.
    </p>
  </div>
</template>
