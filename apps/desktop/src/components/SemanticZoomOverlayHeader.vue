<script setup lang="ts">
import { computed } from 'vue';
import { X } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { DialogClose } from '@/components/ui/dialog';

const props = defineProps<{
  displayLabel: string;
  nodeType: string;
  statusLabel: string;
}>();

const NODE_TYPE_BG: Record<string, string> = {
  prompt: 'var(--color-node-prompt-15)',
  sequentialThinking: 'var(--color-node-thinking-15)',
  tool: 'var(--color-node-tool-15)',
  workflow: 'var(--color-node-workflow-15)',
};

const STATUS_BG: Record<string, string> = {
  active: 'var(--color-status-active-15)',
  completed: 'var(--color-status-completed-15)',
  abandoned: 'var(--color-status-abandoned-15)',
  ready: 'var(--color-status-ready-15)',
};

const STATUS_FG: Record<string, string> = {
  active: 'var(--color-status-active)',
  completed: 'var(--color-status-completed)',
  abandoned: 'var(--color-status-abandoned)',
  ready: 'var(--color-status-ready)',
};

const pillStyle = computed(() => ({
  background: NODE_TYPE_BG[props.nodeType] ?? 'var(--color-node-workflow-15)',
}));

const statusStyle = computed(() => ({
  background: STATUS_BG[props.statusLabel] ?? 'var(--color-status-pending-15)',
  color: STATUS_FG[props.statusLabel] ?? 'var(--color-text-secondary)',
}));
</script>

<template>
  <header class="flex items-center justify-between py-[var(--space-3)] px-[var(--space-4)] bg-[var(--accent,var(--surface-highest))]">
    <div class="flex items-center gap-[var(--space-2)]">
      <span class="font-mono text-[length:var(--text-caption)] font-[number:var(--weight-title)] text-text-primary">{{ displayLabel }}</span>
      <!-- audit-ok: dynamic-binding (background tied to nodeType variant via color-mix tokens) -->
      <span
        class="rounded-[4px] text-[length:var(--text-tiny)] py-[2px] px-[8px] text-text-primary"
        :style="pillStyle"
      >{{ nodeType }}</span>
      <!-- audit-ok: dynamic-binding (background and color tied to statusLabel variant via color-mix tokens) -->
      <span
        class="rounded-[4px] text-[length:var(--text-tiny)] py-[2px] px-[6px]"
        :style="statusStyle"
      >{{ statusLabel }}</span>
    </div>
    <DialogClose as-child>
      <Button
        class="text-text-secondary p-[var(--space-1)] leading-none rounded-[var(--radius-sm)] hover:text-text-primary hover:bg-[var(--color-bg-surface)] focus-visible:outline focus-visible:outline-2 focus-visible:outline-[var(--color-brand)] focus-visible:outline-offset-2"
        variant="ghost"
        size="icon"
        aria-label="Close inspection"
        title="Close (Esc)"
      >
        <X class="size-4" />
      </Button>
    </DialogClose>
  </header>
</template>
