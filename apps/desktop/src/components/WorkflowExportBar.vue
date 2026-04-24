<script setup lang="ts">
import { Button } from '@/components/ui/button';

defineProps<{
  exportPromptVisible: boolean;
  exportState: 'idle' | 'exporting' | 'success' | 'error';
  exportError?: string | null;
  lastExportVersion: 'draft' | 'saved';
}>();

const emit = defineEmits<{
  export: [version: 'draft' | 'saved'];
  cancel: [];
}>();
</script>

<template>
  <div
    v-if="exportPromptVisible || exportState !== 'idle'"
    class="flex items-center flex-wrap gap-2 text-text-secondary text-xs leading-snug"
    role="status"
    aria-live="polite"
  >
    <template v-if="exportPromptVisible">
      <span>Export draft or saved version?</span>
      <Button variant="outline" size="xs" type="button" @click="emit('export', 'draft')">Draft</Button>
      <Button variant="outline" size="xs" type="button" @click="emit('export', 'saved')">Saved</Button>
      <Button variant="ghost" size="xs" type="button" @click="emit('cancel')">Cancel</Button>
    </template>
    <template v-else-if="exportState === 'error'">
      <span>{{ exportError ?? 'Export failed.' }}</span>
      <Button variant="outline" size="xs" type="button" @click="emit('export', lastExportVersion)">Retry</Button>
    </template>
    <template v-else-if="exportState === 'success'">
      <span>Exported ✓</span>
    </template>
  </div>
</template>
