<script setup lang="ts">
import { Copy } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import WorkflowRunButton from './WorkflowRunButton.vue';
import type { WorkflowAssetDefinition } from '../lib/workflow-asset-types';

defineProps<{
  title: string;
  workflow: WorkflowAssetDefinition;
  hasSteps: boolean;
  allExpanded: boolean;
  exportState: 'idle' | 'exporting' | 'success' | 'error';
  isDirty: boolean;
  saveState: string;
  saveButtonLabel: string;
  duplicateDisabled?: boolean;
}>();

const emit = defineEmits<{
  'toggle-all': [];
  'export-click': [];
  save: [];
  duplicate: [];
}>();
</script>

<template>
  <header class="flex items-start justify-between gap-4 pb-3 bg-bg-elevated/30 rounded-sm">
    <h1 class="m-0 text-lg font-semibold text-text-primary flex items-center gap-2 min-w-0 leading-tight">
      {{ title }}
    </h1>
    <div class="flex items-center gap-2 shrink-0">
      <Button
        variant="outline"
        class="text-text-secondary hover:text-node-workflow hover:border-node-workflow/40"
        :disabled="!hasSteps"
        @click="emit('toggle-all')"
      >
        {{ allExpanded ? 'Collapse all' : 'Expand all' }}
      </Button>
      <Button variant="outline" :disabled="exportState === 'exporting'" @click="emit('export-click')">
        {{ exportState === 'exporting' ? 'Exporting…' : 'Export' }}
      </Button>
      <Button
        variant="outline"
        class="text-text-secondary hover:text-node-workflow hover:border-node-workflow/40"
        :disabled="duplicateDisabled"
        title="Duplicate workflow"
        @click="emit('duplicate')"
      >
        <Copy class="w-4 h-4" />
      </Button>
      <Button
        variant="outline"
        :class="[
          isDirty
            ? 'text-status-active border-status-active bg-status-active/10 hover:bg-status-active/20'
            : saveState === 'error'
              ? 'text-status-abandoned border-status-abandoned bg-status-abandoned/10'
              : 'text-text-secondary opacity-40 cursor-default',
        ]"
        :disabled="saveState === 'saving' || (!isDirty && saveState !== 'error')"
        @click="emit('save')"
      >
        {{ saveButtonLabel }}
      </Button>
      <WorkflowRunButton :workflow="workflow" />
    </div>
  </header>
</template>
