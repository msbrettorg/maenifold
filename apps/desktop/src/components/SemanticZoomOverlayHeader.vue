<script setup lang="ts">
import { computed } from 'vue';
import { X } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { DialogClose } from '@/components/ui/dialog';

const props = defineProps<{
  displayLabel: string;
  nodeType: string;
  statusLabel: string;
}>();

const NODE_TYPE_CLASSES: Record<string, string> = {
  prompt: 'bg-node-prompt/15 text-node-prompt border-node-prompt/20',
  sequentialThinking: 'bg-node-thinking/15 text-node-thinking border-node-thinking/20',
  tool: 'bg-node-tool/15 text-node-tool border-node-tool/20',
  workflow: 'bg-node-workflow/15 text-node-workflow border-node-workflow/20',
};

const STATUS_CLASSES: Record<string, string> = {
  active: 'bg-status-active/15 text-status-active border-status-active/30',
  completed: 'bg-status-completed/15 text-status-completed border-status-completed/30',
  abandoned: 'bg-destructive/10 text-destructive border-destructive/20',
  ready: 'bg-status-ready/15 text-status-ready border-status-ready/30',
};

const nodeTypeClass = computed(
  () => NODE_TYPE_CLASSES[props.nodeType] ?? NODE_TYPE_CLASSES.workflow,
);

const statusClass = computed(
  () => STATUS_CLASSES[props.statusLabel] ?? 'bg-muted text-muted-foreground border-border',
);
</script>

<template>
  <header class="flex items-center justify-between py-3 px-4 bg-muted/40 border-b border-border">
    <div class="flex items-center gap-2">
      <span class="font-mono text-xs font-semibold text-foreground">{{ displayLabel }}</span>
      <Badge variant="outline" :class="nodeTypeClass">{{ nodeType }}</Badge>
      <Badge variant="outline" :class="statusClass">{{ statusLabel }}</Badge>
    </div>
    <DialogClose as-child>
      <Button
        variant="ghost"
        size="icon"
        class="size-7 text-muted-foreground hover:text-foreground"
        aria-label="Close inspection"
        title="Close (Esc)"
      >
        <X class="size-4" />
      </Button>
    </DialogClose>
  </header>
</template>
