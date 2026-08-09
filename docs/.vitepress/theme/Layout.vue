<script setup lang="ts">
import DefaultTheme from "vitepress/theme";
import ImageLightbox from "./ImageLightbox.vue";

const { Layout } = DefaultTheme;
</script>

<template>
  <Layout>
    <template #layout-bottom>
      <ImageLightbox />
    </template>
  </Layout>
</template>
