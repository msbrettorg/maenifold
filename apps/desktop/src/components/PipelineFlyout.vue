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
      class="w-[min(320px,88vw)] border-0 bg-[var(--surface)] p-0 text-[var(--color-text-primary)] shadow-[var(--shadow-16)]"
    >
      <div ref="flyoutRef" class="flex h-full flex-col" @keydown="handleKeydown">
        <SheetHeader class="gap-1 px-4 py-4 text-left">
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0">
              <SheetTitle class="font-display text-[var(--text-title)] font-medium text-[var(--color-text-primary)]">Pipelines</SheetTitle>
              <SheetDescription class="text-[var(--text-body-sm)] text-[var(--text-secondary)]">Browse, load, and create pipeline sessions.</SheetDescription>
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

        <div class="px-4 pb-3">
          <Input
            v-model="searchQuery"
            type="text"
            class="border-0 bg-[var(--surface)] text-[var(--color-text-primary)]"
            placeholder="Search pipelines…"
          />
        </div>

        <ScrollArea class="h-[calc(100vh-7rem)]">
          <div
            v-if="showLoadingState"
            class="grid min-h-[calc(100vh-12rem)] place-content-center gap-3 p-6 text-center"
            aria-live="polite"
          >
            <div class="skeleton-pulse h-[1.1rem] w-[min(14rem,85vw)] justify-self-center rounded-[var(--radius-4)] bg-[var(--surface-high)]" />
            <div class="skeleton-pulse h-3.5 w-[min(28rem,90vw)] justify-self-center rounded-[var(--radius-4)] bg-[var(--surface-high)]" />
            <div class="skeleton-pulse h-3.5 w-[min(20rem,88vw)] justify-self-center rounded-[var(--radius-4)] bg-[var(--surface-high)]" />
          </div>

          <div
            v-else-if="showErrorState"
            class="grid min-h-[calc(100vh-12rem)] place-content-center gap-3 p-6 text-center text-[var(--color-text-primary)]"
            role="alert"
          >
            <p class="m-0 text-[length:var(--text-lg)] text-[var(--color-text-primary)]">Failed to load</p>
            <p class="m-0 text-[length:var(--text-sm)] text-[var(--text-secondary)]">{{ connectionError }}</p>
            <Button type="button" class="rounded-[4px]" @click="emit('retry')">Retry</Button>
          </div>

          <div
            v-else-if="pipelines.length === 0"
            class="grid min-h-[calc(100vh-12rem)] place-content-center gap-3 p-6 text-center"
          >
            <p class="m-0 text-[length:var(--text-lg)] text-[var(--color-text-primary)]">No pipelines yet</p>
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

<!-- audit-ok: keyframe-animation — @keyframes skeleton-pulse for skeleton loading states; not expressible as Tailwind utilities. -->
<style scoped>
.skeleton-pulse {
  animation: skeleton-pulse 1.4s ease-in-out infinite;
}

@keyframes skeleton-pulse {
  0%, 100% { background: var(--surface-high); }
  50% { background: var(--surface-container); }
}
</style>
