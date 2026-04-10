<script setup lang="ts">
import { computed, ref } from 'vue';
import { useConfigStore } from '../stores/config';
import { addAgent, removeAgent, updateAgent } from '../lib/tauri';

interface AgentRow {
  name: string;
  command: string;
}

const configStore = useConfigStore();
const agents = computed<AgentRow[]>(() =>
  Object.entries(configStore.config.agents).map(([name, config]) => ({
    name,
    command: config.command,
  }))
);

const showAddForm = ref(false);
const editingAgent = ref<string | null>(null);
const formName = ref('');
const formCommand = ref('');
const formError = ref('');
const isSubmitting = ref(false);

function resetForm() {
  showAddForm.value = false;
  editingAgent.value = null;
  formName.value = '';
  formCommand.value = '';
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
    const nextConfig = editingAgent.value
      ? await updateAgent(formName.value, formCommand.value)
      : await addAgent(formName.value, formCommand.value);
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
      <label class="form-group">
        <span>Name</span>
        <input v-model="formName" type="text" placeholder="My Agent" :disabled="!!editingAgent" />
      </label>
      <label class="form-group">
        <span>Command</span>
        <input v-model="formCommand" type="text" placeholder="npx @github/copilot-language-server@latest --acp" />
        <small>Full shell command including arguments.</small>
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
      <article v-for="agent in agents" :key="agent.name" class="agent-item">
        <div class="agent-info">
          <div class="agent-name">{{ agent.name }}</div>
          <code class="agent-command">{{ agent.command }}</code>
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
.settings-section,.agents-list { display: flex; flex-direction: column; }
.settings-section { gap: 1rem; }
.section-header,.agent-item,.form-actions,.agent-actions { display: flex; justify-content: space-between; gap: 0.75rem; }
.section-header h3,.section-header p,.agent-form h4,.config-card h4,.config-path,.empty-state { margin: 0; }
.section-header p,.empty-state,.config-card small { color: var(--text-secondary); }
.add-btn,.save-btn,.cancel-btn,.edit-btn,.delete-btn { border-radius: 6px; cursor: pointer; }
.add-btn,.edit-btn,.cancel-btn { border: 1px solid var(--border-color); background: transparent; color: inherit; }
.add-btn,.save-btn,.cancel-btn { padding: 0.5rem 0.9rem; }
.agent-form,.agent-item,.config-card { padding: 1rem; border-radius: 8px; background: var(--bg-sidebar); }
.form-group { display: block; margin-bottom: 0.75rem; }
.form-group span { display: block; margin-bottom: 0.25rem; font-weight: 500; }
.form-group input { width: 100%; padding: 0.5rem; border: 1px solid var(--border-color); border-radius: 6px; background: var(--bg-main); color: var(--text-primary); }
.form-group small,.agent-command,.config-path { font-size: var(--text-sm); }
.form-error,.delete-btn { color: var(--bg-danger); }
.save-btn { border: none; background: var(--bg-primary); color: white; }
.delete-btn { border: 1px solid var(--bg-danger); background: transparent; padding: 0.25rem 0.6rem; }
.agent-item { align-items: center; }
.agent-info { min-width: 0; }
.agent-name { margin-bottom: 0.25rem; font-weight: 600; }
.agent-command,.config-path { display: block; word-break: break-all; font-family: var(--font-mono); }

@media (max-width: 640px) {
  .section-header,.agent-item,.form-actions { flex-direction: column; align-items: stretch; }
  .agent-actions { justify-content: flex-start; }
}
</style>
