<script setup lang="ts">
import { computed, ref } from 'vue';
import { DAEMON_BASE_URL } from '../lib/daemon-url';
import type { AgentsConfig, McpServerEntry } from '../lib/types';
import { useConfigStore } from '../stores/config';

type McpServerType = McpServerEntry['type'];

const configStore = useConfigStore();
const mcpServers = computed(() => configStore.config.mcpServers ?? []);

const showAddForm = ref(false);
const editingServer = ref<string | null>(null);
const formName = ref('');
const formType = ref<McpServerType>('stdio');
const formCommand = ref('');
const formArgs = ref('');
const formUrl = ref('');
const formError = ref('');
const isSubmitting = ref(false);
const feedback = ref<{ type: 'success' | 'error'; message: string } | null>(null);

async function parseConfigResponse(response: Response, action: string): Promise<AgentsConfig> {
  if (!response.ok) {
    throw new Error(`${action} failed: HTTP ${response.status}`);
  }
  return response.json() as Promise<AgentsConfig>;
}

function resetForm() {
  showAddForm.value = false;
  editingServer.value = null;
  formName.value = '';
  formType.value = 'stdio';
  formCommand.value = '';
  formArgs.value = '';
  formUrl.value = '';
  formError.value = '';
}

function startAdd() {
  resetForm();
  showAddForm.value = true;
}

function startEdit(server: McpServerEntry) {
  resetForm();
  editingServer.value = server.name;
  formName.value = server.name;
  formType.value = server.type;

  if (server.type === 'stdio') {
    formCommand.value = server.command;
    formArgs.value = (server.args ?? []).join('\n');
  } else {
    formUrl.value = server.url;
  }
}

function buildEntry(): McpServerEntry {
  if (formType.value === 'stdio') {
    const args = formArgs.value
      .split('\n')
      .map((arg) => arg.trim())
      .filter(Boolean);

    return {
      type: 'stdio',
      name: formName.value.trim(),
      command: formCommand.value.trim(),
      ...(args.length > 0 ? { args } : {}),
    };
  }

  return {
    type: 'http',
    name: formName.value.trim(),
    url: formUrl.value.trim(),
  };
}

async function handleSubmit() {
  formError.value = '';
  feedback.value = null;

  if (!formName.value.trim()) return void (formError.value = 'Name is required');
  if (/^\d+$/.test(formName.value.trim())) {
    formError.value = 'MCP server name cannot be purely numeric';
    return;
  }
  if (!editingServer.value && mcpServers.value.some((server) => server.name === formName.value.trim())) {
    formError.value = 'An MCP server with this name already exists';
    return;
  }
  if (formType.value === 'stdio' && !formCommand.value.trim()) {
    formError.value = 'Command is required for stdio servers';
    return;
  }
  if (formType.value === 'http' && !formUrl.value.trim()) {
    formError.value = 'URL is required for HTTP servers';
    return;
  }

  isSubmitting.value = true;

  try {
    const entry = buildEntry();
    const response = editingServer.value
      ? await fetch(`${DAEMON_BASE_URL}/api/config/mcp-servers/${encodeURIComponent(editingServer.value)}`, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ entry }),
        })
      : await fetch(`${DAEMON_BASE_URL}/api/config/mcp-servers`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ entry }),
        });
    configStore.updateFromEvent(await parseConfigResponse(response, editingServer.value ? 'Update MCP server' : 'Add MCP server'));
    feedback.value = {
      type: 'success',
      message: editingServer.value ? `Updated MCP server "${entry.name}".` : `Added MCP server "${entry.name}".`,
    };
    resetForm();
  } catch (error) {
    formError.value = error instanceof Error ? error.message : String(error);
    feedback.value = { type: 'error', message: formError.value };
  } finally {
    isSubmitting.value = false;
  }
}

async function handleDelete(name: string) {
  if (!confirm(`Delete MCP server "${name}"?`)) return;

  try {
    feedback.value = null;
    const response = await fetch(`${DAEMON_BASE_URL}/api/config/mcp-servers/${encodeURIComponent(name)}`, {
      method: 'DELETE',
    });
    configStore.updateFromEvent(await parseConfigResponse(response, 'Delete MCP server'));
    await configStore.loadConfig();
    if (editingServer.value === name) {
      resetForm();
    }
    feedback.value = { type: 'success', message: `Deleted MCP server "${name}".` };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    feedback.value = { type: 'error', message };
    console.error('Failed to delete MCP server:', error);
  }
}

function getServerDetails(server: McpServerEntry): string {
  return server.type === 'stdio'
    ? [server.command, ...(server.args ?? [])].join(' ')
    : server.url;
}

function getServerMeta(server: McpServerEntry): string | null {
  if (server.type !== 'stdio' || !server.args?.length) return null;
  return `${server.args.length} arg${server.args.length === 1 ? '' : 's'}`;
}
</script>

<template>
  <section class="settings-section">
    <div class="section-header">
      <div>
        <h3>MCP servers</h3>
        <p>Manage the shared global MCP server entries available to every configured agent.</p>
      </div>
      <button class="add-btn" :disabled="showAddForm" @click="startAdd">+ Add MCP Server</button>
    </div>

    <p v-if="feedback" class="feedback" :class="feedback.type">{{ feedback.message }}</p>

    <div v-if="showAddForm || editingServer" class="agent-form">
      <h4>{{ editingServer ? 'Edit MCP Server' : 'Add New MCP Server' }}</h4>

      <label class="form-group">
        <span>Name</span>
        <input v-model="formName" type="text" placeholder="filesystem" :disabled="!!editingServer" />
      </label>

      <label class="form-group">
        <span>Type</span>
        <select v-model="formType">
          <option value="stdio">stdio</option>
          <option value="http">http</option>
        </select>
      </label>

      <template v-if="formType === 'stdio'">
        <label class="form-group">
          <span>Command</span>
          <input v-model="formCommand" type="text" placeholder="npx @modelcontextprotocol/server-filesystem" />
        </label>

        <label class="form-group">
          <span>Arguments</span>
          <textarea
            v-model="formArgs"
            rows="4"
            placeholder="/path/to/workspace&#10;--read-only"
          />
          <small>Enter one argument per line.</small>
        </label>
      </template>

      <label v-else class="form-group">
        <span>URL</span>
        <input v-model="formUrl" type="url" placeholder="http://localhost:8080/mcp" />
      </label>

      <p v-if="formError" class="form-error">{{ formError }}</p>

      <div class="form-actions">
        <button class="save-btn" :disabled="isSubmitting" @click="handleSubmit">
          {{ isSubmitting ? 'Saving...' : 'Save' }}
        </button>
        <button class="cancel-btn" @click="resetForm">Cancel</button>
      </div>
    </div>

    <div class="agents-list">
      <article v-for="server in mcpServers" :key="server.name" class="agent-item">
        <div class="agent-info">
          <div class="agent-name-row">
            <div class="agent-name">{{ server.name }}</div>
            <span class="type-badge">{{ server.type }}</span>
          </div>
          <code class="agent-command">{{ getServerDetails(server) }}</code>
          <small v-if="getServerMeta(server)" class="server-meta">{{ getServerMeta(server) }}</small>
        </div>
        <div class="agent-actions">
          <button class="edit-btn" @click="startEdit(server)">Edit</button>
          <button class="delete-btn" @click="handleDelete(server.name)">Delete</button>
        </div>
      </article>

      <p v-if="mcpServers.length === 0" class="empty-state">
        No MCP servers configured yet. Add one to share it across all agents.
      </p>
    </div>
  </section>
</template>

<style scoped>
.settings-section,.agents-list { display: flex; flex-direction: column; }
.settings-section { gap: 1rem; }
.section-header,.agent-item,.form-actions,.agent-actions,.agent-name-row { display: flex; gap: 0.75rem; }
.section-header,.agent-item,.form-actions,.agent-actions { justify-content: space-between; }
.section-header h3,.section-header p,.agent-form h4,.empty-state,.server-meta { margin: 0; }
.section-header p,.empty-state,.server-meta { color: var(--text-secondary); }
.add-btn,.save-btn,.cancel-btn,.edit-btn,.delete-btn { border-radius: 6px; cursor: pointer; }
.add-btn,.edit-btn,.cancel-btn { border: 1px solid var(--border-default); background: transparent; color: inherit; }
.add-btn,.save-btn,.cancel-btn { padding: 0.5rem 0.9rem; }
.agent-form,.agent-item { padding: 1rem; border-radius: 8px; background: var(--bg-panel); }
.form-group { display: block; margin-bottom: 0.75rem; }
.form-group span { display: block; margin-bottom: 0.25rem; font-weight: var(--weight-display); }
.form-group input,.form-group select,.form-group textarea {
  width: 100%;
  padding: 0.5rem;
  border: 1px solid var(--border-default);
  border-radius: 6px;
  background: var(--bg-main);
  color: var(--text-primary);
}
.form-group textarea { resize: vertical; }
.form-group small,.agent-command,.server-meta { font-size: var(--text-caption); }
.form-error,.delete-btn { color: var(--border-error); }
.feedback {
  margin: 0;
  padding: 0.75rem 1rem;
  border: 1px solid var(--border-default);
  border-radius: 8px;
  font-size: var(--text-caption);
}
.feedback.success {
  border-color: color-mix(in srgb, var(--color-status-completed) 45%, transparent);
  background: color-mix(in srgb, var(--color-status-completed) 14%, var(--bg-panel));
  color: var(--text-primary);
}
.feedback.error {
  border-color: color-mix(in srgb, var(--border-error) 45%, transparent);
  background: color-mix(in srgb, var(--border-error) 12%, var(--bg-panel));
  color: var(--border-error);
}
.save-btn { border: none; background: var(--border-focus); color: white; }
.delete-btn { border: 1px solid var(--border-error); background: transparent; padding: 0.25rem 0.6rem; }
.agent-item { align-items: center; }
.agent-info { min-width: 0; }
.agent-name-row { align-items: center; justify-content: flex-start; }
.agent-name { font-weight: var(--weight-title); }
.agent-command {
  display: block;
  margin-top: 0.25rem;
  word-break: break-all;
  font-family: var(--font-code);
}
.type-badge {
  display: inline-flex;
  align-items: center;
  padding: 0.15rem 0.45rem;
  border-radius: 999px;
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: var(--text-caption);
  text-transform: lowercase;
}

@media (max-width: 640px) {
  .section-header,.agent-item,.form-actions { flex-direction: column; align-items: stretch; }
  .agent-actions { justify-content: flex-start; }
}
</style>
