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
    <nav
      class="flex h-full w-full flex-col items-center gap-1.5 bg-bg-elevated p-1.5"
      aria-label="Pipeline navigation"
    >
      <Tooltip>
        <TooltipTrigger as-child>
          <Button
            ref="pipelineButtonRef"
            type="button"
            variant="ghost"
            size="icon"
            class="relative h-9 w-9 text-text-primary transition-colors duration-150 hover:bg-bg-surface focus-visible:bg-bg-surface focus-visible:outline-none"
            :class="isPipelineActive ? 'bg-brand/15' : ''"
            :aria-expanded="flyoutOpen"
            aria-controls="pipeline-flyout"
            :aria-label="flyoutOpen ? 'Close pipelines flyout' : 'Open pipelines flyout'"
            @click="emit('toggle-flyout')"
          >
            <span
              aria-hidden="true"
              class="absolute -left-1.5 top-[7px] bottom-[7px] w-0.5 rounded-sm"
              :class="isPipelineActive ? 'bg-brand' : 'bg-transparent'"
            />
            <component :is="flyoutOpen ? PanelLeftClose : PanelLeftOpen" :size="18" />
          </Button>
        </TooltipTrigger>
        <TooltipContent
          side="right"
          class="max-w-[220px] rounded-sm bg-surface-highest text-text-primary"
        >
          {{ pipelineTooltip }}
        </TooltipContent>
      </Tooltip>

      <Tooltip>
        <TooltipTrigger as-child>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            class="relative h-9 w-9 text-text-primary transition-colors duration-150 hover:bg-bg-surface focus-visible:bg-bg-surface focus-visible:outline-none"
            aria-label="Create new pipeline"
            @click="emit('new-pipeline')"
          >
            <Plus :size="18" />
          </Button>
        </TooltipTrigger>
        <TooltipContent
          side="right"
          class="rounded-sm bg-surface-highest text-text-primary"
        >
          Create new pipeline
        </TooltipContent>
      </Tooltip>
    </nav>
  </TooltipProvider>
</template>
