<script setup lang="ts">
import { computed, ref } from 'vue';
import SettingsMcpServersDetail from './SettingsMcpServersDetail.vue';
import SettingsMcpServersList from './SettingsMcpServersList.vue';
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
</script>

<template>
  <section class="settings-section" role="form" aria-label="MCP server settings">
    <SettingsMcpServersList
      :mcp-servers="mcpServers"
      :show-add-form="showAddForm"
      :editing-server="editingServer"
      :feedback="feedback"
      @add="startAdd"
      @edit="startEdit"
      @delete="handleDelete"
    />

    <SettingsMcpServersDetail
      :visible="showAddForm || !!editingServer"
      :editing-server="editingServer"
      :form-name="formName"
      :form-type="formType"
      :form-command="formCommand"
      :form-args="formArgs"
      :form-url="formUrl"
      :form-error="formError"
      :is-submitting="isSubmitting"
      @update:form-name="formName = $event"
      @update:form-type="formType = $event"
      @update:form-command="formCommand = $event"
      @update:form-args="formArgs = $event"
      @update:form-url="formUrl = $event"
      @submit="handleSubmit"
      @cancel="resetForm"
    />
  </section>
</template>

<style scoped>
.settings-section { display: flex; flex-direction: column; gap: 1rem; }
</style>
