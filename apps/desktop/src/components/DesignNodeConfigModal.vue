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
      class="config-panel gap-0 p-0 text-[#e5e2e1] sm:max-w-[360px]"
      :show-close-button="false"
    >
      <SheetHeader class="config-header">
        <div class="config-heading">
          <SheetTitle class="config-title">{{ modalTitle }}</SheetTitle>
          <SheetDescription class="text-[var(--text-body-sm)] text-[#707070]">{{ modalDescription }}</SheetDescription>
        </div>
        <Button
          class="config-close rounded-[4px]"
          variant="ghost"
          size="sm"
          type="button"
          aria-label="Close configuration"
          @click="emit('close')"
        >
          Close
        </Button>
      </SheetHeader>
      <ScrollArea class="config-body">
        <DesignNodeConfigPanel
          :selected-node-id="selectedNodeId"
          :selected-edge="selectedEdge"
        />
      </ScrollArea>
    </SheetContent>
  </Sheet>
</template>

<style scoped>
.config-panel {
  width: min(360px, 92vw);
  max-height: 100vh;
  display: flex;
  flex-direction: column;
  background: #1c1b1b;
  border-radius: 0;
  overflow: hidden;
  border: 0;
  box-shadow: 0 0 32px rgba(173, 198, 255, 0.08);
}

.config-heading {
  display: grid;
  gap: 0.25rem;
  text-align: left;
}

.config-header {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  background: #1c1b1b;
}

.config-title {
  font-family: var(--font-display);
  font-size: var(--text-title);
  font-weight: 500;
  color: #e5e2e1;
}

.config-body {
  height: calc(100vh - 5rem);
}

.config-body :deep(.inspector) {
  border-left: none;
  height: auto;
  min-height: 100%;
}
</style>
