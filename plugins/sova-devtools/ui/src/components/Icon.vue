<script setup lang="ts">
import type { Component } from "vue";

defineProps<{
  icon: Component;
  size?: number;
  stroke?: number;
  class?: string;
}>();
</script>

<template>
  <component
    :is="icon"
    :size="size ?? 14"
    :stroke-width="stroke ?? 2"
    class="shrink-0"
    :class="$props.class"
  />
</template>
