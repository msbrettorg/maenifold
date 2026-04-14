<script setup lang="ts">
import { computed, reactive, ref } from 'vue';
import type { AgentConfig, AgentsConfig, McpServerEntry } from '../lib/types';
import { DAEMON_BASE_URL } from '../lib/daemon-url';
import { useConfigStore } from '../stores/config';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import SettingsConnectionAgents from './SettingsConnectionAgents.vue';
import SettingsConnectionDaemon from './SettingsConnectionDaemon.vue';

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
const capabilitiesStore = useAgentCapabilitiesStore();

const agents = computed<AgentRow[]>(() =>
  Object.entries(configStore.config.agents).map(([name, config]) => ({
    name,
    command: config.command,
    env: config.env ?? {},
    mcpServers: config.mcpServers ?? [],
  })),
);

const showAddForm = ref(false);
const editingAgent = ref<string | null>(null);
const isSubmitting = ref(false);
const form = reactive({
  name: '',
  command: '',
  env: [] as EnvVarRow[],
  mcpServers: [] as McpServerFormRow[],
  error: '',
});

function getProbe(agentName: string) {
  return capabilitiesStore.getProbe(agentName);
}

function isProbeLoading(agentName: string) {
  return capabilitiesStore.probeStatus === 'probing' && !getProbe(agentName);
}

function toEnvRows(env: Record<string, string> = {}): EnvVarRow[] {
  return Object.entries(env).map(([key, value]) => ({ key, value }));
}

function toMcpServerRows(mcpServers: McpServerEntry[] = []): McpServerFormRow[] {
  return mcpServers.map((server) =>
    server.type === 'http'
      ? { type: 'http', name: server.name, commandOrUrl: server.url, args: '' }
      : {
          type: 'stdio',
          name: server.name,
          commandOrUrl: server.command,
          args: (server.args ?? []).join(' '),
        },
  );
}

function buildMcpServers(rows: McpServerFormRow[]): McpServerEntry[] {
  return rows
    .filter((row) => row.name.trim() && row.commandOrUrl.trim())
    .map((row) =>
      row.type === 'http'
        ? { type: 'http' as const, name: row.name.trim(), url: row.commandOrUrl.trim() }
        : {
            type: 'stdio' as const,
            name: row.name.trim(),
            command: row.commandOrUrl.trim(),
            ...(row.args.trim() ? { args: row.args.split(/\s+/).filter(Boolean) } : {}),
          },
    );
}

async function parseConfigResponse(response: Response, action: string): Promise<AgentsConfig> {
  if (!response.ok) {
    throw new Error(`${action} failed: HTTP ${response.status}`);
  }
  return response.json() as Promise<AgentsConfig>;
}

function resetForm() {
  showAddForm.value = false;
  editingAgent.value = null;
  form.name = '';
  form.command = '';
  form.env = [];
  form.mcpServers = [];
  form.error = '';
}

function startAdd() {
  resetForm();
  showAddForm.value = true;
}

function startEdit(agent: AgentRow) {
  resetForm();
  editingAgent.value = agent.name;
  form.name = agent.name;
  form.command = agent.command;
  form.env = toEnvRows(agent.env);
  form.mcpServers = toMcpServerRows(agent.mcpServers);
}

function addEnvRow() {
  form.env.push({ key: '', value: '' });
}

function removeEnvRow(index: number) {
  form.env.splice(index, 1);
}

function addMcpServerRow() {
  form.mcpServers.push({ type: 'stdio', name: '', commandOrUrl: '', args: '' });
}

function removeMcpServerRow(index: number) {
  form.mcpServers.splice(index, 1);
}

async function handleSubmit() {
  form.error = '';

  if (!form.name.trim()) return void (form.error = 'Name is required');
  if (!form.command.trim()) return void (form.error = 'Command is required');
  if (/^\d+$/.test(form.name)) {
    form.error = 'Agent name cannot be purely numeric';
    return;
  }
  if (!editingAgent.value && configStore.config.agents[form.name]) {
    form.error = 'An agent with this name already exists';
    return;
  }

  isSubmitting.value = true;

  try {
    const config: AgentConfig = {
      command: form.command,
      env: Object.fromEntries(
        form.env.filter((entry) => entry.key.trim()).map((entry) => [entry.key.trim(), entry.value]),
      ),
      mcpServers: buildMcpServers(form.mcpServers),
    };
    const response = editingAgent.value
      ? await fetch(`${DAEMON_BASE_URL}/api/config/agents/${encodeURIComponent(form.name)}`, {
          method: 'PUT',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ config }),
        })
      : await fetch(`${DAEMON_BASE_URL}/api/config/agents`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ name: form.name, config }),
        });
    configStore.updateFromEvent(
      await parseConfigResponse(response, editingAgent.value ? 'Update agent' : 'Add agent'),
    );
    resetForm();
  } catch (error) {
    form.error = error instanceof Error ? error.message : String(error);
  } finally {
    isSubmitting.value = false;
  }
}

async function handleDelete(name: string) {
  if (!confirm(`Delete agent "${name}"?`)) return;

  try {
    const response = await fetch(`${DAEMON_BASE_URL}/api/config/agents/${encodeURIComponent(name)}`, {
      method: 'DELETE',
    });
    configStore.updateFromEvent(await parseConfigResponse(response, 'Delete agent'));
    if (editingAgent.value === name) {
      resetForm();
    }
  } catch (error) {
    console.error('Failed to delete agent:', error);
  }
}

async function refreshHealth() {
  await capabilitiesStore.refreshAll();
}
</script>

<template>
  <section class="settings-section">
    <div class="section-header">
      <div>
        <h3>Connection settings</h3>
        <p>Manage the configured ACP agents available in the application.</p>
      </div>
    </div>

    <SettingsConnectionDaemon
      :agents="agents"
      :config-path="configStore.configPath"
      :daemon-url="DAEMON_BASE_URL"
      :is-refreshing="capabilitiesStore.probeStatus === 'probing'"
      :get-probe="getProbe"
      :on-refresh="refreshHealth"
    />

    <SettingsConnectionAgents
      :agents="agents"
      :editing-agent="editingAgent"
      :form="form"
      :get-probe="getProbe"
      :is-probe-loading="isProbeLoading"
      :is-submitting="isSubmitting"
      :on-add-env-row="addEnvRow"
      :on-add-mcp-server-row="addMcpServerRow"
      :on-delete="handleDelete"
      :on-remove-env-row="removeEnvRow"
      :on-remove-mcp-server-row="removeMcpServerRow"
      :on-reset-form="resetForm"
      :on-start-add="startAdd"
      :on-start-edit="startEdit"
      :on-submit="handleSubmit"
      :show-add-form="showAddForm"
    />
  </section>
</template>

<style scoped>
.settings-section {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.section-header h3,
.section-header p {
  margin: 0;
}

.section-header p {
  color: var(--muted-foreground);
}
</style>
