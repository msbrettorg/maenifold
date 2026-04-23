<script setup lang="ts">
import type { McpServerEntry } from '../lib/types'
import { Button } from '@/components/ui/button'

defineProps<{
  mcpServers: McpServerEntry[]
  showAddForm: boolean
  editingServer: string | null
  feedback: { type: 'success' | 'error'; message: string } | null
}>()

const emit = defineEmits<{
  (e: 'add'): void
  (e: 'edit', server: McpServerEntry): void
  (e: 'delete', name: string): void
}>()
</script>

<template>
  <div class="flex flex-col gap-4" role="group" aria-label="Configured MCP servers">
    <div class="rounded-[8px] border border-border-card bg-bg-elevated p-4">
      <div class="flex justify-between gap-3 mb-3 text-[11px] font-semibold tracking-[0.06em] text-text-tertiary max-sm:flex-col max-sm:items-stretch">
        <div>
          <h3 class="m-0">MCP servers</h3>
          <p class="m-0 text-text-secondary">Manage the shared global MCP server entries available to every configured agent.</p>
        </div>
        <Button variant="outline" class="rounded-[var(--radius)] cursor-pointer border border-border-subtle bg-transparent text-inherit px-3.5 py-2" :disabled="showAddForm || !!editingServer" @click="emit('add')">+ Add MCP server</Button>
      </div>

      <p
        v-if="feedback"
        class="m-0 px-4 py-3 border border-border-subtle rounded-[var(--radius)] text-xs"
        :class="feedback.type"
        :style="feedback.type === 'success'
          ? 'border-color: color-mix(in srgb, var(--color-status-completed) 45%, transparent); background: color-mix(in srgb, var(--color-status-completed) 14%, var(--color-bg-elevated)); color: var(--color-text-primary);'
          : 'border-color: color-mix(in srgb, var(--color-danger) 45%, transparent); background: color-mix(in srgb, var(--color-danger) 12%, var(--color-bg-elevated)); color: var(--color-danger);'"
      >{{ feedback.message }}</p>
      <!-- audit-ok: dynamic-binding -->

      <div class="flex flex-col" role="list" aria-label="Configured MCP servers">
        <article
          v-for="server in mcpServers"
          :key="server.name"
          class="flex justify-between gap-3 items-center p-4 rounded-[var(--radius)] bg-bg-elevated max-sm:flex-col max-sm:items-stretch"
          role="listitem"
        >
          <div class="min-w-0">
            <div class="flex gap-3 items-center justify-start">
              <div class="font-semibold">{{ server.name }}</div>
              <span class="inline-flex items-center px-1.5 py-0.5 rounded bg-text-tertiary text-text-secondary text-xs">{{ server.type }}</span>
            </div>
          </div>
          <div class="flex justify-between gap-3 items-center max-sm:justify-start">
            <Button variant="outline" class="rounded-[var(--radius)] cursor-pointer border border-border-subtle bg-transparent text-inherit" @click="emit('edit', server)">Edit</Button>
            <Button variant="destructive" class="rounded-[var(--radius)] cursor-pointer border border-danger bg-transparent text-danger px-2.5 py-1" @click="emit('delete', server.name)">Delete</Button>
          </div>
        </article>

        <p v-if="mcpServers.length === 0" class="m-0 text-text-secondary">
          No MCP servers configured yet. Add one to share it across all agents.
        </p>
      </div>
    </div>
  </div>
</template>
