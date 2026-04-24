<script setup lang="ts">
import { Search } from 'lucide-vue-next'
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
    <div class="relative px-4 pt-4 pb-3">
      <Search
        class="pointer-events-none absolute left-7 top-1/2 size-3.5 -translate-y-1/2 text-text-secondary"
        aria-hidden="true"
        :stroke-width="2"
      />
      <Input
        :model-value="props.searchQuery"
        type="text"
        class="h-9 w-full pl-8 text-sm"
        placeholder="Find workflow..."
        aria-label="Find workflow"
        @update:model-value="(value) => emit('update:searchQuery', String(value))"
      />
    </div>

    <div
      class="flex shrink-0 gap-2 overflow-x-auto px-4 pb-4 [&::-webkit-scrollbar]:hidden [scrollbar-width:none]"
    >
      <Button
        v-for="chip in props.chips"
        :key="chip.value"
        type="button"
        variant="ghost"
        size="sm"
        :class="[
          'h-7 shrink-0 whitespace-nowrap px-2 text-xs font-normal',
          props.categoryFilter === chip.value
            ? 'bg-bg-surface-hover text-text-primary hover:bg-bg-surface-hover'
            : 'text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary',
        ]"
        :aria-pressed="props.categoryFilter === chip.value"
        @click="emit('update:categoryFilter', chip.value)"
      >
        {{ chip.label }}
      </Button>
    </div>

    <div
      v-if="props.browserError"
      class="mx-4 mb-3 flex flex-wrap items-center gap-2 rounded-md border border-border-error bg-bg-error p-3 text-text-error"
      role="alert"
    >
      <span class="flex-1 break-words text-xs font-medium leading-snug">
        {{ props.browserError }}
      </span>
      <Button
        type="button"
        variant="outline"
        size="sm"
        class="h-6 px-2 text-xs"
        @click="emit('retry')"
      >
        Retry
      </Button>
    </div>

    <div
      v-if="props.importConflict || props.importError"
      :class="[
        'mx-4 mb-3 flex flex-wrap items-center gap-2 rounded-md border p-3',
        props.importError
          ? 'border-border-error bg-bg-error text-text-error'
          : 'border-border-subtle bg-bg-elevated text-text-primary',
      ]"
      :role="props.importError ? 'alert' : 'status'"
      aria-live="polite"
    >
      <template v-if="props.importConflict">
        <span class="flex-1 break-words text-xs font-medium leading-snug">
          Workflow ID
          <code class="rounded bg-bg-code px-1 font-mono text-text-code">{{ props.importConflict.id }}</code>
          already exists ({{ props.importConflict.name }}).
        </span>
        <Button
          type="button"
          variant="outline"
          size="sm"
          class="h-6 px-2 text-xs"
          @click="emit('resolve-conflict', 'overwrite')"
        >
          Overwrite
        </Button>
        <Button
          type="button"
          variant="outline"
          size="sm"
          class="h-6 px-2 text-xs"
          @click="emit('resolve-conflict', 'copy')"
        >
          Import as Copy
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          class="h-6 px-2 text-xs"
          @click="emit('clear-import-state')"
        >
          Cancel
        </Button>
      </template>
      <template v-else-if="props.importError">
        <span class="flex-1 break-words text-xs font-medium leading-snug">
          {{ props.importError }}
        </span>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          class="h-6 px-2 text-xs"
          @click="emit('dismiss-import-error')"
        >
          Dismiss
        </Button>
      </template>
    </div>
  </div>
</template>
