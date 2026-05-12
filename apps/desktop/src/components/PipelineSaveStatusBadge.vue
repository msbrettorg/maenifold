<script setup lang="ts">
import { computed } from 'vue';
import { Button } from '@/components/ui/button';
import type { PipelineSavingState } from '../stores/pipeline-design';

const props = defineProps<{
  designMode: boolean;
  dirty: boolean;
  hasDraftNodes: boolean;
  pipelineName: string;
  savingState: PipelineSavingState;
}>();

const emit = defineEmits<{ (e: 'save'): void }>();

const badgeLabel = computed(() => {
  if (props.savingState === 'saving') return 'Saving…';
  if (props.savingState === 'saved') return 'Saved';
  if (props.savingState === 'error') return 'Error';
  return props.dirty ? 'Unsaved changes' : '';
});

const badgeTone = computed(() => {
  if (props.savingState === 'saved') return 'border-success/40 bg-success/15 text-success';
  if (props.savingState === 'error') return 'border-danger/40 bg-danger/15 text-danger';
  if (props.savingState === 'saving') return 'border-brand/40 bg-brand/15 text-brand';
  return 'border-warning/40 bg-warning/15 text-warning';
});

const showStatus = computed(() =>
  props.designMode || props.savingState === 'saving' || props.savingState === 'saved' || props.savingState === 'error'
);
</script>

<template>
  <div v-if="showStatus" class="flex items-center gap-2 rounded-lg border border-border-subtle/80 bg-surface-container px-3 py-2 shadow-fluent-16" role="group" aria-label="Pipeline save status">
    <span class="inline-flex items-center gap-1.5 text-sm font-medium text-text-primary">
      <span v-if="designMode && dirty" class="size-2 rounded-full bg-warning" aria-label="Unsaved draft" />
      <span>{{ pipelineName }}</span>
    </span>
    <span
      v-if="badgeLabel"
      class="inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium"
      :class="badgeTone"
      role="status"
      aria-live="polite"
    >
      {{ badgeLabel }}
    </span>
    <Button
      v-if="designMode"
      type="button"
      size="sm"
      :disabled="!hasDraftNodes || savingState === 'saving'"
      @click="emit('save')"
    >
      {{ savingState === 'saving' ? 'Saving…' : 'Save pipeline' }}
    </Button>
  </div>
</template>
