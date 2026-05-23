<script setup lang="ts">
import { Button } from '@/components/ui/button';

type Status = {
  label: string;
  description: string;
  toneClass: string;
  dotClass: string;
  actionLabel: string;
  action: string;
};

const props = defineProps<{
  status: Status | null;
}>();

const emit = defineEmits<{
  action: [];
}>();
</script>

<template>
  <div
    v-if="props.status"
    id="home-connection-status"
    class="mb-4 flex max-w-[800px] items-center gap-3 rounded-full border px-3 py-1.5 text-xs"
    :class="props.status.toneClass"
    role="status"
  >
    <span class="size-2 rounded-full" :class="props.status.dotClass" />
    <span class="font-semibold">{{ props.status.label }}</span>
    <span class="text-text-tertiary">{{ props.status.description }}</span>
    <Button
      v-if="props.status.actionLabel"
      type="button"
      variant="ghost"
      size="xs"
      class="ml-1 h-5 rounded-full px-2 text-[11px]"
      @click="emit('action')"
    >
      {{ props.status.actionLabel }}
    </Button>
  </div>
</template>
