<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { Plus, Trash2 } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { useUnsavedChangesGuard } from '@/composables/useUnsavedChangesGuard';
import type { McpServerEntry } from '@/lib/types';

type Transport = 'stdio' | 'http';
type EditableServer = McpServerEntry & {
  env?: Record<string, string>;
  timeout?: number;
};

interface EnvRow {
  id: number;
  key: string;
  value: string;
  show: boolean;
}

const props = defineProps<{
  open: boolean;
  server: EditableServer | null;
  existingNames: string[];
}>();

const emit = defineEmits<{
  'update:open': [open: boolean];
  save: [server: EditableServer];
}>();

const guard = useUnsavedChangesGuard({ title: 'Discard MCP server changes?' });
const form = reactive({
  name: '',
  transport: 'stdio' as Transport,
  command: '',
  args: '',
  timeout: '',
  url: '',
});
const envRows = ref<EnvRow[]>([]);
const errors = reactive<Record<string, string>>({});
const isDirty = ref(false);
let nextEnvId = 1;
let suppressDirty = false;

const title = computed(() => (props.server ? 'Edit MCP server' : 'Add MCP server'));
const description = computed(() =>
  props.server
    ? 'Update this server in maenifold.json.'
    : 'Add a Stdio or Http MCP server to maenifold.json.',
);

function entryValue(entry: EditableServer, key: string): unknown {
  return (entry as unknown as Record<string, unknown>)[key];
}

function resetForm(): void {
  suppressDirty = true;
  const server = props.server;
  form.name = server?.name ?? '';
  form.transport = server?.type ?? 'stdio';
  form.command = server?.type === 'stdio' ? server.command : '';
  form.args = server?.type === 'stdio' ? (server.args ?? []).join(' ') : '';
  const timeout = server ? entryValue(server, 'timeout') : undefined;
  form.timeout = typeof timeout === 'number'
    ? String(timeout)
    : '';
  form.url = server?.type === 'http' ? server.url : '';
  const env = entryValue((server ?? {}) as EditableServer, 'env');
  envRows.value = isEnvRecord(env)
    ? Object.entries(env).map(([key, value]) => ({ id: nextEnvId++, key, value, show: false }))
    : [];
  clearErrors();
  isDirty.value = false;
  guard.markClean();
  suppressDirty = false;
}

function isEnvRecord(value: unknown): value is Record<string, string> {
  return typeof value === 'object'
    && value !== null
    && !Array.isArray(value)
    && Object.values(value).every((item) => typeof item === 'string');
}

function clearErrors(): void {
  for (const key of Object.keys(errors)) {
    delete errors[key];
  }
}

function markDirty(): void {
  if (suppressDirty) return;
  isDirty.value = true;
  guard.markDirty();
}

function parseArgs(value: string): string[] {
  const trimmed = value.trim();
  if (!trimmed) return [];
  if (trimmed.startsWith('[')) {
    const parsed: unknown = JSON.parse(trimmed);
    if (!Array.isArray(parsed) || !parsed.every((item) => typeof item === 'string')) {
      throw new Error('Args JSON must be an array of strings.');
    }
    return parsed;
  }
  return trimmed.split(/\s+/).filter(Boolean);
}

function validate(): boolean {
  clearErrors();
  const name = form.name.trim();
  if (!name) errors.name = 'Name is required.';
  const duplicate = props.existingNames.some((candidate) =>
    candidate !== props.server?.name && candidate.toLowerCase() === name.toLowerCase()
  );
  if (duplicate) errors.name = 'Name must be unique.';
  if (form.transport === 'stdio' && !form.command.trim()) {
    errors.command = 'Command is required.';
  }
  if (form.transport === 'http') {
    try {
      new URL(form.url);
    } catch {
      errors.url = 'Enter a valid URL.';
    }
  }
  if (form.timeout.trim() && Number.isNaN(Number(form.timeout))) {
    errors.timeout = 'Timeout must be a number of milliseconds.';
  }
  for (const row of envRows.value) {
    if (!row.key.trim() && row.value.trim()) {
      errors.env = 'Env var keys are required when values are set.';
    }
  }
  return Object.keys(errors).length === 0;
}

function buildEntry(): EditableServer {
  if (form.transport === 'http') {
    return { type: 'http', name: form.name.trim(), url: form.url.trim() };
  }
  const entry: EditableServer = {
    type: 'stdio',
    name: form.name.trim(),
    command: form.command.trim(),
  };
  const args = parseArgs(form.args);
  if (args.length > 0) entry.args = args;
  const env = Object.fromEntries(
    envRows.value
      .map((row) => [row.key.trim(), row.value] as const)
      .filter(([key]) => key.length > 0),
  );
  if (Object.keys(env).length > 0) entry.env = env;
  if (form.timeout.trim()) entry.timeout = Number(form.timeout);
  return entry;
}

function submit(): void {
  try {
    if (!validate()) return;
    emit('save', buildEntry());
    isDirty.value = false;
    guard.markClean();
  } catch (error) {
    errors.args = error instanceof Error ? error.message : String(error);
  }
}

async function requestClose(): Promise<void> {
  if (isDirty.value && !(await guard.confirmIfDirty())) return;
  emit('update:open', false);
}

async function handleOpenChange(open: boolean): Promise<void> {
  if (open) {
    emit('update:open', true);
    return;
  }
  await requestClose();
}

function addEnvRow(): void {
  envRows.value.push({ id: nextEnvId++, key: '', value: '', show: false });
  markDirty();
}

function removeEnvRow(id: number): void {
  envRows.value = envRows.value.filter((row) => row.id !== id);
  markDirty();
}

watch(() => props.open, (open) => {
  if (open) resetForm();
}, { immediate: true });
</script>

<template>
  <Dialog :open="open" @update:open="handleOpenChange">
    <DialogContent class="sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle>{{ title }}</DialogTitle>
        <DialogDescription>{{ description }}</DialogDescription>
      </DialogHeader>

      <form class="grid gap-4" @submit.prevent="submit">
        <div class="grid gap-2">
          <Label for="mcp-server-name">Name</Label>
          <Input id="mcp-server-name" v-model="form.name" @update:model-value="markDirty" />
          <p v-if="errors.name" class="text-sm text-destructive" role="alert">{{ errors.name }}</p>
        </div>

        <div class="grid gap-2">
          <Label for="mcp-server-transport">Transport</Label>
          <Select :model-value="form.transport" @update:model-value="(value) => { form.transport = String(value) as Transport; markDirty(); }">
            <SelectTrigger id="mcp-server-transport">
              <SelectValue placeholder="Select transport" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="stdio">Stdio</SelectItem>
              <SelectItem value="http">Http</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <template v-if="form.transport === 'stdio'">
          <div class="grid gap-2">
            <Label for="mcp-server-command">Command</Label>
            <Input id="mcp-server-command" v-model="form.command" placeholder="npx" @update:model-value="markDirty" />
            <p v-if="errors.command" class="text-sm text-destructive" role="alert">{{ errors.command }}</p>
          </div>
          <div class="grid gap-2">
            <Label for="mcp-server-args">Args</Label>
            <Textarea id="mcp-server-args" v-model="form.args" placeholder="-y @modelcontextprotocol/server-filesystem /tmp" @update:model-value="markDirty" />
            <p v-if="errors.args" class="text-sm text-destructive" role="alert">{{ errors.args }}</p>
          </div>
          <div class="grid gap-2">
            <Label for="mcp-server-timeout">Timeout (ms)</Label>
            <Input id="mcp-server-timeout" v-model="form.timeout" inputmode="numeric" placeholder="Optional" @update:model-value="markDirty" />
            <p v-if="errors.timeout" class="text-sm text-destructive" role="alert">{{ errors.timeout }}</p>
          </div>
          <div class="grid gap-3">
            <div class="flex items-center justify-between gap-2">
              <Label>Env vars</Label>
              <Button type="button" variant="outline" size="sm" @click="addEnvRow"><Plus :size="14" /> Add env var</Button>
            </div>
            <div v-for="row in envRows" :key="row.id" class="grid gap-2 rounded-md bg-muted/40 p-3 sm:grid-cols-[1fr_1fr_auto_auto]">
              <Input v-model="row.key" placeholder="KEY" aria-label="Env key" @update:model-value="markDirty" />
              <Input v-model="row.value" :type="row.show ? 'text' : 'password'" placeholder="Value" aria-label="Env value" @update:model-value="markDirty" />
              <Button type="button" variant="outline" size="sm" @click="row.show = !row.show">{{ row.show ? 'Hide value' : 'Show value' }}</Button>
              <Button type="button" variant="ghost" size="icon-sm" aria-label="Remove env var" @click="removeEnvRow(row.id)"><Trash2 :size="14" /></Button>
            </div>
            <p v-if="errors.env" class="text-sm text-destructive" role="alert">{{ errors.env }}</p>
          </div>
        </template>

        <div v-else class="grid gap-2">
          <Label for="mcp-server-url">URL</Label>
          <Input id="mcp-server-url" v-model="form.url" placeholder="https://example.com/mcp" @update:model-value="markDirty" />
          <p v-if="errors.url" class="text-sm text-destructive" role="alert">{{ errors.url }}</p>
        </div>

        <DialogFooter>
          <Button type="button" variant="outline" @click="requestClose">Cancel</Button>
          <Button type="submit">{{ server ? 'Save changes' : 'Add server' }}</Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
