<script setup lang="ts">
import { computed } from 'vue';
import { X } from 'lucide-vue-next';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
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
  <Dialog :open="true" @update:open="(open) => !open && emit('close')">
    <DialogContent
      class="flex max-h-[85vh] w-[80vw] max-w-[80vw] flex-col gap-0 overflow-hidden p-0"
      :show-close-button="false"
    >
      <DialogHeader class="flex flex-row items-center justify-between gap-4 border-b border-border-default px-4 py-3">
        <div class="grid gap-1 text-left">
          <DialogTitle class="text-base font-medium text-text-primary">
            {{ modalTitle }}
          </DialogTitle>
          <DialogDescription class="text-xs text-text-secondary">
            {{ modalDescription }}
          </DialogDescription>
        </div>
        <Button
          variant="ghost"
          size="icon"
          type="button"
          aria-label="Close configuration"
          class="h-8 w-8 shrink-0"
          @click="emit('close')"
        >
          <X class="h-4 w-4" />
        </Button>
      </DialogHeader>
      <ScrollArea class="flex-1 overflow-hidden">
        <div class="px-4 py-3">
          <DesignNodeConfigPanel
            :selected-node-id="selectedNodeId"
            :selected-edge="selectedEdge"
          />
        </div>
      </ScrollArea>
    </DialogContent>
  </Dialog>
</template>
