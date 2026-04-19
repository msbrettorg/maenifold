<script setup lang="ts">
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

interface EnvVarRow {
  key: string;
  value: string;
}

interface McpServerFormRow {
  type: 'stdio' | 'http';
  name: string;
  commandOrUrl: string;
  args: string;
}

defineProps<{
  form: {
    env: EnvVarRow[];
    mcpServers: McpServerFormRow[];
  };
  onAddEnvRow: () => void;
  onRemoveEnvRow: (index: number) => void;
  onAddMcpServerRow: () => void;
  onRemoveMcpServerRow: (index: number) => void;
}>();

function formatServerCount(count: number) {
  return `${count} MCP server${count === 1 ? '' : 's'}`;
}

function formatEnvCount(count: number) {
  return `${count} env var${count === 1 ? '' : 's'}`;
}
</script>

<template>
  <details class="form-section details-section" open>
    <summary>
      <div>
        <h5 class="text-sm font-medium text-text-secondary">Environment variables</h5>
        <p>Optional key/value pairs passed to this agent process.</p>
      </div>
      <span class="summary-count">{{ formatEnvCount(form.env.length) }}</span>
    </summary>

    <div class="section-body">
      <div v-if="form.env.length === 0" class="empty-detail">No environment variables configured.</div>
      <div v-for="(entry, index) in form.env" :key="`env-${index}`" class="kv-row">
        <Input v-model="entry.key" type="text" placeholder="KEY" />
        <Input v-model="entry.value" type="text" placeholder="value" />
        <Button variant="ghost" size="icon" class="remove-btn" type="button" aria-label="Remove variable" @click="onRemoveEnvRow(index)">×</Button>
      </div>
      <Button variant="outline" class="add-inline-btn" type="button" @click="onAddEnvRow">+ Add Variable</Button>
    </div>
  </details>

  <details class="form-section details-section" open>
    <summary>
      <div>
        <h5 class="text-sm font-medium text-text-secondary">Per-agent MCP servers</h5>
        <p>Optional MCP servers available only to this agent.</p>
      </div>
      <span class="summary-count">{{ formatServerCount(form.mcpServers.length) }}</span>
    </summary>

    <div class="section-body">
      <div v-if="form.mcpServers.length === 0" class="empty-detail">No per-agent MCP servers configured.</div>
      <div v-for="(server, index) in form.mcpServers" :key="`mcp-${index}`" class="server-editor">
        <div class="server-editor-header">
          <strong>Server {{ index + 1 }}</strong>
          <Button variant="ghost" size="icon" class="remove-btn" type="button" aria-label="Remove MCP server" @click="onRemoveMcpServerRow(index)">
            ×
          </Button>
        </div>

        <div class="server-grid">
          <div class="form-group">
            <Label>Type</Label>
            <Select :model-value="server.type" @update:model-value="(val) => server.type = val as 'stdio' | 'http'">
              <SelectTrigger class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="stdio">stdio</SelectItem>
                <SelectItem value="http">http</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="form-group">
            <Label>Name</Label>
            <Input v-model="server.name" type="text" placeholder="server-name" />
          </div>
          <div class="form-group server-primary-field">
            <Label>{{ server.type === 'http' ? 'URL' : 'Command' }}</Label>
            <Input
              v-model="server.commandOrUrl"
              type="text"
              :placeholder="server.type === 'http' ? 'https://example.test/mcp' : 'npx @scope/server'"
            />
          </div>
          <div v-if="server.type === 'stdio'" class="form-group server-primary-field">
            <Label>Args</Label>
            <Input v-model="server.args" type="text" placeholder="--flag value" />
            <small>Space-separated arguments.</small>
          </div>
        </div>
      </div>

      <Button variant="outline" class="add-inline-btn" type="button" @click="onAddMcpServerRow">+ Add MCP Server</Button>
    </div>
  </details>
</template>

<style scoped>
.form-section,
.section-body,
.server-grid,
.server-editor {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.form-section {
  padding: 0.9rem;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: var(--color-bg-app);
}

.details-section summary,
.kv-row,
.server-editor-header {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
}

.details-section summary {
  align-items: center;
  list-style: none;
  cursor: pointer;
}

.details-section summary::-webkit-details-marker {
  display: none;
}

.details-section[open] summary {
  margin-bottom: 0.75rem;
}

.details-section h5,
.details-section p {
  margin: 0;
}

.details-section p,
.summary-count,
.empty-detail {
  color: var(--color-text-secondary);
}

.summary-count,
.empty-detail,
.form-group small {
  font-size: 0.75rem;
}

.summary-count {
  padding: 0.15rem 0.5rem;
  border-radius: 4px;
  background: var(--color-bg-elevated);
  white-space: nowrap;
}

.form-group span {
  display: block;
  margin-bottom: 0.25rem;
  font-weight: 500;
}

.form-group input,
.form-group select {
  width: 100%;
  padding: 0.5rem;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: var(--color-bg-app);
  color: var(--color-text-primary);
}

.kv-row {
  align-items: end;
}

.kv-row input:first-child {
  max-width: 14rem;
}

.server-editor {
  padding: 0.75rem;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: var(--color-bg-elevated);
}

.server-editor-header {
  align-items: center;
}

.server-primary-field {
  width: 100%;
}

.add-inline-btn,
.remove-btn {
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: transparent;
  color: inherit;
  cursor: pointer;
}

.add-inline-btn {
  padding: 0.5rem 0.9rem;
}

.remove-btn {
  width: 2rem;
  height: 2rem;
  padding: 0;
  align-self: end;
  font-size: 0.9375rem;
  line-height: 1;
}

@media (max-width: 640px) {
  .details-section summary,
  .kv-row {
    flex-direction: column;
    align-items: stretch;
  }

  .kv-row input:first-child {
    max-width: none;
  }

  .remove-btn {
    align-self: flex-start;
  }
}
</style>
