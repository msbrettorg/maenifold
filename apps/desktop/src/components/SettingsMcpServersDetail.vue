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
  <div v-if="visible" class="p-4 rounded bg-bg-elevated" role="group" :aria-label="editingServer ? `Edit MCP server ${editingServer}` : 'Add MCP server'">
    <h4 class="m-0 mb-3">{{ editingServer ? 'Edit MCP server' : 'Add new MCP server' }}</h4>

    <div class="block mb-3">
      <Label>Name</Label>
      <Input :model-value="formName" type="text" placeholder="filesystem" :disabled="!!editingServer" @update:model-value="(val) => updateValue({ target: { value: String(val) } } as unknown as Event, 'formName')" />
    </div>

    <div class="block mb-3">
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
      <div class="block mb-3">
        <Label>Command</Label>
        <Input :model-value="formCommand" type="text" placeholder="npx @modelcontextprotocol/server-filesystem" @update:model-value="(val) => updateValue({ target: { value: String(val) } } as unknown as Event, 'formCommand')" />
      </div>

      <div class="block mb-3">
        <Label>Arguments</Label>
        <Textarea :value="formArgs" rows="4" placeholder="workspace-folder&#10;--read-only" class="resize-y" @input="updateValue($event, 'formArgs')" />
        <small class="text-xs">Enter one argument per line.</small>
      </div>
    </template>

    <div v-else class="block mb-3">
      <Label>URL</Label>
      <Input :model-value="formUrl" type="url" placeholder="localhost:8080/mcp" @update:model-value="(val) => updateValue({ target: { value: String(val) } } as unknown as Event, 'formUrl')" />
    </div>

    <p v-if="formError" class="text-danger">{{ formError }}</p>

    <div class="flex justify-between gap-3 max-sm:flex-col max-sm:items-stretch">
      <Button class="px-3.5 py-2 rounded cursor-pointer border-0 bg-brand text-text-primary" :disabled="isSubmitting" @click="emit('submit')">
        {{ isSubmitting ? 'Saving...' : 'Save' }}
      </Button>
      <Button variant="outline" class="px-3.5 py-2 rounded cursor-pointer border border-border-subtle bg-transparent text-inherit" @click="emit('cancel')">Cancel</Button>
    </div>
  </div>
</template>
