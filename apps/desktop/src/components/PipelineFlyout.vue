<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
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
  <Sheet :open="true" @update:open="(open) => !open && emit('close')">
    <SheetContent
      id="pipeline-flyout"
      side="left"
      :show-close-button="false"
      class="pipeline-flyout border-0 p-0 text-[var(--foreground)]"
    >
      <div ref="flyoutRef" class="flyout-shell" @keydown="handleKeydown">
        <SheetHeader class="gap-1 px-4 py-4 text-left">
          <div class="flyout-header-row">
            <div class="min-w-0">
              <SheetTitle class="font-display text-[var(--text-title)] font-medium text-[var(--foreground)]">Pipelines</SheetTitle>
              <SheetDescription class="text-[var(--text-body-sm)] text-[var(--text-dim)]">Browse, load, and create pipeline sessions.</SheetDescription>
            </div>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              class="rounded-[4px]"
              aria-label="Refresh pipelines"
              @click="handleRefresh"
            >
              <RefreshCw :size="18" />
            </Button>
          </div>
        </SheetHeader>

        <div class="flyout-search">
          <Input
            v-model="searchQuery"
            type="text"
            class="border-0 bg-[var(--surface)] text-[var(--foreground)]"
            placeholder="Search pipelines…"
          />
        </div>

        <ScrollArea class="flyout-body">
          <div v-if="showLoadingState" class="flyout-state" aria-live="polite">
            <div class="skeleton-block skeleton-block--title" />
            <div class="skeleton-block skeleton-block--wide" />
            <div class="skeleton-block" />
          </div>

          <div v-else-if="showErrorState" class="flyout-state flyout-state--error" role="alert">
            <p class="flyout-state-title">Failed to load</p>
            <p class="flyout-state-msg">{{ connectionError }}</p>
            <Button type="button" class="rounded-[4px]" @click="emit('retry')">Retry</Button>
          </div>

          <div v-else-if="pipelines.length === 0" class="flyout-state">
            <p class="flyout-state-title">No pipelines yet</p>
            <Button type="button" class="rounded-[4px]" @click="emit('new-pipeline')">New pipeline</Button>
          </div>

          <PipelineSidebar
            v-else
            hide-header
            :filter-query="searchQuery"
            @pipeline-selected="emit('close')"
          />
        </ScrollArea>
      </div>
    </SheetContent>
  </Sheet>
</template>

<style scoped>
.pipeline-flyout {
  width: min(320px, 88vw);
  background: var(--surface);
  box-shadow: 0 0 32px rgba(173, 198, 255, 0.08);
}

.flyout-shell {
  display: flex;
  height: 100%;
  flex-direction: column;
}

.flyout-header-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
}

.flyout-search { padding: 0 1rem 0.75rem; }

.flyout-body {
  height: calc(100vh - 7rem);
}

.flyout-state {
  min-height: calc(100vh - 12rem);
  display: grid;
  place-content: center;
  gap: 0.75rem;
  padding: 1.5rem;
  text-align: center;
}

.flyout-state--error { color: var(--foreground); }

.flyout-state-title {
  margin: 0;
  font-size: var(--text-lg);
  color: var(--foreground);
}

.flyout-state-msg {
  margin: 0;
  color: var(--text-dim);
  font-size: var(--text-sm);
}

.skeleton-block {
  height: 0.875rem;
  border-radius: 4px;
  background: var(--surface-high);
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
  0%, 100% { background: var(--surface-high); }
  50% { background: var(--surface-container); }
}
</style>
