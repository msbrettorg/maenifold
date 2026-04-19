<script setup lang="ts">
import type { WorkflowCategory } from '../lib/workflow-asset-types'

interface CategoryChip {
  label: string
  value: WorkflowCategory | 'all'
}

const props = defineProps<{
  chips: CategoryChip[]
  searchQuery: string
  categoryFilter: WorkflowCategory | 'all'
  browserError: string | null
  importError: string | null
  importConflict: { id: string, json: string, name: string } | null
}>()

const emit = defineEmits<{
  (e: 'update:searchQuery', value: string): void
  (e: 'update:categoryFilter', value: WorkflowCategory | 'all'): void
  (e: 'retry'): void
  (e: 'resolve-conflict', resolution: 'overwrite' | 'copy'): void
  (e: 'clear-import-state'): void
  (e: 'dismiss-import-error'): void
}>()

function updateSearchQuery(event: Event) {
  emit('update:searchQuery', (event.target as HTMLInputElement).value)
}
</script>

<template>
  <div role="search" aria-label="Workflow filters">
    <div class="search-wrapper">
      <svg class="search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="11" cy="11" r="8" />
        <line x1="21" y1="21" x2="16.65" y2="16.65" />
      </svg>
      <input
        :value="searchQuery"
        type="text"
        class="search-input"
        placeholder="Find workflow..."
        @input="updateSearchQuery"
      />
    </div>

    <div class="chip-row">
      <button
        v-for="chip in chips"
        :key="chip.value"
        :class="['chip', { 'chip--active': categoryFilter === chip.value }]"
        type="button"
        @click="emit('update:categoryFilter', chip.value)"
      >
        {{ chip.label }}
      </button>
    </div>

    <div v-if="browserError" class="import-feedback import-feedback--error" role="alert">
      <span class="feedback-text">{{ browserError }}</span>
      <button class="feedback-btn" type="button" @click="emit('retry')">Retry</button>
    </div>

    <div
      v-if="props.importConflict || props.importError"
      class="import-feedback"
      :class="{ 'import-feedback--error': props.importError }"
      :role="props.importError ? 'alert' : 'status'"
      aria-live="polite"
    >
      <template v-if="props.importConflict">
        <span class="feedback-text">
          Workflow ID <code>{{ props.importConflict.id }}</code> already exists ({{ props.importConflict.name }}).
        </span>
        <button class="feedback-btn" type="button" @click="emit('resolve-conflict', 'overwrite')">Overwrite</button>
        <button class="feedback-btn" type="button" @click="emit('resolve-conflict', 'copy')">Import as Copy</button>
        <button class="feedback-btn" type="button" @click="emit('clear-import-state')">Cancel</button>
      </template>
      <template v-else-if="props.importError">
        <span class="feedback-text">{{ props.importError }}</span>
        <button class="feedback-btn" type="button" @click="emit('dismiss-import-error')">Dismiss</button>
      </template>
    </div>
  </div>
</template>

<style scoped>
.search-wrapper { position: relative; padding: var(--space-4) var(--space-4) var(--space-3); }
.search-icon { position: absolute; left: calc(var(--space-4) + var(--space-3)); top: calc(50% + var(--space-1) / 2); transform: translateY(-50%); color: var(--color-text-secondary); pointer-events: none; }
.search-input { width: 100%; padding: var(--space-2) var(--space-3) var(--space-2) calc(var(--space-3) + 20px); background: var(--surface); border: 1px solid transparent; border-radius: var(--radius-sm); color: var(--color-text-primary); font: var(--weight-body) var(--text-caption)/1.4 var(--font-body); outline: none; transition: border-color var(--transition-fast), box-shadow var(--transition-fast), background var(--transition-fast); }
.search-input::placeholder { color: var(--color-text-secondary); }
.search-input:hover:not(:focus) { background: var(--surface-high); }
.search-input:focus { background: var(--surface-high); border-color: rgba(77, 142, 255, 0.4); box-shadow: none; }
.chip-row { display: flex; gap: var(--space-2); padding: 0 var(--space-4) var(--space-4); overflow-x: auto; scrollbar-width: none; flex-shrink: 0; }
.chip-row::-webkit-scrollbar { display: none; }
.chip { flex-shrink: 0; padding: var(--space-1) var(--space-2); border: 1px solid transparent; border-radius: var(--radius-sm); background: transparent; color: var(--color-text-secondary); font: var(--weight-body) var(--text-caption)/1.4 var(--font-body); cursor: pointer; white-space: nowrap; transition: background var(--transition-fast), color var(--transition-fast); }
.chip:hover { background: var(--color-text-tertiary); color: var(--color-text-primary); }
.chip--active { background: color-mix(in srgb, var(--color-node-workflow) 12%, var(--color-text-tertiary)); color: var(--color-node-workflow); }
.import-feedback { display: flex; flex-wrap: wrap; gap: var(--space-2); align-items: center; margin: 0 var(--space-4) var(--space-3); padding: var(--space-3); border: 1px solid var(--color-border-subtle); border-radius: var(--radius-md); background: var(--color-bg-elevated); }
.import-feedback--error { border-color: color-mix(in srgb, var(--color-status-abandoned) 45%, transparent); color: var(--color-status-abandoned); }
.feedback-text { flex: 1 1 100%; font: var(--weight-display) var(--text-tiny)/1.4 var(--font-body); color: inherit; word-break: break-word; }
.feedback-btn { min-height: 24px; padding: 0 var(--space-2); border: 1px solid var(--color-border-subtle); border-radius: var(--radius-sm); background: transparent; color: var(--color-text-primary); font: var(--weight-display) var(--text-tiny)/1 var(--font-body); cursor: pointer; }
.feedback-btn:hover { background: var(--color-bg-surface); }
</style>
