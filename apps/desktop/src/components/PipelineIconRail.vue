<script setup lang="ts">
import { computed, ref } from 'vue';
import { PanelLeftClose, PanelLeftOpen, Plus } from 'lucide-vue-next';
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

const pipelineButtonRef = ref<HTMLButtonElement | null>(null);

const pipelineTooltip = computed(() =>
  props.activePipelineLabel
    ? `Active pipeline: ${props.activePipelineLabel}`
    : 'No active pipeline selected'
);

const isPipelineActive = computed(() => props.flyoutOpen || props.activePipelineLabel.length > 0);

function focusPipelineToggle() {
  pipelineButtonRef.value?.focus();
}

defineExpose({ focusPipelineToggle });
</script>

<template>
  <TooltipProvider v-if="!isNarrowViewport" :delay-duration="150">
    <nav class="pipeline-icon-rail" aria-label="Pipeline navigation">
      <Tooltip>
        <TooltipTrigger as-child>
          <button
            ref="pipelineButtonRef"
            type="button"
            class="rail-button"
            :class="{ 'is-active': isPipelineActive }"
            :aria-expanded="flyoutOpen"
            aria-controls="pipeline-flyout"
            :aria-label="flyoutOpen ? 'Close pipelines flyout' : 'Open pipelines flyout'"
            @click="emit('toggle-flyout')"
          >
            <component :is="flyoutOpen ? PanelLeftClose : PanelLeftOpen" :size="18" />
          </button>
        </TooltipTrigger>
        <TooltipContent side="right" class="max-w-[220px] rounded-[4px] bg-[var(--surface-highest)] text-[var(--foreground)]">
          {{ pipelineTooltip }}
        </TooltipContent>
      </Tooltip>

      <Tooltip>
        <TooltipTrigger as-child>
          <button
            type="button"
            class="rail-button"
            aria-label="Create new pipeline"
            @click="emit('new-pipeline')"
          >
            <Plus :size="18" />
          </button>
        </TooltipTrigger>
        <TooltipContent side="right" class="rounded-[4px] bg-[var(--surface-highest)] text-[var(--foreground)]">
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
  background: var(--card);
}

.rail-button {
  position: relative;
  width: 36px;
  height: 36px;
  border: 0;
  border-radius: var(--radius);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--foreground);
  background: transparent;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}

.rail-button::before {
  content: '';
  position: absolute;
  left: -6px;
  top: 7px;
  bottom: 7px;
  width: 2px;
  border-radius: 4px;
  background: transparent;
}

.rail-button:hover,
.rail-button:focus-visible {
  background: var(--muted);
  outline: none;
}

.rail-button.is-active {
  background: color-mix(in srgb, var(--ring) 16%, var(--card));
}

.rail-button.is-active::before {
  background: var(--ring);
}
</style>
