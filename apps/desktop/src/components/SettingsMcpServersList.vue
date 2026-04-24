<script setup lang="ts">
import type { McpServerEntry } from '../lib/types'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Plus, Pencil, Trash2 } from 'lucide-vue-next'

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
  <section
    class="flex flex-col gap-4 rounded-lg border border-border-card bg-bg-elevated p-4"
    role="group"
    aria-label="Configured MCP servers"
  >
    <header class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
      <div class="flex flex-col gap-1">
        <h3 class="text-sm font-semibold text-text-primary">MCP servers</h3>
        <p class="text-xs text-text-secondary">
          Manage the shared global MCP server entries available to every configured agent.
        </p>
      </div>
      <Button
        variant="outline"
        size="sm"
        :disabled="showAddForm || !!editingServer"
        @click="emit('add')"
      >
        <Plus class="h-4 w-4" />
        Add MCP server
      </Button>
    </header>

    <div
      v-if="feedback"
      role="status"
      aria-live="polite"
      class="rounded-md border px-3 py-2 text-xs"
      :class="feedback.type === 'success'
        ? 'border-status-completed/40 bg-status-completed/10 text-text-primary'
        : 'border-danger/40 bg-danger/10 text-danger'"
    >
      {{ feedback.message }}
    </div>

    <div v-if="mcpServers.length === 0" class="rounded-md border border-dashed border-border-subtle p-4 text-xs text-text-secondary">
      No MCP servers configured yet. Add one to share it across all agents.
    </div>

    <div v-else class="rounded-md border border-border-subtle">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Name</TableHead>
            <TableHead>Type</TableHead>
            <TableHead class="w-[1%] text-right">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow v-for="server in mcpServers" :key="server.name">
            <TableCell class="font-medium text-text-primary">{{ server.name }}</TableCell>
            <TableCell>
              <Badge variant="secondary">{{ server.type }}</Badge>
            </TableCell>
            <TableCell class="text-right">
              <div class="flex justify-end gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  :disabled="showAddForm || !!editingServer"
                  @click="emit('edit', server)"
                >
                  <Pencil class="h-3.5 w-3.5" />
                  Edit
                </Button>
                <Button
                  variant="destructive"
                  size="sm"
                  :disabled="showAddForm || !!editingServer"
                  @click="emit('delete', server.name)"
                >
                  <Trash2 class="h-3.5 w-3.5" />
                  Delete
                </Button>
              </div>
            </TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </div>
  </section>
</template>
