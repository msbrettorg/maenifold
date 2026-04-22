<script setup lang="ts">
import type { McpServerEntry } from '../lib/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

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

    <div class="form-group">
      <Label>Name</Label>
      <Input :model-value="formName" type="text" placeholder="filesystem" :disabled="!!editingServer" @update:model-value="(val) => updateValue({ target: { value: String(val) } } as unknown as Event, 'formName')" />
    </div>

    <div class="form-group">
      <Label>Type</Label>
      <Select :model-value="formType" @update:model-value="(val) => updateType({ target: { value: String(val) } } as unknown as Event)">
        <SelectTrigger class="w-full">
          <SelectValue placeholder="Select type" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="stdio">stdio</SelectItem>
          <SelectItem value="http">http</SelectItem>
        </SelectContent>
      </Select>
    </div>

    <template v-if="formType === 'stdio'">
      <div class="form-group">
        <Label>Command</Label>
        <Input :model-value="formCommand" type="text" placeholder="npx @modelcontextprotocol/server-filesystem" @update:model-value="(val) => updateValue({ target: { value: String(val) } } as unknown as Event, 'formCommand')" />
      </div>

      <div class="form-group">
        <Label>Arguments</Label>
        <Textarea :value="formArgs" rows="4" placeholder="workspace-folder&#10;--read-only" @input="updateValue($event, 'formArgs')" />
        <small>Enter one argument per line.</small>
      </div>
    </template>

    <div v-else class="form-group">
      <Label>URL</Label>
      <Input :model-value="formUrl" type="url" placeholder="localhost:8080/mcp" @update:model-value="(val) => updateValue({ target: { value: String(val) } } as unknown as Event, 'formUrl')" />
    </div>

    <p v-if="formError" class="form-error">{{ formError }}</p>

    <div class="form-actions">
      <Button class="save-btn" :disabled="isSubmitting" @click="emit('submit')">
        {{ isSubmitting ? 'Saving...' : 'Save' }}
      </Button>
      <Button variant="outline" class="cancel-btn" @click="emit('cancel')">Cancel</Button>
    </div>
  </div>
</template>

<style scoped>
.agent-form { padding: 1rem; border-radius: var(--radius); background: var(--color-bg-elevated); }
.agent-form h4 { margin: 0 0 0.75rem; }
.form-group { display: block; margin-bottom: 0.75rem; }
.form-group span { display: block; margin-bottom: 0.25rem; font-weight: 500; }
.form-group input,.form-group select,.form-group textarea { width: 100%; padding: 0.5rem; border: 1px solid var(--color-border-subtle); border-radius: var(--radius); background: var(--color-bg-app); color: var(--color-text-primary); }
.form-group textarea { resize: vertical; }
.form-group small { font-size: 0.75rem; }
.form-error { color: var(--color-danger); }
.form-actions { display: flex; justify-content: space-between; gap: 0.75rem; }
.save-btn,.cancel-btn { padding: 0.5rem 0.9rem; border-radius: var(--radius); cursor: pointer; }
.save-btn { border: none; background: var(--color-brand); color: var(--color-text-primary); }
.cancel-btn { border: 1px solid var(--color-border-subtle); background: transparent; color: inherit; }
@media (max-width: 640px) {
  .form-actions { flex-direction: column; align-items: stretch; }
}
</style>
