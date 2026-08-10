<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import Toolbar from "./components/Toolbar.vue";
import TabNav from "./components/TabNav.vue";
import TabSidebar from "./components/TabSidebar.vue";
import Skeleton from "./components/Skeleton.vue";
import RequestList from "./components/RequestList.vue";
import { isTabId } from "./router";
import { useDevToolsStore } from "./stores/devtools";

const store = useDevToolsStore();
const route = useRoute();
const router = useRouter();

const dragging = ref(false);
let startY = 0;
let startH = 0;
/** Skip route → store sync while store drives router.replace. */
const routeSync = ref(false);
let stopAfterEach: (() => void) | null = null;

stopAfterEach = router.afterEach(() => {
  routeSync.value = false;
});

watch(
  () => store.tab,
  (tab) => {
    if (!isTabId(tab) || route.name === tab) return;
    routeSync.value = true;
    void router.replace({ name: tab });
  },
  { immediate: true },
);

watch(
  () => route.name,
  (name) => {
    if (routeSync.value) return;
    if (isTabId(name) && store.tab !== name) {
      store.setTab(name);
    }
  },
);

const panelStyle = computed(() => {
  if (store.isEmbed || store.isPlayground) {
    return { height: "100%", maxHeight: "100%" };
  }
  return {
    height: `${store.effectivePanelH}px`,
    maxHeight: "90dvh",
  };
});

const showResize = computed(() => !store.fullscreen && !store.isPlayground);

function onResizeStart(e: PointerEvent) {
  if (store.fullscreen) return;
  dragging.value = true;
  startY = e.clientY;
  startH = store.panelH;
  window.addEventListener("pointermove", onResizeMove);
  window.addEventListener("pointerup", onResizeEnd);
}

function onResizeMove(e: PointerEvent) {
  if (!dragging.value) return;
  store.setPanelH(startH + (startY - e.clientY));
}

function onResizeEnd() {
  dragging.value = false;
  window.removeEventListener("pointermove", onResizeMove);
  window.removeEventListener("pointerup", onResizeEnd);
}

onMounted(() => {
  // Sync dock iframe height with saved panelH when embedded.
  if (store.isEmbed) {
    store.setPanelH(store.panelH);
  }
});

onBeforeUnmount(() => {
  window.removeEventListener("pointermove", onResizeMove);
  window.removeEventListener("pointerup", onResizeEnd);
  stopAfterEach?.();
});
</script>

<template>
  <Toolbar v-if="!store.isEmbed" />
  <div
    v-show="store.open || store.isEmbed || store.isPlayground"
    class="dt-panel-enter flex flex-col border-t border-[var(--dt-border-strong)] bg-[var(--dt-bg)] text-[var(--dt-text)]"
    :class="
      store.isEmbed || store.isPlayground
        ? 'h-full min-h-0'
        : 'fixed inset-x-0 bottom-[var(--dt-bar-h)] z-[var(--dt-z)]'
    "
    :style="panelStyle"
  >
    <div
      v-if="showResize"
      class="group flex h-2.5 shrink-0 cursor-ns-resize items-center justify-center"
      title="Drag to resize"
      @pointerdown.prevent="onResizeStart"
    >
      <span
        class="h-0.5 w-12 rounded-full bg-[var(--dt-border-strong)] transition-colors group-hover:bg-[var(--dt-accent)]"
        :class="dragging ? '!bg-[var(--dt-accent)]' : ''"
      />
    </div>
    <TabNav v-if="!store.useSidebarNav" />
    <div class="flex min-h-0 flex-1 flex-col md:flex-row">
      <TabSidebar v-if="store.useSidebarNav" />
      <aside
        class="dt-scroll max-h-[40%] shrink-0 overflow-auto border-b border-[var(--dt-border)] md:max-h-none md:w-[min(32%,280px)] md:border-b-0 md:border-r"
      >
        <RequestList />
      </aside>
      <main class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        <div class="dt-scroll min-h-0 flex-1 overflow-auto p-3">
          <Skeleton v-if="store.loading" :rows="5" :cols="2" />
          <div
            v-else-if="store.error"
            class="rounded-[var(--dt-radius)] border border-[var(--dt-err)] bg-[var(--dt-surface)] p-3 text-[var(--dt-err)]"
          >
            {{ store.error }}
          </div>
          <RouterView v-else />
        </div>
      </main>
    </div>
  </div>
</template>
