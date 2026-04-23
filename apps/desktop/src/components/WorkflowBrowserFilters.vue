<script setup lang="ts">
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
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
</script>

<template>
  <div role="search" aria-label="Workflow filters">
    <div class="relative p-4 pb-3">
      <svg class="absolute left-[calc(var(--space-4)+var(--space-3))] top-[calc(50%+var(--space-1)/2)] -translate-y-1/2 text-[var(--color-text-secondary)] pointer-events-none" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="11" cy="11" r="8" />
        <line x1="21" y1="21" x2="16.65" y2="16.65" />
      </svg>
      <Input
        :model-value="searchQuery"
        type="text"
        class="w-full py-2 pr-3 pl-[calc(var(--space-3)+20px)] bg-[var(--surface)] border border-transparent rounded-[var(--radius-sm)] text-[var(--color-text-primary)] [font-weight:var(--weight-body)] text-[length:var(--text-caption)] leading-[1.4] [font-family:var(--font-body)] outline-none transition-[border-color,box-shadow,background] duration-[var(--transition-fast)] placeholder:text-[var(--color-text-secondary)] hover:[&:not(:focus)]:bg-[var(--surface-high)] focus:bg-[var(--surface-high)] focus:border-[var(--color-brand)] focus:shadow-none"
        placeholder="Find workflow..."
        @update:model-value="(v) => emit('update:searchQuery', String(v))"
      />
    </div>

    <div class="chip-row flex gap-2 px-4 pb-4 overflow-x-auto shrink-0">
      <Button
        v-for="chip in chips"
        :key="chip.value"
        variant="ghost"
        size="sm"
        :class="[
          'shrink-0 py-1 px-2 border border-transparent rounded-[var(--radius-sm)] bg-transparent text-[var(--color-text-secondary)] [font-weight:var(--weight-body)] text-[length:var(--text-caption)] leading-[1.4] [font-family:var(--font-body)] cursor-pointer whitespace-nowrap transition-[background,color] duration-[var(--transition-fast)] hover:bg-[var(--color-text-tertiary)] hover:text-[var(--color-text-primary)]',
          categoryFilter === chip.value && 'bg-[var(--color-brand-light,rgba(0,120,212,0.15))] text-[var(--color-text-primary)]'
        ]"
        type="button"
        @click="emit('update:categoryFilter', chip.value)"
      >
        {{ chip.label }}
      </Button>
    </div>

    <div v-if="browserError" class="flex flex-wrap gap-2 items-center mx-4 mb-3 p-3 border border-[var(--color-border-subtle)] rounded-[var(--radius-md)] bg-[var(--color-bg-elevated)]" :style="{ borderColor: 'color-mix(in srgb, var(--color-status-abandoned) 45%, transparent)', color: 'var(--color-status-abandoned)' }" role="alert">
      <span class="flex-1 [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-body)] break-words">{{ browserError }}</span>
      <Button variant="outline" size="xs" class="min-h-6 px-2 border border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] bg-transparent text-[var(--color-text-primary)] [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-none [font-family:var(--font-body)] cursor-pointer hover:bg-[var(--color-bg-surface)]" type="button" @click="emit('retry')">Retry</Button>
    </div>

    <div
      v-if="props.importConflict || props.importError"
      class="flex flex-wrap gap-2 items-center mx-4 mb-3 p-3 border border-[var(--color-border-subtle)] rounded-[var(--radius-md)] bg-[var(--color-bg-elevated)]"
      :style="props.importError ? { borderColor: 'color-mix(in srgb, var(--color-status-abandoned) 45%, transparent)', color: 'var(--color-status-abandoned)' } : {}"
      :role="props.importError ? 'alert' : 'status'"
      aria-live="polite"
    >
      <template v-if="props.importConflict">
        <span class="flex-1 [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-body)] break-words">
          Workflow ID <code>{{ props.importConflict.id }}</code> already exists ({{ props.importConflict.name }}).
        </span>
        <Button variant="outline" size="xs" class="min-h-6 px-2 border border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] bg-transparent text-[var(--color-text-primary)] [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-none [font-family:var(--font-body)] cursor-pointer hover:bg-[var(--color-bg-surface)]" type="button" @click="emit('resolve-conflict', 'overwrite')">Overwrite</Button>
        <Button variant="outline" size="xs" class="min-h-6 px-2 border border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] bg-transparent text-[var(--color-text-primary)] [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-none [font-family:var(--font-body)] cursor-pointer hover:bg-[var(--color-bg-surface)]" type="button" @click="emit('resolve-conflict', 'copy')">Import as Copy</Button>
        <Button variant="ghost" size="xs" class="min-h-6 px-2 border border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] bg-transparent text-[var(--color-text-primary)] [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-none [font-family:var(--font-body)] cursor-pointer hover:bg-[var(--color-bg-surface)]" type="button" @click="emit('clear-import-state')">Cancel</Button>
      </template>
      <template v-else-if="props.importError">
        <span class="flex-1 [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-[1.4] [font-family:var(--font-body)] break-words">{{ props.importError }}</span>
        <Button variant="ghost" size="xs" class="min-h-6 px-2 border border-[var(--color-border-subtle)] rounded-[var(--radius-sm)] bg-transparent text-[var(--color-text-primary)] [font-weight:var(--weight-display)] text-[length:var(--text-tiny)] leading-none [font-family:var(--font-body)] cursor-pointer hover:bg-[var(--color-bg-surface)]" type="button" @click="emit('dismiss-import-error')">Dismiss</Button>
      </template>
    </div>
  </div>
</template>

<!-- audit-ok: webkit-scrollbar -->
<style scoped>
.chip-row::-webkit-scrollbar { display: none; }
</style>
