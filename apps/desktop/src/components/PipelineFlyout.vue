<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { nextTick, onMounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { RefreshCw } from 'lucide-vue-next';
import PipelineSidebar from './PipelineSidebar.vue';
import { usePipelineStore } from '../stores/pipeline';

const props = defineProps<{
  connectionError: string;
  showErrorState: boolean;
  showLoadingState: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'new-pipeline'): void;
  (e: 'retry'): void;
}>();

const store = usePipelineStore();
const { pipelines } = storeToRefs(store);
const flyoutRef = ref<HTMLElement | null>(null);

function getFocusableElements() {
  return Array.from(
    flyoutRef.value?.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [href], [tabindex]:not([tabindex="-1"])'
    ) ?? []
  ).filter((element) => !element.hasAttribute('disabled') && element.getClientRects().length > 0);
}

function focusInitialElement() {
  nextTick(() => {
    const firstPipelineItem = flyoutRef.value?.querySelector<HTMLElement>('[data-pipeline-item]');
    const fallback = getFocusableElements()[0];
    (firstPipelineItem ?? fallback)?.focus();
  });
}

function focusSiblingPipelineItem(current: HTMLElement, direction: -1 | 1) {
  const items = Array.from(
    flyoutRef.value?.querySelectorAll<HTMLElement>('[data-pipeline-item]') ?? []
  );
  const currentIndex = items.indexOf(current);
  if (currentIndex === -1 || items.length === 0) return;

  const nextIndex = (currentIndex + direction + items.length) % items.length;
  items[nextIndex]?.focus();
}

async function handleRefresh() {
  await store.refreshList();
}

function handleNewPipeline() {
  emit('new-pipeline');
}

function handleKeydown(event: KeyboardEvent) {
  const target = event.target instanceof HTMLElement ? event.target : null;
  if (!target) return;

  if (event.key === 'Tab') {
    const focusableElements = getFocusableElements();
    if (focusableElements.length === 0) return;

    const firstElement = focusableElements[0];
    const lastElement = focusableElements[focusableElements.length - 1];

    if (event.shiftKey && target === firstElement) {
      event.preventDefault();
      lastElement.focus();
    } else if (!event.shiftKey && target === lastElement) {
      event.preventDefault();
      firstElement.focus();
    }
    return;
  }

  const pipelineItem = target.closest<HTMLElement>('[data-pipeline-item]');
  if (!pipelineItem) return;

  if (event.key === 'ArrowDown') {
    event.preventDefault();
    focusSiblingPipelineItem(pipelineItem, 1);
  } else if (event.key === 'ArrowUp') {
    event.preventDefault();
    focusSiblingPipelineItem(pipelineItem, -1);
  } else if (event.key === 'Home') {
    event.preventDefault();
    flyoutRef.value?.querySelector<HTMLElement>('[data-pipeline-item]')?.focus();
  } else if (event.key === 'End') {
    event.preventDefault();
    const items = flyoutRef.value?.querySelectorAll<HTMLElement>('[data-pipeline-item]');
    items?.[items.length - 1]?.focus();
  }
}

onMounted(() => {
  focusInitialElement();
});

watch(
  () => [props.showLoadingState, props.showErrorState, pipelines.value.length],
  () => {
    focusInitialElement();
  }
);
</script>

<template>
  <aside
    id="pipeline-flyout"
    ref="flyoutRef"
    class="pipeline-flyout"
    aria-label="Pipelines"
    @keydown="handleKeydown"
  >
    <header class="pipeline-flyout-header">
      <h3>Pipelines</h3>
      <button
        type="button"
        class="pipeline-flyout-refresh"
        aria-label="Refresh pipelines"
        title="Refresh pipelines"
        @click="handleRefresh"
      >
        <RefreshCw :size="18" />
      </button>
    </header>

    <div class="pipeline-flyout-body">
      <div v-if="showLoadingState" class="pipeline-state pipeline-state--loading" aria-live="polite">
        <div class="skeleton-block skeleton-block--title" />
        <div class="skeleton-block skeleton-block--wide" />
        <div class="skeleton-block" />
      </div>

      <div v-else-if="showErrorState" class="pipeline-state pipeline-state--error" role="alert">
        <p class="pipeline-state-title">Failed to load</p>
        <p class="pipeline-state-message">{{ connectionError }}</p>
        <Button type="button" @click="emit('retry')">Retry</Button>
      </div>

      <div v-else-if="pipelines.length === 0" class="pipeline-state pipeline-state--empty">
        <p class="pipeline-state-title">No pipelines yet</p>
        <Button type="button" @click="handleNewPipeline">+ New Pipeline</Button>
      </div>

      <PipelineSidebar v-else hide-header @pipeline-selected="emit('close')" />
    </div>
  </aside>
</template>

<style scoped>
.pipeline-flyout {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.pipeline-flyout-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 1rem;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 80%, transparent);
}

.pipeline-flyout-header h3 {
  margin: 0;
  font-size: var(--text-xl);
  color: var(--muted-foreground);
}

.pipeline-flyout-refresh {
  width: 36px;
  height: 36px;
  border: 0;
  border-radius: 10px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--foreground);
  cursor: pointer;
  transition: background 0.15s ease;
}

.pipeline-flyout-refresh:hover,
.pipeline-flyout-refresh:focus-visible {
  background: var(--muted);
  outline: none;
}

.pipeline-flyout-body {
  min-height: 0;
  flex: 1;
  display: flex;
  overflow: hidden;
}

.pipeline-state {
  flex: 1;
  display: grid;
  place-content: center;
  gap: 0.75rem;
  padding: 1.5rem;
  text-align: center;
}

.pipeline-state--error {
  color: var(--foreground);
}

.pipeline-state-title {
  margin: 0;
  font-size: var(--text-lg);
}

.pipeline-state-message {
  margin: 0;
  color: var(--muted-foreground);
  font-size: var(--text-sm);
}

.pipeline-state--loading {
  width: min(40rem, 100%);
  margin: auto;
}

.skeleton-block {
  height: 0.875rem;
  border-radius: 0.5rem;
  background: var(--muted);
  animation: skeleton-pulse 1.4s ease-in-out infinite;
}

.skeleton-block--title {
  width: min(14rem, 85vw);
  height: 1.1rem;
  justify-self: center;
}

.skeleton-block--wide {
  width: min(28rem, 90vw);
  justify-self: center;
}

.skeleton-block:not(.skeleton-block--title):not(.skeleton-block--wide) {
  width: min(20rem, 88vw);
  justify-self: center;
}

@keyframes skeleton-pulse {
  0%,
  100% {
    background: var(--muted);
  }
  50% {
    background: var(--card);
  }
}
</style>
