<script setup lang="ts">
import type { PropType } from 'vue';
import SchemaForm from './SchemaForm.vue';

defineProps({
  toolName: { type: String, required: true },
  toolArgs: { type: Object as PropType<Record<string, unknown>>, required: true },
  canSubmit: { type: Boolean, default: false },
  selectedToolDisplayName: { type: String, default: '' },
  selectedToolDescription: { type: String as PropType<string | null>, default: null },
  selectedToolServer: { type: String as PropType<string | null>, default: null },
  requiredArgNames: { type: Array as PropType<string[]>, required: true },
  selectedToolSchema: { type: Object as PropType<Record<string, unknown> | null | undefined>, default: undefined },
  isToolPinned: { type: Function as PropType<(name: string) => boolean>, required: true },
  togglePin: { type: Function as PropType<(name: string) => void>, required: true },
  clearSelection: { type: Function as PropType<() => void>, required: true },
  onToolArgsChange: { type: Function as PropType<(data: Record<string, unknown>) => void>, required: true },
  onToolArgsValidation: { type: Function as PropType<(state: { valid: boolean }) => void>, required: true },
  submitSelectedTool: { type: Function as PropType<() => void>, required: true },
});
</script>

<template>
  <section v-if="toolName" class="tool-palette__detail">
    <div class="tool-palette__detail-head">
      <div>
        <strong>{{ selectedToolDisplayName }}</strong>
        <p>{{ selectedToolDescription || 'No description.' }}</p>
      </div>
      <div class="tool-palette__detail-actions">
        <span v-if="selectedToolServer" class="tool-palette__server">{{ selectedToolServer }}</span>
        <button type="button" class="btn btn-ghost btn-xs" @click="togglePin(toolName)">
          {{ isToolPinned(toolName) ? 'Unpin' : 'Pin' }}
        </button>
        <button type="button" class="btn btn-ghost btn-xs" @click="clearSelection">Clear</button>
      </div>
    </div>

    <div class="tool-palette__required">
      <span>Required args</span>
      <div class="tool-palette__required-tags">
        <span v-for="requiredName in requiredArgNames" :key="requiredName" class="tool-palette__required-tag">{{ requiredName }}</span>
        <span v-if="requiredArgNames.length === 0" class="tool-palette__required-empty">No required args</span>
      </div>
    </div>

    <SchemaForm
      :schema="selectedToolSchema"
      :data="toolArgs"
      :ariaLabel="`${selectedToolDisplayName || toolName.split('.').pop() || 'Workflow tool'} arguments`"
      @change="onToolArgsChange"
      @validation="onToolArgsValidation"
    />

    <div class="tool-palette__detail-footer">
      <button type="button" class="btn btn-primary" :disabled="!canSubmit" @click="submitSelectedTool">Add Tool</button>
    </div>
  </section>
</template>
