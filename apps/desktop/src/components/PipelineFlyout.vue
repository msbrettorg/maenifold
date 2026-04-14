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
const searchQuery = ref('');

function getFocusableElements() {
  return Array.from(
    flyoutRef.value?.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [href], input, [tabindex]:not([tabindex="-1"])'
    ) ?? []
  ).filter((el) => !el.hasAttribute('disabled') && el.getClientRects().length > 0);
}

function focusInitialElement() {
  nextTick(() => {
    const first = flyoutRef.value?.querySelector<HTMLElement>('[data-pipeline-item]');
    (first ?? getFocusableElements()[0])?.focus();
  });
}

function focusSiblingPipelineItem(current: HTMLElement, direction: -1 | 1) {
  const items = Array.from(
    flyoutRef.value?.querySelectorAll<HTMLElement>('[data-pipeline-item]') ?? []
  );
  const idx = items.indexOf(current);
  if (idx === -1 || items.length === 0) return;
  items[(idx + direction + items.length) % items.length]?.focus();
}

async function handleRefresh() {
  await store.refreshList();
}

function handleKeydown(event: KeyboardEvent) {
  const target = event.target instanceof HTMLElement ? event.target : null;
  if (!target) return;

  if (event.key === 'Tab') {
    const els = getFocusableElements();
    if (els.length === 0) return;
    if (event.shiftKey && target === els[0]) {
      event.preventDefault();
      els[els.length - 1].focus();
    } else if (!event.shiftKey && target === els[els.length - 1]) {
      event.preventDefault();
      els[0].focus();
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

onMounted(focusInitialElement);

watch(
  () => [props.showLoadingState, props.showErrorState, pipelines.value.length],
  () => focusInitialElement(),
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
    <header class="flyout-header">
      <h3>Pipelines</h3>
      <button
        type="button"
        class="flyout-refresh"
        aria-label="Refresh pipelines"
        title="Refresh pipelines"
        @click="handleRefresh"
      >
        <RefreshCw :size="18" />
      </button>
    </header>

    <div class="flyout-search">
      <input
        v-model="searchQuery"
        type="text"
        class="flyout-search-input"
        placeholder="Search pipelines…"
      />
    </div>

    <div class="flyout-body">
      <div v-if="showLoadingState" class="flyout-state" aria-live="polite">
        <div class="skeleton-block skeleton-block--title" />
        <div class="skeleton-block skeleton-block--wide" />
        <div class="skeleton-block" />
      </div>

      <div v-else-if="showErrorState" class="flyout-state flyout-state--error" role="alert">
        <p class="flyout-state-title">Failed to load</p>
        <p class="flyout-state-msg">{{ connectionError }}</p>
        <Button type="button" @click="emit('retry')">Retry</Button>
      </div>

      <div v-else-if="pipelines.length === 0" class="flyout-state">
        <p class="flyout-state-title">No pipelines yet</p>
        <Button type="button" @click="emit('new-pipeline')">+ New pipeline</Button>
      </div>

      <PipelineSidebar
        v-else
        hide-header
        :filter-query="searchQuery"
        @pipeline-selected="emit('close')"
      />
    </div>
  </aside>
</template>

<style scoped>
.pipeline-flyout {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #1c1b1b;
  animation: slide-in 0.2s ease-out;
}

@keyframes slide-in {
  from { transform: translateX(-100%); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}

.flyout-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 1rem;
}

.flyout-header h3 {
  margin: 0;
  font-family: var(--font-display);
  font-weight: 500;
  font-size: var(--text-title);
  color: #e5e2e1;
}

.flyout-refresh {
  width: 36px;
  height: 36px;
  border: 0;
  border-radius: 4px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: #e5e2e1;
  cursor: pointer;
  transition: background 0.15s;
}

.flyout-refresh:hover,
.flyout-refresh:focus-visible { background: #2a2a2a; outline: none; }

.flyout-search { padding: 0 1rem 0.75rem; }

.flyout-search-input {
  width: 100%;
  padding: 0.4rem 0.6rem;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 4px;
  background: #1c1b1b;
  color: #e5e2e1;
  font-size: var(--text-sm);
  outline: none;
}

.flyout-search-input::placeholder { color: #707070; }

.flyout-search-input:focus {
  background: #2a2a2a;
  border-color: rgba(77, 142, 255, 0.4);
}

.flyout-body {
  min-height: 0;
  flex: 1;
  display: flex;
  overflow: hidden;
}

.flyout-state {
  flex: 1;
  display: grid;
  place-content: center;
  gap: 0.75rem;
  padding: 1.5rem;
  text-align: center;
}

.flyout-state--error { color: #e5e2e1; }

.flyout-state-title {
  margin: 0;
  font-size: var(--text-lg);
  color: #e5e2e1;
}

.flyout-state-msg {
  margin: 0;
  color: #707070;
  font-size: var(--text-sm);
}

.skeleton-block {
  height: 0.875rem;
  border-radius: 4px;
  background: #2a2a2a;
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
  0%, 100% { background: #2a2a2a; }
  50% { background: #201f1f; }
}
</style>
