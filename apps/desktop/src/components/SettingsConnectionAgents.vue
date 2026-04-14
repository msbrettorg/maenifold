<script setup lang="ts">
import type { AgentProbeResult } from '../lib/agent-probe-types';
import type { McpServerEntry } from '../lib/types';
import AgentProbePreview from './AgentProbePreview.vue';
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

defineProps<{
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
  isProbeLoading: (agentName: string) => boolean;
}>();

function formatServerCount(count: number) {
  return `${count} MCP server${count === 1 ? '' : 's'}`;
}

function formatEnvCount(count: number) {
  return `${count} env var${count === 1 ? '' : 's'}`;
}
</script>

<template>
  <section class="settings-section">
    <div class="section-header">
      <div>
        <h4>Agents</h4>
        <p>Add, edit, and remove ACP agent definitions.</p>
      </div>
      <button class="add-btn" :disabled="showAddForm" @click="onStartAdd">+ Add Agent</button>
    </div>

    <div v-if="showAddForm || editingAgent" class="agent-form">
      <h5>{{ editingAgent ? 'Edit Agent' : 'Add New Agent' }}</h5>

      <div class="form-section">
        <label class="form-group">
          <span>Name</span>
          <input v-model="form.name" type="text" placeholder="My Agent" :disabled="editingAgent !== null" />
        </label>
        <label class="form-group">
          <span>Command</span>
          <input v-model="form.command" type="text" placeholder="npx @github/copilot-language-server@latest --acp" />
          <small>Full shell command including arguments.</small>
        </label>
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
        <button class="save-btn" :disabled="isSubmitting" @click="onSubmit">
          {{ isSubmitting ? 'Saving...' : 'Save' }}
        </button>
        <button class="cancel-btn" @click="onResetForm">Cancel</button>
      </div>
    </div>

    <div class="agents-list">
      <article v-for="agent in agents" :key="agent.name" class="agent-item">
        <div class="agent-info">
          <div class="agent-name">{{ agent.name }}</div>
          <code class="agent-command">{{ agent.command }}</code>
          <div class="agent-summary">
            {{ formatEnvCount(Object.keys(agent.env).length) }} · {{ formatServerCount(agent.mcpServers.length) }}
          </div>
          <AgentProbePreview
            :agent-name="agent.name"
            :probe="getProbe(agent.name)"
            :loading="isProbeLoading(agent.name)"
          />
        </div>
        <div class="agent-actions">
          <button class="edit-btn" @click="onStartEdit(agent)">Edit</button>
          <button class="delete-btn" @click="onDelete(agent.name)">Delete</button>
        </div>
      </article>
      <p v-if="agents.length === 0" class="empty-state">No agents configured yet. Add one to get started.</p>
    </div>
  </section>
</template>

<style scoped>
.settings-section,
.agents-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
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
  color: var(--muted-foreground);
}

.agent-form,
.agent-item {
  padding: 1rem;
  border-radius: 8px;
  background: var(--card);
}

.form-section {
  display: grid;
  gap: 0.75rem;
  padding: 0.9rem;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--background);
}

.form-group span {
  display: block;
  margin-bottom: 0.25rem;
  font-weight: 500;
}

.form-group input {
  width: 100%;
  padding: 0.5rem;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--background);
  color: var(--foreground);
}

.form-group small,
.agent-command,
.agent-summary {
  font-size: 0.75rem;
}

.add-btn,
.save-btn,
.cancel-btn,
.edit-btn,
.delete-btn {
  padding: 0.5rem 0.9rem;
  border-radius: 6px;
  cursor: pointer;
}

.add-btn,
.edit-btn,
.cancel-btn {
  border: 1px solid var(--border);
  background: transparent;
  color: inherit;
}

.save-btn {
  border: none;
  background: var(--ring);
  color: white;
}

.delete-btn,
.form-error {
  color: var(--destructive);
}

.delete-btn {
  border: 1px solid var(--destructive);
  background: transparent;
  padding-inline: 0.6rem;
}

.agent-item {
  align-items: flex-start;
}

.agent-info {
  display: grid;
  gap: 0.75rem;
  min-width: 0;
}

.agent-name {
  font-weight: 600;
}

.agent-command {
  display: block;
  word-break: break-all;
  font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;
}

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
