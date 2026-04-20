<script setup lang="ts">
import { computed, ref } from 'vue';
import { PanelLeftClose, PanelLeftOpen, Plus } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

const props = defineProps<{
  activePipelineLabel: string;
  flyoutOpen: boolean;
  isNarrowViewport: boolean;
}>();

const emit = defineEmits<{
  (e: 'toggle-flyout'): void;
  (e: 'new-pipeline'): void;
}>();

const pipelineButtonRef = ref<HTMLElement | { $el?: HTMLElement } | null>(null);

const pipelineTooltip = computed(() =>
  props.activePipelineLabel
    ? `Active pipeline: ${props.activePipelineLabel}`
    : 'No active pipeline selected'
);

const isPipelineActive = computed(() => props.flyoutOpen || props.activePipelineLabel.length > 0);

function focusPipelineToggle() {
  if (pipelineButtonRef.value instanceof HTMLElement) {
    pipelineButtonRef.value.focus();
    return;
  }
  pipelineButtonRef.value?.$el?.focus();
}

defineExpose({ focusPipelineToggle });
</script>

<template>
  <TooltipProvider v-if="!isNarrowViewport" :delay-duration="150">
    <nav class="pipeline-icon-rail" aria-label="Pipeline navigation">
      <Tooltip>
        <TooltipTrigger as-child>
          <Button
            ref="pipelineButtonRef"
            type="button"
            class="rail-button"
            variant="ghost"
            size="icon"
            :class="{ 'is-active': isPipelineActive }"
            :aria-expanded="flyoutOpen"
            aria-controls="pipeline-flyout"
            :aria-label="flyoutOpen ? 'Close pipelines flyout' : 'Open pipelines flyout'"
            @click="emit('toggle-flyout')"
          >
            <component :is="flyoutOpen ? PanelLeftClose : PanelLeftOpen" :size="18" />
          </Button>
        </TooltipTrigger>
        <TooltipContent side="right" class="max-w-[220px] rounded-[4px] bg-[var(--surface-highest)] text-[var(--color-text-primary)]">
          {{ pipelineTooltip }}
        </TooltipContent>
      </Tooltip>

      <Tooltip>
        <TooltipTrigger as-child>
          <Button
            type="button"
            class="rail-button"
            variant="ghost"
            size="icon"
            aria-label="Create new pipeline"
            @click="emit('new-pipeline')"
          >
            <Plus :size="18" />
          </Button>
        </TooltipTrigger>
        <TooltipContent side="right" class="rounded-[4px] bg-[var(--surface-highest)] text-[var(--color-text-primary)]">
          Create new pipeline
        </TooltipContent>
      </Tooltip>
    </nav>
  </TooltipProvider>
</template>

<style scoped>
.pipeline-icon-rail {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 6px;
  background: var(--color-bg-elevated);
}

.rail-button {
  position: relative;
  width: 36px;
  height: 36px;
  color: var(--color-text-primary);
  transition: background 0.15s ease, color 0.15s ease;
}

.rail-button::before {
  content: '';
  position: absolute;
  left: -6px;
  top: 7px;
  bottom: 7px;
  width: 2px;
  border-radius: var(--radius-4);
  background: transparent;
}

.rail-button:hover,
.rail-button:focus-visible {
  background: var(--color-bg-surface);
  outline: none;
}

.rail-button.is-active {
  background: color-mix(in srgb, var(--color-brand) 16%, var(--color-bg-elevated));
}

.rail-button.is-active::before {
  background: var(--color-brand);
}
</style>
