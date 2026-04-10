<script setup lang="ts">
import { computed, ref } from 'vue';
import type { AgentConfig, McpServerEntry } from '../lib/types';
import { useConfigStore } from '../stores/config';
import { addAgent, removeAgent, updateAgent } from '../lib/tauri';

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

const configStore = useConfigStore();
const agents = computed<AgentRow[]>(() =>
  Object.entries(configStore.config.agents).map(([name, config]) => ({
    name,
    command: config.command,
    env: config.env ?? {},
    mcpServers: config.mcpServers ?? [],
  }))
);

const showAddForm = ref(false);
const editingAgent = ref<string | null>(null);
const formName = ref('');
const formCommand = ref('');
const formEnv = ref<EnvVarRow[]>([]);
const formMcpServers = ref<McpServerFormRow[]>([]);
const formError = ref('');
const isSubmitting = ref(false);

function formatServerCount(count: number) {
  return `${count} MCP server${count === 1 ? '' : 's'}`;
}

function formatEnvCount(count: number) {
  return `${count} env var${count === 1 ? '' : 's'}`;
}

function toEnvRows(env: Record<string, string> = {}): EnvVarRow[] {
  return Object.entries(env).map(([key, value]) => ({ key, value }));
}

function toMcpServerRows(mcpServers: McpServerEntry[] = []): McpServerFormRow[] {
  return mcpServers.map((server) =>
    server.type === 'http'
      ? {
          type: 'http',
          name: server.name,
          commandOrUrl: server.url,
          args: '',
        }
      : {
          type: 'stdio',
          name: server.name,
          commandOrUrl: server.command,
          args: (server.args ?? []).join(' '),
        }
  );
}

function buildMcpServers(rows: McpServerFormRow[]): McpServerEntry[] {
  return rows
    .filter((row) => row.name.trim() && row.commandOrUrl.trim())
    .map((row) =>
      row.type === 'http'
        ? {
            type: 'http' as const,
            name: row.name.trim(),
            url: row.commandOrUrl.trim(),
          }
        : {
            type: 'stdio' as const,
            name: row.name.trim(),
            command: row.commandOrUrl.trim(),
            ...(row.args.trim()
              ? { args: row.args.split(/\s+/).filter(Boolean) }
              : {}),
          }
    );
}

function resetForm() {
  showAddForm.value = false;
  editingAgent.value = null;
  formName.value = '';
  formCommand.value = '';
  formEnv.value = [];
  formMcpServers.value = [];
  formError.value = '';
}

function startAdd() {
  resetForm();
  showAddForm.value = true;
}

function startEdit(agent: AgentRow) {
  resetForm();
  editingAgent.value = agent.name;
  formName.value = agent.name;
  formCommand.value = agent.command;
  formEnv.value = toEnvRows(agent.env);
  formMcpServers.value = toMcpServerRows(agent.mcpServers);
}

function addEnvRow() {
  formEnv.value.push({ key: '', value: '' });
}

function removeEnvRow(index: number) {
  formEnv.value.splice(index, 1);
}

function addMcpServerRow() {
  formMcpServers.value.push({
    type: 'stdio',
    name: '',
    commandOrUrl: '',
    args: '',
  });
}

function removeMcpServerRow(index: number) {
  formMcpServers.value.splice(index, 1);
}

async function handleSubmit() {
  formError.value = '';

  if (!formName.value.trim()) return void (formError.value = 'Name is required');
  if (!formCommand.value.trim()) return void (formError.value = 'Command is required');
  if (/^\d+$/.test(formName.value)) {
    formError.value = 'Agent name cannot be purely numeric';
    return;
  }
  if (!editingAgent.value && configStore.config.agents[formName.value]) {
    formError.value = 'An agent with this name already exists';
    return;
  }

  isSubmitting.value = true;

  try {
    const config: AgentConfig = {
      command: formCommand.value,
      env: Object.fromEntries(
        formEnv.value
          .filter((entry) => entry.key.trim())
          .map((entry) => [entry.key.trim(), entry.value])
      ),
      mcpServers: buildMcpServers(formMcpServers.value),
    };
    const nextConfig = editingAgent.value
      ? await updateAgent(formName.value, config)
      : await addAgent(formName.value, config);
    configStore.updateFromEvent(nextConfig);
    resetForm();
  } catch (error) {
    formError.value = error instanceof Error ? error.message : String(error);
  } finally {
    isSubmitting.value = false;
  }
}

async function handleDelete(name: string) {
  if (!confirm(`Delete agent "${name}"?`)) return;

  try {
    configStore.updateFromEvent(await removeAgent(name));
  } catch (error) {
    console.error('Failed to delete agent:', error);
  }
}
</script>

<template>
  <section class="settings-section">
    <div class="section-header">
      <div>
        <h3>Connection settings</h3>
        <p>Manage the configured ACP agents available in the application.</p>
      </div>
      <button class="add-btn" :disabled="showAddForm" @click="startAdd">+ Add Agent</button>
    </div>

    <div v-if="showAddForm || editingAgent" class="agent-form">
      <h4>{{ editingAgent ? 'Edit Agent' : 'Add New Agent' }}</h4>

      <div class="form-section">
        <h5>Basic settings</h5>
        <label class="form-group">
          <span>Name</span>
          <input v-model="formName" type="text" placeholder="My Agent" :disabled="!!editingAgent" />
        </label>
        <label class="form-group">
          <span>Command</span>
          <input
            v-model="formCommand"
            type="text"
            placeholder="npx @github/copilot-language-server@latest --acp"
          />
          <small>Full shell command including arguments.</small>
        </label>
      </div>

      <details class="form-section details-section" open>
        <summary>
          <div>
            <h5>Environment variables</h5>
            <p>Optional key/value pairs passed to this agent process.</p>
          </div>
          <span class="summary-count">{{ formatEnvCount(formEnv.length) }}</span>
        </summary>

        <div class="section-body">
          <div v-if="formEnv.length === 0" class="empty-detail">No environment variables configured.</div>
          <div v-for="(entry, index) in formEnv" :key="`env-${index}`" class="kv-row">
            <input v-model="entry.key" type="text" placeholder="KEY" />
            <input v-model="entry.value" type="text" placeholder="value" />
            <button class="remove-btn" type="button" aria-label="Remove variable" @click="removeEnvRow(index)">
              ×
            </button>
          </div>
          <button class="add-inline-btn" type="button" @click="addEnvRow">+ Add Variable</button>
        </div>
      </details>

      <details class="form-section details-section" open>
        <summary>
          <div>
            <h5>Per-agent MCP servers</h5>
            <p>Optional MCP servers available only to this agent.</p>
          </div>
          <span class="summary-count">{{ formatServerCount(formMcpServers.length) }}</span>
        </summary>

        <div class="section-body">
          <div v-if="formMcpServers.length === 0" class="empty-detail">No per-agent MCP servers configured.</div>
          <div v-for="(server, index) in formMcpServers" :key="`mcp-${index}`" class="server-editor">
            <div class="server-editor-header">
              <strong>Server {{ index + 1 }}</strong>
              <button class="remove-btn" type="button" aria-label="Remove MCP server" @click="removeMcpServerRow(index)">
                ×
              </button>
            </div>
            <div class="server-grid">
              <label class="form-group">
                <span>Type</span>
                <select v-model="server.type">
                  <option value="stdio">stdio</option>
                  <option value="http">http</option>
                </select>
              </label>
              <label class="form-group">
                <span>Name</span>
                <input v-model="server.name" type="text" placeholder="server-name" />
              </label>
              <label class="form-group server-primary-field">
                <span>{{ server.type === 'http' ? 'URL' : 'Command' }}</span>
                <input
                  v-model="server.commandOrUrl"
                  type="text"
                  :placeholder="server.type === 'http' ? 'https://example.test/mcp' : 'npx @scope/server'"
                />
              </label>
              <label v-if="server.type === 'stdio'" class="form-group server-primary-field">
                <span>Args</span>
                <input v-model="server.args" type="text" placeholder="--flag value" />
                <small>Space-separated arguments.</small>
              </label>
            </div>
          </div>
          <button class="add-inline-btn" type="button" @click="addMcpServerRow">+ Add MCP Server</button>
        </div>
      </details>

      <p v-if="formError" class="form-error">{{ formError }}</p>
      <div class="form-actions">
        <button class="save-btn" :disabled="isSubmitting" @click="handleSubmit">
          {{ isSubmitting ? 'Saving...' : 'Save' }}
        </button>
        <button class="cancel-btn" @click="resetForm">Cancel</button>
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
        </div>
        <div class="agent-actions">
          <button class="edit-btn" @click="startEdit(agent)">Edit</button>
          <button class="delete-btn" @click="handleDelete(agent.name)">Delete</button>
        </div>
      </article>
      <p v-if="agents.length === 0" class="empty-state">No agents configured yet. Add one to get started.</p>
    </div>

    <div class="config-card">
      <h4>Config file</h4>
      <p class="config-path">{{ configStore.configPath }}</p>
      <small>Changes to this file are automatically reloaded.</small>
    </div>
  </section>
</template>

<style scoped>
.settings-section,
.agents-list,
.section-body,
.server-grid,
.server-editor {
  display: flex;
  flex-direction: column;
}

.settings-section {
  gap: 1rem;
}

.section-header,
.agent-item,
.form-actions,
.agent-actions,
.kv-row,
.server-editor-header,
.details-section summary {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
}

.section-header h3,
.section-header p,
.agent-form h4,
.agent-form h5,
.config-card h4,
.config-path,
.empty-state,
.details-section summary p {
  margin: 0;
}

.section-header p,
.empty-state,
.config-card small,
.details-section summary p,
.agent-summary,
.empty-detail {
  color: var(--text-secondary);
}

.add-btn,
.save-btn,
.cancel-btn,
.edit-btn,
.delete-btn,
.add-inline-btn,
.remove-btn {
  border-radius: 6px;
  cursor: pointer;
}

.add-btn,
.edit-btn,
.cancel-btn,
.add-inline-btn,
.remove-btn {
  border: 1px solid var(--border-color);
  background: transparent;
  color: inherit;
}

.add-btn,
.save-btn,
.cancel-btn,
.add-inline-btn {
  padding: 0.5rem 0.9rem;
}

.agent-form,
.agent-item,
.config-card {
  padding: 1rem;
  border-radius: 8px;
  background: var(--bg-sidebar);
}

.agent-form,
.server-grid,
.section-body,
.server-editor {
  gap: 0.75rem;
}

.form-section {
  padding: 0.9rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-main);
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

.summary-count {
  align-self: flex-start;
  padding: 0.15rem 0.5rem;
  border-radius: 999px;
  background: var(--bg-sidebar);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  white-space: nowrap;
}

.form-group {
  display: block;
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
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-main);
  color: var(--text-primary);
}

.form-group small,
.agent-command,
.config-path,
.agent-summary,
.summary-count {
  font-size: var(--text-sm);
}

.form-error,
.delete-btn {
  color: var(--bg-danger);
}

.save-btn {
  border: none;
  background: var(--bg-primary);
  color: white;
}

.delete-btn {
  border: 1px solid var(--bg-danger);
  background: transparent;
  padding: 0.25rem 0.6rem;
}

.remove-btn {
  width: 2rem;
  height: 2rem;
  padding: 0;
  align-self: end;
  font-size: 1.1rem;
  line-height: 1;
}

.agent-item {
  align-items: center;
}

.agent-info {
  min-width: 0;
}

.agent-name {
  margin-bottom: 0.25rem;
  font-weight: 600;
}

.agent-command,
.config-path {
  display: block;
  word-break: break-all;
  font-family: var(--font-mono);
}

.agent-summary {
  margin-top: 0.35rem;
}

.kv-row {
  align-items: end;
}

.kv-row input:first-child {
  max-width: 14rem;
}

.server-editor {
  padding: 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-sidebar);
}

.server-editor-header {
  align-items: center;
}

.server-primary-field {
  width: 100%;
}

.empty-detail {
  font-size: var(--text-sm);
}

@media (max-width: 640px) {
  .section-header,
  .agent-item,
  .form-actions,
  .kv-row,
  .details-section summary {
    flex-direction: column;
    align-items: stretch;
  }

  .agent-actions,
  .server-editor-header {
    justify-content: flex-start;
  }

  .kv-row input:first-child {
    max-width: none;
  }

  .remove-btn {
    align-self: flex-start;
  }
}
</style>
