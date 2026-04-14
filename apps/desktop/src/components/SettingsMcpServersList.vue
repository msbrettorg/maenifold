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

function getServerDetails(server: McpServerEntry): string {
  return server.type === 'stdio' ? [server.command, ...(server.args ?? [])].join(' ') : server.url
}

function getServerMeta(server: McpServerEntry): string | null {
  if (server.type !== 'stdio' || !server.args?.length) return null
  return `${server.args.length} arg${server.args.length === 1 ? '' : 's'}`
}
</script>

<template>
  <div class="settings-section" role="group" aria-label="Configured MCP servers">
    <div class="section-header">
      <div>
        <h3>MCP servers</h3>
        <p>Manage the shared global MCP server entries available to every configured agent.</p>
      </div>
      <button class="add-btn" :disabled="showAddForm || !!editingServer" @click="emit('add')">+ Add MCP Server</button>
    </div>

    <p v-if="feedback" class="feedback" :class="feedback.type">{{ feedback.message }}</p>

    <div class="agents-list" role="list" aria-label="Configured MCP servers">
      <article v-for="server in mcpServers" :key="server.name" class="agent-item" role="listitem">
        <div class="agent-info">
          <div class="agent-name-row">
            <div class="agent-name">{{ server.name }}</div>
            <span class="type-badge">{{ server.type }}</span>
          </div>
          <code class="agent-command">{{ getServerDetails(server) }}</code>
          <small v-if="getServerMeta(server)" class="server-meta">{{ getServerMeta(server) }}</small>
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
</template>

<style scoped>
.settings-section,.agents-list { display: flex; flex-direction: column; }
.settings-section { gap: 1rem; }
.section-header,.agent-item,.agent-actions,.agent-name-row { display: flex; gap: 0.75rem; }
.section-header,.agent-item,.agent-actions { justify-content: space-between; }
.section-header h3,.section-header p,.empty-state,.server-meta { margin: 0; }
.section-header p,.empty-state,.server-meta { color: var(--muted-foreground); }
.add-btn,.edit-btn,.delete-btn { border-radius: 6px; cursor: pointer; }
.add-btn,.edit-btn { border: 1px solid var(--border); background: transparent; color: inherit; }
.add-btn { padding: 0.5rem 0.9rem; }
.agent-item { padding: 1rem; border-radius: 8px; background: var(--card); align-items: center; }
.feedback { margin: 0; padding: 0.75rem 1rem; border: 1px solid var(--border); border-radius: 8px; font-size: 0.75rem; }
.feedback.success { border-color: color-mix(in srgb, var(--color-status-completed) 45%, transparent); background: color-mix(in srgb, var(--color-status-completed) 14%, var(--card)); color: var(--foreground); }
.feedback.error { border-color: color-mix(in srgb, var(--destructive) 45%, transparent); background: color-mix(in srgb, var(--destructive) 12%, var(--card)); color: var(--destructive); }
.delete-btn { border: 1px solid var(--destructive); background: transparent; color: var(--destructive); padding: 0.25rem 0.6rem; }
.agent-info { min-width: 0; }
.agent-name-row { align-items: center; justify-content: flex-start; }
.agent-name { font-weight: 600; }
.agent-command { display: block; margin-top: 0.25rem; word-break: break-all; font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace; font-size: 0.75rem; }
.server-meta { font-size: 0.75rem; }
.type-badge { display: inline-flex; align-items: center; padding: 0.15rem 0.45rem; border-radius: 999px; background: var(--muted); color: var(--muted-foreground); font-size: 0.75rem; text-transform: lowercase; }
@media (max-width: 640px) {
  .section-header,.agent-item { flex-direction: column; align-items: stretch; }
  .agent-actions { justify-content: flex-start; }
}
</style>
