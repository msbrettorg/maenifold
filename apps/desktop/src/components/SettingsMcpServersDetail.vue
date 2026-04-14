<script setup lang="ts">
import type { McpServerEntry } from '../lib/types'

type McpServerType = McpServerEntry['type']

defineProps<{
  visible: boolean
  editingServer: string | null
  formName: string
  formType: McpServerType
  formCommand: string
  formArgs: string
  formUrl: string
  formError: string
  isSubmitting: boolean
}>()

const emit = defineEmits<{
  (e: 'update:formName', value: string): void
  (e: 'update:formType', value: McpServerType): void
  (e: 'update:formCommand', value: string): void
  (e: 'update:formArgs', value: string): void
  (e: 'update:formUrl', value: string): void
  (e: 'submit'): void
  (e: 'cancel'): void
}>()

function updateValue(event: Event, field: 'formName' | 'formCommand' | 'formArgs' | 'formUrl') {
  const value = (event.target as HTMLInputElement | HTMLTextAreaElement).value
  emit(`update:${field}` as never, value)
}

function updateType(event: Event) {
  emit('update:formType', (event.target as HTMLSelectElement).value as McpServerType)
}
</script>

<template>
  <div v-if="visible" class="agent-form" role="group" :aria-label="editingServer ? `Edit MCP server ${editingServer}` : 'Add MCP server'">
    <h4>{{ editingServer ? 'Edit MCP server' : 'Add new MCP server' }}</h4>

    <label class="form-group">
      <span>Name</span>
      <input :value="formName" type="text" placeholder="filesystem" :disabled="!!editingServer" @input="updateValue($event, 'formName')" />
    </label>

    <label class="form-group">
      <span>Type</span>
      <select :value="formType" @change="updateType">
        <option value="stdio">stdio</option>
        <option value="http">http</option>
      </select>
    </label>

    <template v-if="formType === 'stdio'">
      <label class="form-group">
        <span>Command</span>
        <input :value="formCommand" type="text" placeholder="npx @modelcontextprotocol/server-filesystem" @input="updateValue($event, 'formCommand')" />
      </label>

      <label class="form-group">
        <span>Arguments</span>
        <textarea :value="formArgs" rows="4" placeholder="workspace-folder&#10;--read-only" @input="updateValue($event, 'formArgs')" />
        <small>Enter one argument per line.</small>
      </label>
    </template>

    <label v-else class="form-group">
      <span>URL</span>
      <input :value="formUrl" type="url" placeholder="localhost:8080/mcp" @input="updateValue($event, 'formUrl')" />
    </label>

    <p v-if="formError" class="form-error">{{ formError }}</p>

    <div class="form-actions">
      <button class="save-btn" :disabled="isSubmitting" @click="emit('submit')">
        {{ isSubmitting ? 'Saving...' : 'Save' }}
      </button>
      <button class="cancel-btn" @click="emit('cancel')">Cancel</button>
    </div>
  </div>
</template>

<style scoped>
.agent-form { padding: 1rem; border-radius: 8px; background: var(--card); }
.agent-form h4 { margin: 0 0 0.75rem; }
.form-group { display: block; margin-bottom: 0.75rem; }
.form-group span { display: block; margin-bottom: 0.25rem; font-weight: 500; }
.form-group input,.form-group select,.form-group textarea { width: 100%; padding: 0.5rem; border: 1px solid var(--border); border-radius: 6px; background: var(--background); color: var(--foreground); }
.form-group textarea { resize: vertical; }
.form-group small { font-size: 0.75rem; }
.form-error { color: var(--destructive); }
.form-actions { display: flex; justify-content: space-between; gap: 0.75rem; }
.save-btn,.cancel-btn { padding: 0.5rem 0.9rem; border-radius: 6px; cursor: pointer; }
.save-btn { border: none; background: var(--ring); color: white; }
.cancel-btn { border: 1px solid var(--border); background: transparent; color: inherit; }
@media (max-width: 640px) {
  .form-actions { flex-direction: column; align-items: stretch; }
}
</style>
