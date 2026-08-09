<script setup lang="ts">
import { computed, ref } from "vue";

const widths = [
  { id: "mobile", label: "375 · phone", w: 375 },
  { id: "tablet", label: "768 · tablet", w: 768 },
  { id: "desktop", label: "1280 · desktop", w: 1280 },
] as const;

const viewport = ref<(typeof widths)[number]["id"]>("desktop");
const frameW = computed(
  () => widths.find((x) => x.id === viewport.value)?.w ?? 1280,
);
const frameKey = ref(0);

function reloadMock() {
  frameKey.value += 1;
}
</script>

<template>
  <div class="flex min-h-screen flex-col bg-[#07080c] text-[#eef0f4]">
    <header
      class="flex flex-wrap items-center gap-3 border-b border-[#2c3340] bg-[#141820] px-4 py-3"
    >
      <strong class="text-[#3dd68c]">Sova DevTools · Playground</strong>
      <div class="flex flex-wrap gap-1">
        <button
          v-for="v in widths"
          :key="v.id"
          type="button"
          class="min-h-9 cursor-pointer rounded-md border px-3 text-[12px]"
          :class="
            viewport === v.id
              ? 'border-[#3dd68c] bg-[color-mix(in_srgb,#3dd68c_16%,transparent)] text-[#3dd68c]'
              : 'border-[#2c3340] text-[#9aa3b2]'
          "
          @click="viewport = v.id"
        >
          {{ v.label }}
        </button>
      </div>
      <button
        type="button"
        class="min-h-9 cursor-pointer rounded-md border border-[#2c3340] bg-[#0b0d12] px-3 text-[12px] text-[#9aa3b2]"
        @click="reloadMock"
      >
        Reload mock
      </button>
    </header>

    <div class="flex flex-1 flex-col gap-4 p-4 lg:flex-row">
      <div class="flex flex-1 justify-center overflow-auto">
        <div
          class="flex flex-col overflow-hidden rounded-xl border-2 border-[#454d5e] bg-[#0b0d12] shadow-2xl"
          :style="{ width: frameW + 'px', maxWidth: '100%', height: '740px' }"
        >
          <div
            class="flex items-center gap-2 border-b border-[#2c3340] bg-[#1a1f2a] px-3 py-1.5 text-[10px] text-[#6b7382]"
          >
            <span class="h-2 w-2 rounded-full bg-[#f07178]" />
            <span class="h-2 w-2 rounded-full bg-[#e6b35a]" />
            <span class="h-2 w-2 rounded-full bg-[#3dd68c]" />
            <span class="ml-2 font-mono">{{ frameW }}px · iframe (real MQ)</span>
          </div>
          <iframe
            :key="frameKey + '-' + viewport"
            title="DevTools preview"
            src="/playground-embed.html"
            class="h-full w-full flex-1 border-0 bg-[#0b0d12]"
          />
        </div>
      </div>

      <aside class="w-full shrink-0 space-y-3 lg:w-64">
        <div class="rounded-md border border-[#2c3340] bg-[#141820] p-3">
          <h2 class="m-0 mb-2 text-[12px] font-semibold text-[#3dd68c]">
            Checklist
          </h2>
          <ul class="m-0 list-disc space-y-1.5 pl-4 text-[11px] text-[#9aa3b2]">
            <li>Chips bordered, left accent visible</li>
            <li>No bare text outside Pane / DefList / CodeBlock</li>
            <li>Touch targets ≥36px on 375</li>
            <li>Tabs scroll on narrow; split on 768+</li>
            <li>SQL slow + duplicate badges</li>
            <li>Search filters Timeline / DB / Logs / HTTP</li>
            <li>Copy buttons work</li>
          </ul>
        </div>
        <div
          class="rounded-md border border-[#2c3340] bg-[#141820] p-3 text-[11px] text-[#6b7382]"
        >
          <code class="font-mono text-[#9aa3b2]">npm run playground</code>
        </div>
      </aside>
    </div>
  </div>
</template>
