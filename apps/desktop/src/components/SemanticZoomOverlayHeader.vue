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
  prompt: 'bg-blue-500/10 text-blue-700 dark:text-blue-300 border-blue-500/20',
  sequentialThinking: 'bg-purple-500/10 text-purple-700 dark:text-purple-300 border-purple-500/20',
  tool: 'bg-amber-500/10 text-amber-700 dark:text-amber-300 border-amber-500/20',
  workflow: 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-300 border-emerald-500/20',
};

const STATUS_CLASSES: Record<string, string> = {
  active: 'bg-blue-500/10 text-blue-700 dark:text-blue-300 border-blue-500/20',
  completed: 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-300 border-emerald-500/20',
  abandoned: 'bg-destructive/10 text-destructive border-destructive/20',
  ready: 'bg-amber-500/10 text-amber-700 dark:text-amber-300 border-amber-500/20',
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
