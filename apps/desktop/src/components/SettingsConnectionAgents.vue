<script setup lang="ts">
import type { AgentProbeResult } from '../lib/agent-probe-types';
import type { McpServerEntry } from '../lib/types';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import SettingsConnectionKeys from './SettingsConnectionKeys.vue';

interface AgentRow {
  name: string;
  command: string;
  env: Record<string, string>;
  mcpServers: McpServerEntry[];
}

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

const props = defineProps<{
  agents: AgentRow[];
  showAddForm: boolean;
  editingAgent: string | null;
  form: {
    name: string;
    command: string;
    env: EnvVarRow[];
    mcpServers: McpServerFormRow[];
    error: string;
  };
  isSubmitting: boolean;
  onStartAdd: () => void;
  onStartEdit: (agent: AgentRow) => void;
  onResetForm: () => void;
  onSubmit: () => void | Promise<void>;
  onDelete: (name: string) => void | Promise<void>;
  onAddEnvRow: () => void;
  onRemoveEnvRow: (index: number) => void;
  onAddMcpServerRow: () => void;
  onRemoveMcpServerRow: (index: number) => void;
  getProbe: (agentName: string) => AgentProbeResult | undefined;
}>();

function probeStateClass(name: string): string {
  const probe = props.getProbe(name);
  if (!probe) return 'pending';
  return probe.state === 'ready' ? 'ready' : probe.state === 'offline' ? 'offline' : 'error';
}

function formatServerCount(count: number) {
  return `${count} MCP server${count === 1 ? '' : 's'}`;
}

function formatEnvCount(count: number) {
  return `${count} env var${count === 1 ? '' : 's'}`;
}
</script>

<template>
  <section class="settings-section">
    <div class="rounded-[8px] border border-border-card bg-bg-elevated p-4">
      <div class="section-header">
        <div>
          <h4 class="text-sm font-medium text-text-secondary">Agents</h4>
          <p>Add, edit, and remove ACP agent definitions.</p>
        </div>
        <Button variant="outline" class="add-btn" :disabled="showAddForm" @click="onStartAdd">+ Add agent</Button>
      </div>

      <div v-if="showAddForm || editingAgent" class="agent-form">
        <h5>{{ editingAgent ? 'Edit agent' : 'Add new agent' }}</h5>

        <div class="form-section">
          <div class="form-group">
            <Label>Name</Label>
            <Input v-model="form.name" type="text" placeholder="My Agent" :disabled="editingAgent !== null" />
          </div>
          <div class="form-group">
            <Label>Command</Label>
            <Input v-model="form.command" type="text" placeholder="npx @github/copilot-language-server@latest --acp" />
            <small>Full shell command including arguments.</small>
          </div>
        </div>

        <SettingsConnectionKeys
          :form="form"
          :on-add-env-row="onAddEnvRow"
          :on-add-mcp-server-row="onAddMcpServerRow"
          :on-remove-env-row="onRemoveEnvRow"
          :on-remove-mcp-server-row="onRemoveMcpServerRow"
        />

        <p v-if="form.error" class="form-error">{{ form.error }}</p>
        <div class="form-actions">
          <Button class="save-btn" :disabled="isSubmitting" @click="onSubmit">
            {{ isSubmitting ? 'Saving...' : 'Save' }}
          </Button>
          <Button variant="outline" class="cancel-btn" @click="onResetForm">Cancel</Button>
        </div>
      </div>

      <div class="agents-list">
        <article
          v-for="agent in agents"
          :key="agent.name"
          class="agent-item hover:bg-bg-surface-hover rounded-[var(--radius-6)] transition-colors px-2 -mx-2"
        >
          <div class="agent-info">
            <div class="agent-name-row">
              <span class="status-bead" :class="probeStateClass(agent.name)" />
              <span class="agent-name">{{ agent.name }}</span>
            </div>
            <div class="agent-summary">
              {{ formatEnvCount(Object.keys(agent.env).length) }} · {{ formatServerCount(agent.mcpServers.length) }}
            </div>
          </div>
          <div class="agent-actions">
            <Button
              variant="outline"
              class="edit-btn border border-border-subtle text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary"
              @click="onStartEdit(agent)"
            >
              Edit
            </Button>
            <Button variant="destructive" class="delete-btn" @click="onDelete(agent.name)">Delete</Button>
          </div>
        </article>
        <p v-if="agents.length === 0" class="empty-state">No agents configured yet. Add one to get started.</p>
      </div>
    </div>
  </section>
</template>

<!-- audit-ok: max-style-rules -->
<style scoped>
.settings-section,
.agents-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.section-header {
  margin-bottom: 0.75rem;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.06em;
  color: var(--text-tertiary);
}

.section-header,
.agent-item,
.form-actions,
.agent-actions {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
}

.section-header h4,
.section-header p,
.agent-form h5,
.empty-state {
  margin: 0;
}

.section-header p,
.empty-state,
.agent-summary {
  color: var(--color-text-secondary);
}

.agent-form,
.agent-item {
  padding: 1rem;
  border-radius: var(--radius);
  background: var(--color-bg-elevated);
}

.form-section {
  display: grid;
  gap: 0.75rem;
  padding: 0.9rem;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: var(--color-bg-app);
}

.form-group span {
  display: block;
  margin-bottom: 0.25rem;
  font-weight: 500;
}

.form-group input {
  width: 100%;
  padding: 0.5rem;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: var(--color-bg-app);
  color: var(--color-text-primary);
}

.form-group small,
.agent-summary {
  font-size: 0.75rem;
}

.add-btn,
.save-btn,
.cancel-btn,
.edit-btn,
.delete-btn {
  padding: 0.5rem 0.9rem;
  border-radius: var(--radius);
  cursor: pointer;
}

.add-btn,
.cancel-btn {
  border: 1px solid var(--color-border-subtle);
  background: transparent;
  color: inherit;
}

.save-btn {
  border: none;
  background: var(--color-brand);
  color: var(--color-text-primary);
}

.delete-btn,
.form-error {
  color: var(--color-danger);
}

.delete-btn {
  border: 1px solid var(--color-danger);
  background: transparent;
  padding-inline: 0.6rem;
}

.agent-item {
  align-items: flex-start;
}

.agent-info {
  display: grid;
  gap: 0.5rem;
  min-width: 0;
}

.agent-name-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.agent-name {
  font-weight: 600;
}

.status-bead {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 4px;
  background: var(--text-secondary);
  flex-shrink: 0;
}

.status-bead.ready { background: var(--sage); }
.status-bead.offline { background: var(--amber); }
.status-bead.error { background: var(--terracotta); }

@media (max-width: 640px) {
  .section-header,
  .agent-item,
  .form-actions {
    flex-direction: column;
    align-items: stretch;
  }

  .agent-actions {
    justify-content: flex-start;
  }
}
</style>
