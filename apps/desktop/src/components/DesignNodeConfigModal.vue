<script setup lang="ts">
import { computed } from 'vue';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import DesignNodeConfigPanel from './DesignNodeConfigPanel.vue';
import { usePipelineStore } from '../stores/pipeline';

const props = defineProps<{
  selectedNodeId: string | null;
  selectedEdge: { source: string; target: string } | null;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const store = usePipelineStore();

const modalTitle = computed(() => {
  if (props.selectedEdge) return store.designMode ? 'Edge configuration' : 'Edge details';
  return store.designMode ? 'Node configuration' : 'Node inspector';
});

const modalDescription = computed(() =>
  props.selectedEdge
    ? 'Inspect or configure the selected pipeline edge.'
    : 'Inspect or configure the selected pipeline node.',
);
</script>

<template>
  <Sheet :open="true" @update:open="(open) => !open && emit('close')">
    <SheetContent
      side="right"
      class="gap-0 p-0 text-[var(--color-text-primary)] sm:max-w-[360px] w-[min(360px,92vw)] max-h-screen flex flex-col bg-[var(--surface)] rounded-none overflow-hidden border-0 shadow-[var(--shadow-16)] config-panel"
      :show-close-button="false"
    >
      <SheetHeader class="flex flex-row items-center justify-between py-[var(--space-3)] px-[var(--space-4)] bg-[var(--surface)]">
        <div class="grid gap-1 text-left">
          <SheetTitle class="font-[var(--font-display)] text-[length:var(--text-title)] font-medium text-[var(--color-text-primary)]">{{ modalTitle }}</SheetTitle>
          <SheetDescription class="text-[var(--text-body-sm)] text-[var(--text-secondary)]">{{ modalDescription }}</SheetDescription>
        </div>
        <Button
          class="rounded-[4px]"
          variant="ghost"
          size="sm"
          type="button"
          aria-label="Close configuration"
          @click="emit('close')"
        >
          Close
        </Button>
      </SheetHeader>
      <ScrollArea class="config-body h-[calc(100vh-5rem)]">
        <DesignNodeConfigPanel
          :selected-node-id="selectedNodeId"
          :selected-edge="selectedEdge"
        />
      </ScrollArea>
    </SheetContent>
  </Sheet>
</template>

<!-- audit-ok: deep-third-party — :deep(.inspector) targets DesignNodeConfigPanel internal layout -->
<style scoped>
.config-body :deep(.inspector) {
  border-left: none;
  height: auto;
  min-height: 100%;
}
</style>
