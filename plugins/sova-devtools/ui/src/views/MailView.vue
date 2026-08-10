<script setup lang="ts">
import { computed, ref } from "vue";
import { Mail, Send } from "@lucide/vue";
import DefList from "../components/DefList.vue";
import EmptyState from "../components/EmptyState.vue";
import Pane from "../components/Pane.vue";
import { postAction } from "../api";
import { useDevToolsStore } from "../stores/devtools";

const store = useDevToolsStore();
const to = ref("dev@localhost");
const subject = ref("DevTools test");
const body = ref("Hello from DevTools");
const loading = ref(false);
const error = ref<string | null>(null);

const hasMail = computed(() => {
  const cfg = store.config as { features?: string[] } | null;
  return cfg?.features?.includes("console-mail") ?? false;
});

const items = computed(() => store.current?.mail ?? []);

async function send() {
  loading.value = true;
  error.value = null;
  try {
    const res = await postAction(store.api, "mail", {
      op: "send",
      to: to.value,
      subject: subject.value,
      body: body.value,
    });
    if (!res.ok) error.value = res.error ?? "send failed";
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="flex flex-col gap-3">
    <Pane v-if="hasMail" title="Compose (fake)" :icon="Mail">
      <div class="flex flex-col gap-2">
        <input
          v-model="to"
          type="text"
          placeholder="to"
          class="rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1.5 text-[11px]"
        />
        <input
          v-model="subject"
          type="text"
          placeholder="subject"
          class="rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] px-2 py-1.5 text-[11px]"
        />
        <textarea
          v-model="body"
          rows="3"
          class="rounded border border-[var(--dt-border)] bg-[var(--dt-bg)] p-2 text-[11px]"
        />
        <button
          type="button"
          class="inline-flex items-center gap-1 self-start rounded bg-[var(--dt-accent)] px-3 py-1.5 text-[11px] font-semibold text-white disabled:opacity-50"
          :disabled="loading"
          @click="send"
        >
          <Send class="size-3.5" /> Send
        </button>
        <p v-if="error" class="text-[11px] text-[var(--dt-err)]">{{ error }}</p>
      </div>
    </Pane>

    <EmptyState
      v-if="!store.current"
      title="No snapshot"
      hint="Select a request first."
      :icon="Mail"
    />
    <EmptyState
      v-else-if="!items.length"
      title="No mail recorded"
      hint="Fake/SMTP sends on this request show up here."
      :icon="Mail"
    />
    <div v-else class="flex flex-col gap-3">
      <Pane
        v-for="(m, i) in items"
        :key="i"
        :title="m.subject || '(no subject)'"
        :icon="Mail"
        :hint="m.backend"
      >
        <DefList
          :items="[
            { label: 'to', value: (m.to || []).join(', ') || '—' },
            { label: 'backend', value: m.backend },
            { label: 'subject', value: m.subject, mono: false },
          ]"
        />
      </Pane>
    </div>
  </div>
</template>
