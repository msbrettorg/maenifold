<script setup lang="ts">
import type { AgentProbeResult } from '../lib/agent-probe-types';
import type { McpServerEntry } from '../lib/types';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
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

function probeBeadClass(name: string): string {
  const probe = props.getProbe(name);
  if (!probe) return 'bg-text-secondary';
  if (probe.state === 'ready') return 'bg-success';
  if (probe.state === 'offline') return 'bg-warning';
  return 'bg-danger';
}

function formatServerCount(count: number) {
  return `${count} MCP server${count === 1 ? '' : 's'}`;
}

function formatEnvCount(count: number) {
  return `${count} env var${count === 1 ? '' : 's'}`;
}
</script>

<template>
  <section class="flex flex-col gap-4">
    <Card class="rounded-[8px] py-0 shadow-none ring-0 border border-border-card bg-bg-elevated">
      <CardContent class="p-4">
      <div class="flex justify-between gap-3 mb-3 text-[11px] font-semibold tracking-[0.06em] text-text-tertiary max-sm:flex-col max-sm:items-stretch">
        <div>
          <h4 class="m-0 text-sm font-medium text-text-secondary">Agents</h4>
          <p class="m-0 text-text-secondary">Add, edit, and remove ACP agent definitions.</p>
        </div>
        <Button variant="outline" class="px-[0.9rem] py-2 rounded cursor-pointer border border-border-subtle bg-transparent text-inherit" :disabled="showAddForm" @click="onStartAdd">+ Add agent</Button>
      </div>

      <div v-if="showAddForm || editingAgent" class="p-4 rounded bg-bg-elevated">
        <h5 class="m-0">{{ editingAgent ? 'Edit agent' : 'Add new agent' }}</h5>

        <div class="grid gap-3 p-[0.9rem] border border-border-subtle rounded bg-bg-app">
          <div class="flex flex-col gap-[0.45rem]">
            <Label>Name</Label>
            <Input v-model="form.name" type="text" placeholder="My Agent" :disabled="editingAgent !== null" />
          </div>
          <div class="flex flex-col gap-[0.45rem]">
            <Label>Command</Label>
            <Input v-model="form.command" type="text" placeholder="npx @github/copilot-language-server@latest --acp" />
            <small class="text-[0.75rem]">Full shell command including arguments.</small>
          </div>
        </div>

        <SettingsConnectionKeys
          :form="form"
          :on-add-env-row="onAddEnvRow"
          :on-add-mcp-server-row="onAddMcpServerRow"
          :on-remove-env-row="onRemoveEnvRow"
          :on-remove-mcp-server-row="onRemoveMcpServerRow"
        />

        <p v-if="form.error" class="text-danger">{{ form.error }}</p>
        <div class="flex justify-between gap-3 max-sm:flex-col max-sm:items-stretch">
          <Button class="px-[0.9rem] py-2 rounded cursor-pointer border-none bg-brand text-text-primary" :disabled="isSubmitting" @click="onSubmit">
            {{ isSubmitting ? 'Saving...' : 'Save' }}
          </Button>
          <Button variant="outline" class="px-[0.9rem] py-2 rounded cursor-pointer border border-border-subtle bg-transparent text-inherit" @click="onResetForm">Cancel</Button>
        </div>
      </div>

      <div class="flex flex-col gap-4">
        <article
          v-for="agent in agents"
          :key="agent.name"
          class="flex justify-between gap-3 items-start p-4 rounded bg-bg-elevated hover:bg-bg-surface-hover transition-colors px-2 -mx-2 max-sm:flex-col max-sm:items-stretch"
        >
          <div class="grid gap-2 min-w-0">
            <div class="flex items-center gap-2">
              <span class="inline-block w-[6px] h-[6px] rounded shrink-0" :class="probeBeadClass(agent.name)" />
              <span class="font-semibold">{{ agent.name }}</span>
            </div>
            <div class="text-text-secondary text-[0.75rem]">
              {{ formatEnvCount(Object.keys(agent.env).length) }} · {{ formatServerCount(agent.mcpServers.length) }}
            </div>
          </div>
          <div class="flex justify-between gap-3 max-sm:justify-start">
            <Button
              variant="outline"
              class="px-[0.9rem] py-2 rounded cursor-pointer border border-border-subtle text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary"
              @click="onStartEdit(agent)"
            >
              Edit
            </Button>
            <Button variant="destructive" class="py-2 px-[0.6rem] rounded cursor-pointer border border-danger bg-transparent text-danger" @click="onDelete(agent.name)">Delete</Button>
          </div>
        </article>
        <p v-if="agents.length === 0" class="m-0 text-text-secondary">No agents configured yet. Add one to get started.</p>
      </div>
      </CardContent>
    </Card>
  </section>
</template>
