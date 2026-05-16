<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  uri: string;
  title: string;
  firstParagraph: string;
  modified: string | null;
}>();

const formattedModified = computed(() => {
  if (!props.modified) return null;
  const date = new Date(props.modified);
  if (Number.isNaN(date.getTime())) return props.modified;
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' }).format(date);
});
</script>

<template>
  <section class="flex min-w-0 flex-col gap-2">
    <header class="min-w-0">
      <p class="truncate text-sm font-medium text-popover-foreground">{{ title || uri }}</p>
    </header>
    <p
      v-if="firstParagraph"
      class="line-clamp-3 whitespace-pre-wrap break-words text-sm text-muted-foreground"
    >
      {{ firstParagraph }}
    </p>
    <p v-if="formattedModified" class="text-xs text-muted-foreground">Updated {{ formattedModified }}</p>
  </section>
</template>
