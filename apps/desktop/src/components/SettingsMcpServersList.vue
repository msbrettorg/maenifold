<script setup lang="ts">
import type { McpServerEntry } from '../lib/types'

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
  <div class="settings-section" role="group" aria-label="Configured MCP servers">
    <div class="rounded-[8px] border border-border-card bg-bg-elevated p-4">
      <div class="section-header">
        <div>
          <h3>MCP servers</h3>
          <p>Manage the shared global MCP server entries available to every configured agent.</p>
        </div>
        <button class="add-btn" :disabled="showAddForm || !!editingServer" @click="emit('add')">+ Add MCP server</button>
      </div>

      <p v-if="feedback" class="feedback" :class="feedback.type">{{ feedback.message }}</p>

      <div class="agents-list" role="list" aria-label="Configured MCP servers">
        <article v-for="server in mcpServers" :key="server.name" class="agent-item" role="listitem">
          <div class="agent-info">
            <div class="agent-name-row">
              <div class="agent-name">{{ server.name }}</div>
              <span class="type-badge">{{ server.type }}</span>
            </div>
          </div>
          <div class="agent-actions">
            <button class="edit-btn" @click="emit('edit', server)">Edit</button>
            <button class="delete-btn" @click="emit('delete', server.name)">Delete</button>
          </div>
        </article>

        <p v-if="mcpServers.length === 0" class="empty-state">
          No MCP servers configured yet. Add one to share it across all agents.
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-section,.agents-list { display: flex; flex-direction: column; }
.settings-section { gap: 1rem; }
.section-header,.agent-item,.agent-actions,.agent-name-row { display: flex; gap: 0.75rem; }
.section-header,.agent-item,.agent-actions { justify-content: space-between; }
.section-header {
  margin-bottom: 0.75rem;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.06em;
  color: var(--text-tertiary);
}
.section-header h3,.section-header p,.empty-state { margin: 0; }
.section-header p,.empty-state { color: var(--color-text-secondary); }
.add-btn,.edit-btn,.delete-btn { border-radius: var(--radius); cursor: pointer; }
.add-btn,.edit-btn { border: 1px solid var(--color-border-subtle); background: transparent; color: inherit; }
.add-btn { padding: 0.5rem 0.9rem; }
.agent-item { padding: 1rem; border-radius: var(--radius); background: var(--color-bg-elevated); align-items: center; }
.feedback { margin: 0; padding: 0.75rem 1rem; border: 1px solid var(--color-border-subtle); border-radius: var(--radius); font-size: 0.75rem; }
.feedback.success { border-color: color-mix(in srgb, var(--color-status-completed) 45%, transparent); background: color-mix(in srgb, var(--color-status-completed) 14%, var(--color-bg-elevated)); color: var(--color-text-primary); }
.feedback.error { border-color: color-mix(in srgb, var(--color-danger) 45%, transparent); background: color-mix(in srgb, var(--color-danger) 12%, var(--color-bg-elevated)); color: var(--color-danger); }
.delete-btn { border: 1px solid var(--color-danger); background: transparent; color: var(--color-danger); padding: 0.25rem 0.6rem; }
.agent-info { min-width: 0; }
.agent-name-row { align-items: center; justify-content: flex-start; }
.agent-name { font-weight: 600; }
.type-badge { display: inline-flex; align-items: center; padding: 0.15rem 0.45rem; border-radius: 4px; background: var(--color-text-tertiary); color: var(--color-text-secondary); font-size: 0.75rem; }
@media (max-width: 640px) {
  .section-header,.agent-item { flex-direction: column; align-items: stretch; }
  .agent-actions { justify-content: flex-start; }
}
</style>
