<script setup lang="ts">
import { computed, ref } from 'vue';
import { PanelLeftClose, PanelLeftOpen, Plus } from 'lucide-vue-next';

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
  <nav v-if="!isNarrowViewport" class="pipeline-icon-rail" aria-label="Pipeline navigation">
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
      <span class="rail-tooltip">{{ pipelineTooltip }}</span>
    </button>

    <button
      type="button"
      class="rail-button"
      aria-label="Create new pipeline"
      title="Create new pipeline"
      @click="emit('new-pipeline')"
    >
      <Plus :size="18" />
    </button>
  </nav>
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
  background: var(--bg-panel);
}

.rail-button {
  position: relative;
  width: 36px;
  height: 36px;
  border: 0;
  border-radius: 10px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--text-primary);
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
  border-radius: 999px;
  background: transparent;
}

.rail-button:hover,
.rail-button:focus-visible {
  background: var(--bg-surface);
  outline: none;
}

.rail-button.is-active {
  background: color-mix(in srgb, var(--border-focus) 16%, var(--bg-panel));
}

.rail-button.is-active::before {
  background: var(--border-focus);
}

.rail-tooltip {
  position: absolute;
  left: calc(100% + 10px);
  top: 50%;
  max-width: 220px;
  padding: 0.375rem 0.5rem;
  border: 1px solid var(--border-default);
  border-radius: 0.5rem;
  background: color-mix(in srgb, var(--bg-main) 94%, transparent);
  color: var(--text-primary);
  font-size: var(--text-xs);
  line-height: 1.4;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transform: translate(4px, -50%);
  opacity: 0;
  visibility: hidden;
  pointer-events: none;
  transition: opacity 0.15s ease, transform 0.15s ease, visibility 0s linear 0.15s;
  box-shadow: 0 0.5rem 1rem color-mix(in srgb, var(--text-primary) 12%, transparent);
}

.rail-button:hover .rail-tooltip,
.rail-button:focus-visible .rail-tooltip {
  opacity: 1;
  visibility: visible;
  transform: translate(0, -50%);
  transition: opacity 0.15s ease, transform 0.15s ease;
}
</style>
