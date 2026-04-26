<script setup lang="ts">
import { computed, ref } from 'vue';
import SettingsMcpServersDetail from './SettingsMcpServersDetail.vue';
import SettingsMcpServersList from './SettingsMcpServersList.vue';
import type { AgentsConfig, McpServerEntry } from '../lib/types';
import { useMcpClient } from '../composables/useMcpClient';
import { useConfigStore } from '../stores/config';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';

type McpServerType = McpServerEntry['type'];

const configStore = useConfigStore();
const { callTool, connect, connected } = useMcpClient();
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
    if (!connected.value) {
      await connect();
    }
    const updatedConfig = editingServer.value
      ? await callTool<AgentsConfig>('UpdateMcpServer', { name: editingServer.value, entry })
      : await callTool<AgentsConfig>('AddMcpServer', { entry });
    configStore.updateFromEvent(updatedConfig);
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
  pendingDeleteServer.value = name;
}

const pendingDeleteServer = ref<string | null>(null);
const deleteDialogOpen = computed({
  get: () => pendingDeleteServer.value !== null,
  set: (open: boolean) => {
    if (!open) pendingDeleteServer.value = null;
  },
});

async function confirmDeleteServer() {
  const name = pendingDeleteServer.value;
  pendingDeleteServer.value = null;
  if (!name) return;

  try {
    feedback.value = null;
    if (!connected.value) {
      await connect();
    }
    configStore.updateFromEvent(await callTool<AgentsConfig>('DeleteMcpServer', { name }));
    await configStore.loadConfig();
    if (editingServer.value === name) {
      resetForm();
    }
    feedback.value = { type: 'success', message: `Deleted MCP server "${name}".` };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    feedback.value = { type: 'error', message };
  }
}
</script>

<template>
  <section class="flex flex-col gap-4" role="form" aria-label="MCP server settings">
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

    <AlertDialog v-model:open="deleteDialogOpen">
      <AlertDialogContent class="sm:max-w-[28rem]">
        <AlertDialogHeader>
          <AlertDialogTitle>Delete MCP server "{{ pendingDeleteServer }}"?</AlertDialogTitle>
          <AlertDialogDescription>
            This will remove the MCP server configuration. Agents using this
            server will no longer have access to it. This action cannot be
            undone.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            @click="confirmDeleteServer"
          >
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </section>
</template>
