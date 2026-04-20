<script setup lang="ts">
import type { Component } from 'vue';
import { Button } from '@/components/ui/button';

defineProps<{
  icon?: Component;
  title: string;
  description?: string;
  actionLabel?: string;
}>();

const emit = defineEmits<{
  action: [];
}>();
</script>

<template>
  <div class="flex flex-col items-center gap-3 text-center">
    <component
      v-if="icon"
      :is="icon"
      class="size-10 text-text-secondary opacity-40"
    />
    <div class="grid gap-1">
      <p class="text-sm font-medium text-text-primary">{{ title }}</p>
      <p v-if="description" class="text-xs text-text-secondary">{{ description }}</p>
    </div>
    <Button
      v-if="actionLabel"
      variant="outline"
      size="sm"
      class="mt-1 border-border-subtle text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary"
      @click="emit('action')"
    >
      {{ actionLabel }}
    </Button>
  </div>
</template>
