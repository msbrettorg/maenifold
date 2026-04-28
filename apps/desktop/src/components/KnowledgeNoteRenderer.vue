<script setup lang="ts">
import MarkdownView from './MarkdownView.vue';

defineProps<{
  content: string;
  clamp?: boolean;
}>();
</script>

<template>
  <MarkdownView :content="content" :clamp="clamp" />
</template>
