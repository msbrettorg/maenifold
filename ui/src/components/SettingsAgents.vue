<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { AlertTriangle, Info, MessageSquare, Plus, Trash2, X } from '@lucide/vue';

import { isPlainObject } from '../lib/settingsSchema';
import {
  useAgentsAdminStore,
  type AgentEntry,
  type EnvFromCommandEntry,
  type McpServerEntry,
} from '../stores/agentsAdmin';

interface KeyValueRow {
  id: number;
  key: string;
  value: string;
}

interface AgentRow {
  name: string;
  entry: AgentEntry;
}

const agentsAdminStore = useAgentsAdminStore();

const panelError = ref<string | null>(null);
const editorError = ref<string | null>(null);
const editorOpen = ref(false);
const editingName = ref<string | null>(null);

const form = reactive({
  name: '',
  command: '',
  argsText: '',
  envFromCommandText: '',
  scopedMcpServersText: '',
});

const envRows = ref<KeyValueRow[]>([]);
const envFromFileRows = ref<KeyValueRow[]>([]);

let nextRowId = 1;

const agentRows = computed<AgentRow[]>(() =>
  Object.entries(agentsAdminStore.agents)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([name, entry]) => ({ name, entry }))
);

const isEditing = computed(() => editingName.value !== null);

const currentError = computed(() => panelError.value ?? agentsAdminStore.error);

const daemonError = computed(() => {
  const message = currentError.value ?? '';
  return /MCP request failed|Failed to fetch|ECONNREFUSED|fetch failed|network/i.test(message);
});

const inlineError = computed(() =>
  currentError.value && !daemonError.value ? currentError.value : null
);

onMounted(() => {
  void agentsAdminStore.fetchConfig().catch((error) => {
    panelError.value = error instanceof Error ? error.message : String(error);
  });
});

function commandPreview(command: string, parameters: Record<string, unknown>): string {
  const envelope = {
    domain: 'maenifold.system',
    command,
    parameters,
  };

  return ['GatewayExecute', JSON.stringify(envelope, null, 2)].join('\n');
}

function prettyJson(value: unknown): string {
  if (!isPlainObject(value) || Object.keys(value).length === 0) {
    return '';
  }

  return JSON.stringify(value, null, 2);
}

function recordToRows(record?: Record<string, string>): KeyValueRow[] {
  return Object.entries(record ?? {})
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([key, value]) => ({
      id: nextRowId++,
      key,
      value,
    }));
}

function rowsToRecord(rows: KeyValueRow[]): Record<string, string> | undefined {
  const entries = rows
    .map((row) => [row.key.trim(), row.value] as const)
    .filter(([key]) => key.length > 0);

  if (entries.length === 0) {
    return undefined;
  }

  return Object.fromEntries(entries);
}

function parseArgs(raw: string): string[] | undefined {
  const trimmed = raw.trim();
  if (!trimmed) {
    return undefined;
  }

  if (trimmed.startsWith('[')) {
    const parsed = JSON.parse(trimmed) as unknown;
    if (!Array.isArray(parsed) || !parsed.every((item) => typeof item === 'string')) {
      throw new Error('Args JSON must be an array of strings.');
    }
    return parsed.length > 0 ? parsed : undefined;
  }

  const args = trimmed.split(/\s+/).filter(Boolean);
  return args.length > 0 ? args : undefined;
}

function parseEnvFromCommand(raw: string): Record<string, EnvFromCommandEntry> | undefined {
  const trimmed = raw.trim();
  if (!trimmed) {
    return undefined;
  }

  const parsed = JSON.parse(trimmed) as unknown;
  if (!isPlainObject(parsed)) {
    throw new Error('envFromCommand must be a JSON object keyed by environment variable name.');
  }

  const nextEntries: Record<string, EnvFromCommandEntry> = {};

  for (const [key, value] of Object.entries(parsed)) {
    if (!isPlainObject(value) || typeof value.command !== 'string' || !value.command.trim()) {
      throw new Error(`envFromCommand.${key} must include a non-empty command string.`);
    }

    if (
      value.args !== undefined &&
      (!Array.isArray(value.args) || !value.args.every((item) => typeof item === 'string'))
    ) {
      throw new Error(`envFromCommand.${key}.args must be an array of strings.`);
    }

    nextEntries[key] = {
      command: value.command,
      ...(value.args && value.args.length > 0 ? { args: value.args as string[] } : {}),
    };
  }

  return Object.keys(nextEntries).length > 0 ? nextEntries : undefined;
}

function parseScopedMcpServers(raw: string): Record<string, McpServerEntry> | undefined {
  const trimmed = raw.trim();
  if (!trimmed) {
    return undefined;
  }

  const parsed = JSON.parse(trimmed) as unknown;
  if (!isPlainObject(parsed)) {
    throw new Error('Scoped MCP servers must be a JSON object keyed by server name.');
  }

  return parsed as Record<string, McpServerEntry>;
}

function resetForm(name = '', entry?: AgentEntry): void {
  form.name = name;
  form.command = entry?.command ?? '';
  form.argsText = entry?.args?.join(' ') ?? '';
  form.envFromCommandText = prettyJson(entry?.envFromCommand);
  form.scopedMcpServersText = prettyJson(entry?.mcpServers);
  envRows.value = recordToRows(entry?.env);
  envFromFileRows.value = recordToRows(entry?.envFromFile);
  editorError.value = null;
}

function openAdd(): void {
  editingName.value = null;
  resetForm();
  editorOpen.value = true;
}

function openEdit(row: AgentRow): void {
  editingName.value = row.name;
  resetForm(row.name, row.entry);
  editorOpen.value = true;
}

function closeEditor(): void {
  editorOpen.value = false;
  editingName.value = null;
  editorError.value = null;
}

function addEnvRow(): void {
  envRows.value.push({ id: nextRowId++, key: '', value: '' });
}

function addEnvFromFileRow(): void {
  envFromFileRows.value.push({ id: nextRowId++, key: '', value: '' });
}

function removeEnvRow(id: number): void {
  envRows.value = envRows.value.filter((row) => row.id !== id);
}

function removeEnvFromFileRow(id: number): void {
  envFromFileRows.value = envFromFileRows.value.filter((row) => row.id !== id);
}

function buildEntry(): AgentEntry {
  const name = form.name.trim();
  if (!name) {
    throw new Error('Agent name is required.');
  }

  const duplicate = agentRows.value.some(
    (row) =>
      row.name.toLowerCase() === name.toLowerCase() &&
      row.name !== (editingName.value ?? '')
  );
  if (duplicate) {
    throw new Error(`An agent named "${name}" already exists.`);
  }

  const command = form.command.trim();
  if (!command) {
    throw new Error('Agent command is required.');
  }

  const entry: AgentEntry = { command };
  const args = parseArgs(form.argsText);
  const env = rowsToRecord(envRows.value);
  const envFromFile = rowsToRecord(envFromFileRows.value);
  const envFromCommand = parseEnvFromCommand(form.envFromCommandText);
  const mcpServers = parseScopedMcpServers(form.scopedMcpServersText);

  if (args) {
    entry.args = args;
  }
  if (env) {
    entry.env = env;
  }
  if (envFromFile) {
    entry.envFromFile = envFromFile;
  }
  if (envFromCommand) {
    entry.envFromCommand = envFromCommand;
  }
  if (mcpServers) {
    entry.mcpServers = mcpServers;
  }

  return entry;
}

function commandSummary(entry: AgentEntry): string {
  const commandParts = [entry.command, ...(entry.args ?? [])].filter(Boolean).join(' ');
  const extras: string[] = [];

  const envCount = Object.keys(entry.env ?? {}).length;
  const envFileCount = Object.keys(entry.envFromFile ?? {}).length;
  const envCommandCount = Object.keys(entry.envFromCommand ?? {}).length;
  const scopedMcpCount = Object.keys(entry.mcpServers ?? {}).length;

  if (envCount > 0) {
    extras.push(`${envCount} env`);
  }
  if (envFileCount > 0) {
    extras.push(`${envFileCount} env-from-file`);
  }
  if (envCommandCount > 0) {
    extras.push(`${envCommandCount} env-from-command`);
  }
  if (scopedMcpCount > 0) {
    extras.push(`${scopedMcpCount} scoped MCP`);
  }

  return [commandParts, ...extras].join(' · ');
}

async function saveAgent(): Promise<void> {
  editorError.value = null;
  panelError.value = null;

  let entry: AgentEntry;
  let name: string;

  try {
    entry = buildEntry();
    name = form.name.trim();
  } catch (error) {
    editorError.value = error instanceof Error ? error.message : String(error);
    return;
  }

  const command = isEditing.value ? 'UpdateAgent' : 'AddAgent';
  const preview = commandPreview(command, { name, config: entry });
  const confirmed = window.confirm(
    `${preview}\n\n${isEditing.value ? 'Save these agent changes?' : 'Add this agent?'}`
  );
  if (!confirmed) {
    return;
  }

  try {
    if (isEditing.value) {
      await agentsAdminStore.updateAgent(name, entry);
    } else {
      await agentsAdminStore.addAgent(name, entry);
    }
    closeEditor();
  } catch (error) {
    editorError.value = error instanceof Error ? error.message : String(error);
  }
}

async function deleteAgent(name: string): Promise<void> {
  panelError.value = null;

  const preview = commandPreview('DeleteAgent', { name });
  const confirmed = window.confirm(`${preview}\n\nDelete this agent?`);
  if (!confirmed) {
    return;
  }

  try {
    await agentsAdminStore.deleteAgent(name);
  } catch (error) {
    panelError.value = error instanceof Error ? error.message : String(error);
  }
}
</script>

<template>
  <div class="agents-panel">
    <div class="schema-note">
      <Info :size="16" :stroke-width="1.9" />
      <div>
        <strong>Gateway-backed CRUD.</strong>
        Agents are read from <code>GetConfig.agents</code> and saved with
        <code>AddAgent</code>, <code>UpdateAgent</code>, and <code>DeleteAgent</code>.
      </div>
    </div>

    <div v-if="daemonError" class="soft-banner" role="status">
      <AlertTriangle :size="18" :stroke-width="1.9" />
      <div>
        <strong>Daemon unreachable.</strong>
        <span>{{ currentError }}</span>
      </div>
    </div>

    <div v-if="inlineError" class="inline-error" role="alert">
      <AlertTriangle :size="18" :stroke-width="1.9" />
      <div>
        <strong>Agent update error.</strong>
        <span>{{ inlineError }}</span>
      </div>
    </div>

    <section class="settings-card" aria-label="Configured agents">
      <header class="card-head">
        <MessageSquare :size="16" :stroke-width="1.9" />
        <h3>{{ agentRows.length }} {{ agentRows.length === 1 ? 'agent' : 'agents' }}</h3>
        <div class="spacer" />
        <button class="btn btn-primary" type="button" :disabled="agentsAdminStore.loading" @click="openAdd">
          <Plus :size="14" :stroke-width="2" />
          <span>Add agent</span>
        </button>
      </header>

      <div class="card-body">
        <div v-if="agentsAdminStore.loading && agentRows.length === 0" class="loading-state">
          Loading configured agents…
        </div>

        <div v-else-if="agentRows.length === 0" class="empty-state">
          <MessageSquare :size="34" :stroke-width="1.5" />
          <div>No ACP agents configured.</div>
          <div class="empty-copy">
            Add a stdio ACP agent with a binary, optional args, and any environment wiring it
            needs to launch successfully.
          </div>
        </div>

        <div v-else class="list">
          <article v-for="row in agentRows" :key="row.name" class="list-row">
            <div class="row-icon" aria-hidden="true">
              <MessageSquare :size="16" :stroke-width="1.85" />
            </div>

            <div class="row-main">
              <div class="row-name">
                <span>{{ row.name }}</span>
                <span class="badge badge-type">stdio</span>
              </div>
              <div class="row-summary">{{ commandSummary(row.entry) }}</div>
            </div>

            <div class="row-actions">
              <button class="btn" type="button" :disabled="agentsAdminStore.loading" @click="openEdit(row)">
                Edit
              </button>
              <button
                class="btn btn-danger"
                type="button"
                :disabled="agentsAdminStore.loading"
                @click="deleteAgent(row.name)"
              >
                <Trash2 :size="14" :stroke-width="1.9" />
                <span>Delete</span>
              </button>
            </div>
          </article>
        </div>
      </div>
    </section>

    <div v-if="editorOpen" class="editor-overlay" @click.self="closeEditor">
      <section class="editor-dialog" role="dialog" aria-modal="true" aria-label="Agent editor">
        <header class="editor-head">
          <div>
            <h3>{{ isEditing ? 'Edit agent' : 'Add agent' }}</h3>
            <p>
              Configure the agent launch command and any environment sources before sending the
              exact gateway command preview.
            </p>
          </div>

          <button class="icon-btn" type="button" aria-label="Close editor" @click="closeEditor">
            <X :size="18" :stroke-width="1.9" />
          </button>
        </header>

        <div class="editor-body">
          <div v-if="editorError" class="editor-error" role="alert">
            <AlertTriangle :size="18" :stroke-width="1.9" />
            <span>{{ editorError }}</span>
          </div>

          <div class="form-grid">
            <label class="field">
              <span>Agent name</span>
              <input
                v-model="form.name"
                class="text-input"
                type="text"
                placeholder="Copilot CLI"
                :disabled="isEditing"
              />
            </label>

            <label class="field">
              <span>Command</span>
              <input
                v-model="form.command"
                class="text-input mono-input"
                type="text"
                placeholder="npx.cmd"
              />
            </label>

            <label class="field field-full">
              <span>Args</span>
              <textarea
                v-model="form.argsText"
                class="textarea-input mono-input"
                rows="3"
                placeholder="@github/copilot@latest --acp"
              />
              <small class="field-hint">
                Enter whitespace-separated args or a JSON array of strings when quoting matters.
              </small>
            </label>

            <section class="field field-full key-value-editor">
              <div class="kv-head">
                <div>
                  <strong>Environment variables</strong>
                  <span>Inline <code>env</code> entries written onto the agent config.</span>
                </div>

                <button class="btn" type="button" @click="addEnvRow">
                  <Plus :size="14" :stroke-width="2" />
                  <span>Add env</span>
                </button>
              </div>

              <div v-if="envRows.length > 0" class="kv-list">
                <div v-for="row in envRows" :key="row.id" class="kv-row">
                  <input v-model="row.key" class="text-input mono-input" type="text" placeholder="KEY" />
                  <span class="kv-separator">=</span>
                  <input
                    v-model="row.value"
                    class="text-input mono-input"
                    type="text"
                    placeholder="value"
                  />
                  <button class="icon-btn icon-btn-danger" type="button" aria-label="Remove env" @click="removeEnvRow(row.id)">
                    <Trash2 :size="14" :stroke-width="1.9" />
                  </button>
                </div>
              </div>
              <div v-else class="kv-empty">No inline environment variables configured.</div>
            </section>
          </div>

          <details class="advanced-group" open>
            <summary class="advanced-summary">
              <span>Advanced wiring</span>
              <span class="advanced-chip">optional</span>
            </summary>

            <div class="advanced-body">
              <section class="field key-value-editor">
                <div class="kv-head">
                  <div>
                    <strong>Environment from file</strong>
                    <span>Map variable names to file paths via <code>envFromFile</code>.</span>
                  </div>

                  <button class="btn" type="button" @click="addEnvFromFileRow">
                    <Plus :size="14" :stroke-width="2" />
                    <span>Add file binding</span>
                  </button>
                </div>

                <div v-if="envFromFileRows.length > 0" class="kv-list">
                  <div v-for="row in envFromFileRows" :key="row.id" class="kv-row">
                    <input v-model="row.key" class="text-input mono-input" type="text" placeholder="TOKEN" />
                    <span class="kv-separator">→</span>
                    <input
                      v-model="row.value"
                      class="text-input mono-input"
                      type="text"
                      placeholder="C:\\secrets\\token.txt"
                    />
                    <button
                      class="icon-btn icon-btn-danger"
                      type="button"
                      aria-label="Remove file binding"
                      @click="removeEnvFromFileRow(row.id)"
                    >
                      <Trash2 :size="14" :stroke-width="1.9" />
                    </button>
                  </div>
                </div>
                <div v-else class="kv-empty">No env-from-file bindings configured.</div>
              </section>

              <label class="field">
                <span>Environment from command (JSON)</span>
                <textarea
                  v-model="form.envFromCommandText"
                  class="textarea-input mono-input"
                  rows="6"
                  placeholder='{"TOKEN":{"command":"gh","args":["auth","token"]}}'
                />
                <small class="field-hint">
                  Matches the backend <code>envFromCommand</code> shape: each key maps to
                  <code>{ "command": string, "args"?: string[] }</code>.
                </small>
              </label>

              <label class="field">
                <span>Scoped MCP servers (JSON)</span>
                <textarea
                  v-model="form.scopedMcpServersText"
                  class="textarea-input mono-input"
                  rows="8"
                  placeholder='{"filesystem":{"command":"npx","args":["-y","@modelcontextprotocol/server-filesystem","."]}}'
                />
                <small class="field-hint">
                  Optional <code>mcpServers</code> object stored directly on this agent entry.
                </small>
              </label>
            </div>
          </details>
        </div>

        <footer class="editor-footer">
          <button class="btn" type="button" @click="closeEditor">Cancel</button>
          <button class="btn btn-primary" type="button" :disabled="agentsAdminStore.loading" @click="saveAgent">
            <Plus v-if="!isEditing" :size="14" :stroke-width="2" />
            <span>{{ isEditing ? 'Save changes' : 'Add agent' }}</span>
          </button>
        </footer>
      </section>
    </div>
  </div>
</template>

<style scoped>
.agents-panel {
  padding: 1.15rem;
}

.schema-note,
.soft-banner,
.inline-error,
.loading-state {
  display: flex;
  gap: 0.75rem;
  padding: 0.9rem 1rem;
  border-radius: 14px;
  border: 1px solid var(--border-color);
}

.schema-note,
.soft-banner,
.inline-error {
  margin-bottom: 1rem;
}

.schema-note {
  background: color-mix(in srgb, var(--text-accent) 6%, transparent);
  color: var(--text-secondary);
}

.schema-note svg {
  flex-shrink: 0;
  color: var(--text-accent);
  margin-top: 0.1rem;
}

.schema-note code,
.field-hint code,
.kv-head code,
.empty-copy code {
  font-family: ui-monospace, SFMono-Regular, Consolas, 'Liberation Mono', monospace;
  font-size: 0.78rem;
}

.soft-banner,
.inline-error,
.editor-error {
  align-items: flex-start;
}

.soft-banner {
  border-color: color-mix(in srgb, var(--bg-warning) 45%, var(--border-color));
  background: color-mix(in srgb, var(--bg-warning) 18%, var(--bg-main));
  color: var(--text-primary);
}

.inline-error,
.editor-error {
  border: 1px solid color-mix(in srgb, var(--bg-danger) 45%, var(--border-color));
  background: color-mix(in srgb, var(--bg-danger) 12%, var(--bg-main));
  color: var(--text-primary);
}

.soft-banner svg,
.inline-error svg,
.editor-error svg {
  flex-shrink: 0;
  margin-top: 0.1rem;
}

.soft-banner span,
.inline-error span {
  display: block;
  margin-top: 0.2rem;
  color: var(--text-secondary);
  line-height: 1.45;
}

.settings-card {
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: var(--bg-main);
  overflow: hidden;
}

.card-head,
.editor-head,
.editor-footer {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.95rem 1rem;
  background: color-mix(in srgb, var(--bg-sidebar) 55%, var(--bg-main));
}

.card-head {
  border-bottom: 1px solid var(--border-color);
}

.card-head svg {
  flex-shrink: 0;
  color: var(--text-accent);
}

.card-head h3 {
  margin: 0;
  font-size: 0.96rem;
  font-weight: 600;
  color: var(--text-primary);
}

.spacer {
  flex: 1;
}

.card-body {
  padding: 1rem;
}

.btn,
.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.35rem;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: var(--bg-sidebar);
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.8rem;
  cursor: pointer;
  transition: background 0.12s ease, border-color 0.12s ease, color 0.12s ease;
}

.btn {
  padding: 0.48rem 0.72rem;
}

.icon-btn {
  width: 2.1rem;
  height: 2.1rem;
  padding: 0;
}

.btn:hover,
.icon-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.btn:disabled,
.icon-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.btn-primary {
  color: #fff;
  background: var(--text-accent);
  border-color: var(--text-accent);
}

.btn-primary:hover {
  filter: brightness(1.08);
}

.btn-danger,
.icon-btn-danger {
  color: #ff9b9b;
  border-color: color-mix(in srgb, var(--bg-danger) 52%, var(--border-color));
}

.btn-danger:hover,
.icon-btn-danger:hover {
  background: color-mix(in srgb, var(--bg-danger) 20%, var(--bg-main));
  color: #ffb3b3;
}

.loading-state {
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--bg-sidebar) 45%, var(--bg-main));
}

.empty-state {
  display: grid;
  justify-items: center;
  gap: 0.5rem;
  padding: 2rem 1rem;
  text-align: center;
  color: var(--text-muted);
}

.empty-state svg {
  color: var(--text-muted);
}

.empty-copy {
  max-width: 34rem;
  font-size: 0.82rem;
  line-height: 1.55;
}

.list {
  display: flex;
  flex-direction: column;
}

.list-row {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  padding: 0.78rem 0.1rem;
  border-bottom: 1px solid color-mix(in srgb, var(--border-color) 55%, transparent);
}

.list-row:last-child {
  border-bottom: none;
}

.row-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 10px;
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--text-accent) 22%, var(--border-color));
  flex-shrink: 0;
}

.row-main {
  min-width: 0;
  flex: 1;
}

.row-name {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.92rem;
  font-weight: 600;
  color: var(--text-primary);
}

.badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.08rem 0.42rem;
  border-radius: 999px;
  font-size: 0.64rem;
  font-weight: 600;
  letter-spacing: 0.03em;
  text-transform: uppercase;
}

.badge-type {
  color: #4ec98a;
  background: color-mix(in srgb, var(--bg-success) 12%, var(--bg-main));
  border: 1px solid color-mix(in srgb, var(--bg-success) 40%, var(--border-color));
}

.row-summary {
  margin-top: 0.15rem;
  font-family: ui-monospace, SFMono-Regular, Consolas, 'Liberation Mono', monospace;
  font-size: 0.75rem;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.row-actions {
  display: flex;
  gap: 0.45rem;
  flex-shrink: 0;
}

.editor-overlay {
  position: fixed;
  inset: 0;
  z-index: 1100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.25rem;
  background: color-mix(in srgb, #000 55%, transparent);
}

.editor-dialog {
  width: min(920px, 100%);
  max-height: calc(100vh - 2.5rem);
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-color);
  border-radius: 18px;
  background: var(--bg-main);
  overflow: hidden;
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.35);
}

.editor-head {
  align-items: flex-start;
  justify-content: space-between;
  border-bottom: 1px solid var(--border-color);
}

.editor-head h3 {
  margin: 0;
  font-size: 1.08rem;
  font-weight: 600;
  color: var(--text-primary);
}

.editor-head p {
  margin: 0.25rem 0 0;
  color: var(--text-secondary);
  font-size: 0.86rem;
  line-height: 1.5;
}

.editor-body {
  display: grid;
  gap: 1rem;
  padding: 1rem;
  overflow: auto;
}

.editor-error {
  display: flex;
  gap: 0.75rem;
  padding: 0.85rem 0.95rem;
  border-radius: 14px;
}

.editor-error span {
  line-height: 1.5;
}

.form-grid,
.advanced-body {
  display: grid;
  gap: 1rem;
}

.form-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.field {
  display: grid;
  gap: 0.45rem;
  min-width: 0;
}

.field-full {
  grid-column: 1 / -1;
}

.field span,
.kv-head strong {
  color: var(--text-primary);
}

.field label,
.field > span {
  font-size: 0.82rem;
  font-weight: 600;
}

.field-hint,
.kv-head span,
.kv-empty {
  font-size: 0.76rem;
  color: var(--text-muted);
  line-height: 1.5;
}

.text-input,
.textarea-input {
  width: 100%;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: var(--bg-sidebar);
  color: var(--text-primary);
  font: inherit;
  padding: 0.62rem 0.75rem;
}

.textarea-input {
  resize: vertical;
}

.mono-input {
  font-family: ui-monospace, SFMono-Regular, Consolas, 'Liberation Mono', monospace;
  font-size: 0.84rem;
}

.text-input:focus,
.textarea-input:focus {
  outline: none;
  border-color: var(--text-accent);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--text-accent) 35%, transparent);
}

.key-value-editor,
.advanced-group {
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: color-mix(in srgb, var(--bg-sidebar) 35%, var(--bg-main));
  overflow: hidden;
}

.kv-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.9rem 0.95rem;
  border-bottom: 1px solid color-mix(in srgb, var(--border-color) 65%, transparent);
}

.kv-list {
  display: grid;
  gap: 0.75rem;
  padding: 0.95rem;
}

.kv-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr) auto;
  gap: 0.55rem;
  align-items: center;
}

.kv-separator {
  color: var(--text-muted);
  font-family: ui-monospace, SFMono-Regular, Consolas, 'Liberation Mono', monospace;
  font-size: 0.86rem;
}

.kv-empty {
  padding: 0.95rem;
}

.advanced-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.95rem 1rem;
  cursor: pointer;
  list-style: none;
}

.advanced-summary::-webkit-details-marker {
  display: none;
}

.advanced-chip {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.08rem 0.5rem;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 55%, var(--bg-main));
  color: var(--text-secondary);
  font-size: 0.68rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.advanced-body {
  padding: 0 1rem 1rem;
}

.editor-footer {
  justify-content: flex-end;
  border-top: 1px solid var(--border-color);
}

@media (max-width: 840px) {
  .agents-panel {
    padding: 1rem;
  }

  .list-row,
  .card-head,
  .editor-head,
  .editor-footer {
    flex-wrap: wrap;
  }

  .row-actions {
    width: 100%;
  }

  .form-grid {
    grid-template-columns: 1fr;
  }

  .field-full {
    grid-column: auto;
  }
}

@media (max-width: 640px) {
  .kv-head,
  .advanced-summary {
    flex-direction: column;
    align-items: flex-start;
  }

  .kv-row {
    grid-template-columns: 1fr;
  }

  .kv-separator {
    display: none;
  }

  .icon-btn-danger {
    justify-self: flex-start;
  }

  .editor-overlay {
    padding: 0.85rem;
  }
}
</style>
