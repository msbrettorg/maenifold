<script setup lang="ts">
import { computed } from 'vue';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
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
  if (props.selectedEdge) return store.designMode ? 'Edge Configuration' : 'Edge Details';
  return store.designMode ? 'Node Configuration' : 'Node Inspector';
});

const modalDescription = computed(() =>
  props.selectedEdge
    ? 'Inspect or configure the selected pipeline edge.'
    : 'Inspect or configure the selected pipeline node.',
);
</script>

<template>
  <Dialog :open="true" @update:open="(open) => !open && emit('close')">
    <DialogContent class="config-panel gap-0 p-0 sm:max-w-[560px]" :show-close-button="false">
      <DialogHeader class="config-header">
        <div class="config-heading">
          <DialogTitle class="config-title">{{ modalTitle }}</DialogTitle>
          <DialogDescription>{{ modalDescription }}</DialogDescription>
        </div>
        <DialogClose as-child>
          <button
            class="config-close"
            type="button"
            aria-label="Close configuration"
            title="Close (Esc)"
          >
            ×
          </button>
        </DialogClose>
      </DialogHeader>
      <div class="config-body">
        <DesignNodeConfigPanel
          :selected-node-id="selectedNodeId"
          :selected-edge="selectedEdge"
        />
      </div>
    </DialogContent>
  </Dialog>
</template>

<style scoped>
.config-panel {
  width: min(560px, 90vw);
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  background: var(--card, #201f1f);
  border-radius: var(--radius-xl);
  overflow: hidden;
  border: 1px solid var(--border);
  box-shadow: 0 20px 48px rgba(0, 0, 0, 0.35);
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
  background: var(--accent, #353534);
}

.config-title {
  font-family: var(--font-display);
  font-size: var(--text-sm);
  font-weight: var(--weight-title);
  color: var(--foreground);
}

.config-close {
  background: none;
  border: none;
  color: var(--muted-foreground);
  font-size: var(--text-xl);
  cursor: pointer;
  padding: var(--space-1);
  line-height: 1;
  border-radius: var(--radius-sm);
}

.config-close:hover {
  color: var(--foreground);
  background: var(--muted);
}

.config-close:focus-visible {
  outline: 2px solid var(--ring);
  outline-offset: 2px;
}

.config-body {
  flex: 1;
  overflow-y: auto;
}

.config-body :deep(.inspector) {
  border-left: none;
  height: auto;
}
</style>
