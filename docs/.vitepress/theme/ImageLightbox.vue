<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute } from "vitepress";

type Item = { src: string; alt: string };

const route = useRoute();
const open = ref(false);
const items = ref<Item[]>([]);
const index = ref(0);

const current = () => items.value[index.value];

function collect() {
  const root = document.querySelector(".vp-doc");
  if (!root) {
    items.value = [];
    return;
  }
  const imgs = Array.from(root.querySelectorAll<HTMLImageElement>("img"));
  const next: Item[] = [];
  for (const img of imgs) {
    const src = img.currentSrc || img.src;
    if (!src) continue;
    // Skip tiny UI icons / brand marks in markdown if any.
    if (/\/icons\//.test(src) || /favicon|logo\.(png|svg)/i.test(src)) continue;
    if (img.closest("a.header-anchor")) continue;
    img.classList.add("vp-lightbox-thumb");
    img.setAttribute("role", "button");
    img.tabIndex = 0;
    img.dataset.lightboxIndex = String(next.length);
    next.push({ src, alt: img.alt || "" });
  }
  items.value = next;
}

function show(i: number) {
  if (!items.value.length) return;
  index.value = ((i % items.value.length) + items.value.length) % items.value.length;
  open.value = true;
  document.documentElement.style.overflow = "hidden";
}

function close() {
  open.value = false;
  document.documentElement.style.overflow = "";
}

function goPrev() {
  show(index.value - 1);
}

function goNext() {
  show(index.value + 1);
}

function onDocClick(e: MouseEvent) {
  const t = e.target;
  if (!(t instanceof Element)) return;
  const img = t.closest("img.vp-lightbox-thumb");
  if (!img || !(img instanceof HTMLImageElement)) return;
  e.preventDefault();
  const i = Number(img.dataset.lightboxIndex ?? "-1");
  if (Number.isFinite(i) && i >= 0) show(i);
}

function onDocKey(e: KeyboardEvent) {
  if (e.key !== "Enter" && e.key !== " ") return;
  const t = e.target;
  if (!(t instanceof HTMLImageElement) || !t.classList.contains("vp-lightbox-thumb")) {
    return;
  }
  e.preventDefault();
  const i = Number(t.dataset.lightboxIndex ?? "-1");
  if (Number.isFinite(i) && i >= 0) show(i);
}

function onGlobalKey(e: KeyboardEvent) {
  if (!open.value) return;
  if (e.key === "Escape") {
    e.preventDefault();
    close();
  } else if (e.key === "ArrowLeft") {
    e.preventDefault();
    goPrev();
  } else if (e.key === "ArrowRight") {
    e.preventDefault();
    goNext();
  }
}

async function refresh() {
  await nextTick();
  // Images may load late (GIF / large PNG).
  collect();
  requestAnimationFrame(collect);
}

onMounted(() => {
  document.addEventListener("click", onDocClick);
  document.addEventListener("keydown", onDocKey);
  window.addEventListener("keydown", onGlobalKey);
  void refresh();
});

onBeforeUnmount(() => {
  document.removeEventListener("click", onDocClick);
  document.removeEventListener("keydown", onDocKey);
  window.removeEventListener("keydown", onGlobalKey);
  document.documentElement.style.overflow = "";
});

watch(
  () => route.path,
  () => {
    close();
    void refresh();
  },
);
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open && current()"
      class="vp-lightbox"
      role="dialog"
      aria-modal="true"
      :aria-label="current()!.alt || 'Image preview'"
      @click.self="close"
    >
      <button type="button" class="vp-lightbox__close" aria-label="Close" @click="close">
        ×
      </button>

      <button
        v-if="items.length > 1"
        type="button"
        class="vp-lightbox__nav vp-lightbox__nav--prev"
        aria-label="Previous image"
        @click.stop="goPrev"
      >
        ‹
      </button>

      <figure class="vp-lightbox__figure" @click.stop>
        <img
          class="vp-lightbox__img"
          :src="current()!.src"
          :alt="current()!.alt"
        />
        <figcaption v-if="current()!.alt || items.length > 1" class="vp-lightbox__caption">
          <span v-if="current()!.alt">{{ current()!.alt }}</span>
          <span v-if="items.length > 1" class="vp-lightbox__count">
            {{ index + 1 }} / {{ items.length }}
          </span>
        </figcaption>
      </figure>

      <button
        v-if="items.length > 1"
        type="button"
        class="vp-lightbox__nav vp-lightbox__nav--next"
        aria-label="Next image"
        @click.stop="goNext"
      >
        ›
      </button>
    </div>
  </Teleport>
</template>
