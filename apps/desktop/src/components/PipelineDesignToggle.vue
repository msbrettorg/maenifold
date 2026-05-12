<script setup lang="ts">
import { PenSquare } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';

const props = defineProps<{ designMode: boolean }>();
const emit = defineEmits<{
  (e: 'update:designMode', value: boolean): void;
}>();

function toggleDesignMode(): void {
  emit('update:designMode', !props.designMode);
}
</script>

<template>
  <Button
    type="button"
    variant="outline"
    size="sm"
    class="gap-2 bg-surface-container"
    aria-label="Toggle pipeline design mode"
    :aria-pressed="props.designMode"
    @click="toggleDesignMode"
  >
    <PenSquare class="size-4" aria-hidden="true" />
    <span>{{ props.designMode ? 'Done' : 'Design' }}</span>
  </Button>
</template>
