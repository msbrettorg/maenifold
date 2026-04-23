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
      class="flex h-full w-full flex-col items-center gap-1.5 bg-[var(--color-bg-elevated)] p-1.5"
      aria-label="Pipeline navigation"
    >
      <Tooltip>
        <TooltipTrigger as-child>
          <!-- audit-ok: dynamic-binding (active background uses color-mix() not expressible as a Tailwind utility) -->
          <Button
            ref="pipelineButtonRef"
            type="button"
            class="relative h-9 w-9 text-[var(--color-text-primary)] transition-[background,color] duration-150 hover:bg-[var(--color-bg-surface)] focus-visible:bg-[var(--color-bg-surface)] focus-visible:outline-none before:content-[''] before:absolute before:-left-1.5 before:top-[7px] before:bottom-[7px] before:w-0.5 before:rounded-[var(--radius-4)] before:bg-transparent"
            :class="isPipelineActive ? 'before:!bg-[var(--color-brand)]' : ''"
            variant="ghost"
            size="icon"
            :style="isPipelineActive ? { background: 'color-mix(in srgb, var(--color-brand) 16%, var(--color-bg-elevated))' } : undefined"
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
            class="relative h-9 w-9 text-[var(--color-text-primary)] transition-[background,color] duration-150 hover:bg-[var(--color-bg-surface)] focus-visible:bg-[var(--color-bg-surface)] focus-visible:outline-none before:content-[''] before:absolute before:-left-1.5 before:top-[7px] before:bottom-[7px] before:w-0.5 before:rounded-[var(--radius-4)] before:bg-transparent"
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
