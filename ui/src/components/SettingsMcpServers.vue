<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { AlertTriangle, Info, Plus, Server, Trash2, X } from '@lucide/vue';

import { isPlainObject } from '../lib/settingsSchema';
import {
  useAgentsAdminStore,
  type EnvFromCommandEntry,
  type McpServerEntry,
  type McpServerHttpEntry,
  type McpServerStdioEntry,
} from '../stores/agentsAdmin';

type ServerTransport = 'stdio' | 'http';

interface KeyValueRow {
  id: number;
  key: string;
  value: string;
}

interface ServerRow {
  name: string;
  entry: McpServerEntry;
}

const agentsAdminStore = useAgentsAdminStore();

const panelError = ref<string | null>(null);
const editorError = ref<string | null>(null);
const editorOpen = ref(false);
const editingName = ref<string | null>(null);

const form = reactive({
  name: '',
  transport: 'stdio' as ServerTransport,
  command: '',
  argsText: '',
  url: '',
  auth: '',
  timeoutText: '',
  gateway: false,
  envFromCommandText: '',
});

const envRows = ref<KeyValueRow[]>([]);
const envFromFileRows = ref<KeyValueRow[]>([]);
const headerRows = ref<KeyValueRow[]>([]);

let nextRowId = 1;

const serverRows = computed<ServerRow[]>(() =>
  Object.entries(agentsAdminStore.mcpServers)
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

function isHttpEntry(entry: McpServerEntry): entry is McpServerHttpEntry {
  return 'url' in entry;
}

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

function parsePositiveInteger(raw: string, label: string): number | undefined {
  const trimmed = raw.trim();
  if (!trimmed) {
    return undefined;
  }

  const parsed = Number(trimmed);
  if (!Number.isInteger(parsed) || parsed < 1) {
    throw new Error(`${label} must be a whole number greater than zero.`);
  }

  return parsed;
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

function resetForm(name = '', entry?: McpServerEntry): void {
  form.name = name;
  form.transport = entry && isHttpEntry(entry) ? 'http' : 'stdio';
  form.command = entry && !isHttpEntry(entry) ? entry.command : '';
  form.argsText = entry && !isHttpEntry(entry) ? entry.args?.join(' ') ?? '' : '';
  form.url = entry && isHttpEntry(entry) ? entry.url : '';
  form.auth = entry && isHttpEntry(entry) ? entry.auth ?? '' : '';
  form.timeoutText =
    entry && typeof entry.timeout === 'number' ? String(entry.timeout) : '';
  form.gateway = entry?.gateway ?? false;
  form.envFromCommandText =
    entry && !isHttpEntry(entry) ? prettyJson(entry.envFromCommand) : '';

  envRows.value = entry && !isHttpEntry(entry) ? recordToRows(entry.env) : [];
  envFromFileRows.value =
    entry && !isHttpEntry(entry) ? recordToRows(entry.envFromFile) : [];
  headerRows.value = entry && isHttpEntry(entry) ? recordToRows(entry.headers) : [];
  editorError.value = null;
}

function openAdd(): void {
  editingName.value = null;
  resetForm();
  editorOpen.value = true;
}

function openEdit(row: ServerRow): void {
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

function addHeaderRow(): void {
  headerRows.value.push({ id: nextRowId++, key: '', value: '' });
}

function removeEnvRow(id: number): void {
  envRows.value = envRows.value.filter((row) => row.id !== id);
}

function removeEnvFromFileRow(id: number): void {
  envFromFileRows.value = envFromFileRows.value.filter((row) => row.id !== id);
}

function removeHeaderRow(id: number): void {
  headerRows.value = headerRows.value.filter((row) => row.id !== id);
}

function buildEntry(): McpServerEntry {
  const name = form.name.trim();
  if (!name) {
    throw new Error('Server name is required.');
  }

  const duplicate = serverRows.value.some(
    (row) =>
      row.name.toLowerCase() === name.toLowerCase() &&
      row.name !== (editingName.value ?? '')
  );
  if (duplicate) {
    throw new Error(`An MCP server named "${name}" already exists.`);
  }

  const timeout = parsePositiveInteger(form.timeoutText, 'Timeout');

  if (form.transport === 'http') {
    const url = form.url.trim();
    if (!url) {
      throw new Error('HTTP MCP servers require a URL.');
    }

    try {
      new URL(url);
    } catch {
      throw new Error('Enter a valid HTTP MCP server URL.');
    }

    const entry: McpServerHttpEntry = { url };
    const headers = rowsToRecord(headerRows.value);

    if (headers) {
      entry.headers = headers;
    }
    if (form.auth === 'azure-default') {
      entry.auth = 'azure-default';
    }
    if (timeout !== undefined) {
      entry.timeout = timeout;
    }
    if (form.gateway) {
      entry.gateway = true;
    }

    return entry;
  }

  const command = form.command.trim();
  if (!command) {
    throw new Error('Stdio MCP servers require a command.');
  }

  const entry: McpServerStdioEntry = { command };
  const args = parseArgs(form.argsText);
  const env = rowsToRecord(envRows.value);
  const envFromFile = rowsToRecord(envFromFileRows.value);
  const envFromCommand = parseEnvFromCommand(form.envFromCommandText);

  if (args) {
    entry.args = args;
  }
  if (timeout !== undefined) {
    entry.timeout = timeout;
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
  if (form.gateway) {
    entry.gateway = true;
  }

  return entry;
}

function serverSummary(entry: McpServerEntry): string {
  if (isHttpEntry(entry)) {
    const extras: string[] = [];
    const headerCount = Object.keys(entry.headers ?? {}).length;

    if (headerCount > 0) {
      extras.push(`${headerCount} headers`);
    }
    if (entry.auth) {
      extras.push(entry.auth);
    }
    if (entry.timeout) {
      extras.push(`${entry.timeout}s timeout`);
    }

    return [entry.url, ...extras].join(' · ');
  }

  const commandParts = [entry.command, ...(entry.args ?? [])].filter(Boolean).join(' ');
  const extras: string[] = [];
  const envCount = Object.keys(entry.env ?? {}).length;
  const envFileCount = Object.keys(entry.envFromFile ?? {}).length;
  const envCommandCount = Object.keys(entry.envFromCommand ?? {}).length;

  if (envCount > 0) {
    extras.push(`${envCount} env`);
  }
  if (envFileCount > 0) {
    extras.push(`${envFileCount} env-from-file`);
  }
  if (envCommandCount > 0) {
    extras.push(`${envCommandCount} env-from-command`);
  }
  if (entry.timeout) {
    extras.push(`${entry.timeout}s timeout`);
  }

  return [commandParts, ...extras].join(' · ');
}

async function saveServer(): Promise<void> {
  editorError.value = null;
  panelError.value = null;

  let entry: McpServerEntry;
  let name: string;

  try {
    entry = buildEntry();
    name = form.name.trim();
  } catch (error) {
    editorError.value = error instanceof Error ? error.message : String(error);
    return;
  }

  const command = isEditing.value ? 'UpdateMcpServer' : 'AddMcpServer';
  const preview = commandPreview(command, { name, entry });
  const confirmed = window.confirm(
    `${preview}\n\n${isEditing.value ? 'Save these server changes?' : 'Add this MCP server?'}`
  );
  if (!confirmed) {
    return;
  }

  try {
    if (isEditing.value) {
      await agentsAdminStore.updateMcpServer(name, entry);
    } else {
      await agentsAdminStore.addMcpServer(name, entry);
    }
    closeEditor();
  } catch (error) {
    editorError.value = error instanceof Error ? error.message : String(error);
  }
}

async function deleteServer(name: string): Promise<void> {
  panelError.value = null;

  const preview = commandPreview('DeleteMcpServer', { name });
  const confirmed = window.confirm(`${preview}\n\nDelete this MCP server?`);
  if (!confirmed) {
    return;
  }

  try {
    await agentsAdminStore.deleteMcpServer(name);
  } catch (error) {
    panelError.value = error instanceof Error ? error.message : String(error);
  }
}
</script>

<template>
  <div class="mcp-servers-panel">
    <div class="schema-note">
      <Info :size="16" :stroke-width="1.9" />
      <div>
        <strong>Gateway-backed CRUD.</strong>
        MCP servers are read from <code>GetConfig.mcpServers</code> and saved with
        <code>AddMcpServer</code>, <code>UpdateMcpServer</code>, and
        <code>DeleteMcpServer</code>.
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
        <strong>MCP server update error.</strong>
        <span>{{ inlineError }}</span>
      </div>
    </div>

    <section class="settings-card" aria-label="Configured MCP servers">
      <header class="card-head">
        <Server :size="16" :stroke-width="1.9" />
        <h3>{{ serverRows.length }} {{ serverRows.length === 1 ? 'server' : 'servers' }}</h3>
        <div class="spacer" />
        <button class="btn btn-primary" type="button" :disabled="agentsAdminStore.loading" @click="openAdd">
          <Plus :size="14" :stroke-width="2" />
          <span>Add server</span>
        </button>
      </header>

      <div class="card-body">
        <div v-if="agentsAdminStore.loading && serverRows.length === 0" class="loading-state">
          Loading configured MCP servers…
        </div>

        <div v-else-if="serverRows.length === 0" class="empty-state">
          <Server :size="34" :stroke-width="1.5" />
          <div>No MCP servers configured.</div>
          <div class="empty-copy">
            Add a stdio (<code>command + args</code>) or remote (<code>url</code>) server. Toggle
            <code>gateway</code> to expose it through the Maenifold gateway.
          </div>
        </div>

        <div v-else class="list">
          <article v-for="row in serverRows" :key="row.name" class="list-row">
            <div class="row-icon" aria-hidden="true">
              <Server :size="16" :stroke-width="1.85" />
            </div>

            <div class="row-main">
              <div class="row-name">
                <span>{{ row.name }}</span>
                <span class="badge badge-type">{{ isHttpEntry(row.entry) ? 'http' : 'stdio' }}</span>
                <span v-if="row.entry.gateway" class="badge badge-gateway">gateway</span>
              </div>
              <div class="row-summary">{{ serverSummary(row.entry) }}</div>
            </div>

            <div class="row-actions">
              <button class="btn" type="button" :disabled="agentsAdminStore.loading" @click="openEdit(row)">
                Edit
              </button>
              <button
                class="btn btn-danger"
                type="button"
                :disabled="agentsAdminStore.loading"
                @click="deleteServer(row.name)"
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
      <section class="editor-dialog" role="dialog" aria-modal="true" aria-label="MCP server editor">
        <header class="editor-head">
          <div>
            <h3>{{ isEditing ? 'Edit MCP server' : 'Add MCP server' }}</h3>
            <p>
              Configure either a stdio launcher or an HTTP endpoint, then confirm the exact
              gateway command preview before saving.
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
              <span>Server name</span>
              <input
                v-model="form.name"
                class="text-input"
                type="text"
                placeholder="filesystem"
                :disabled="isEditing"
              />
            </label>

            <div class="field">
              <span>Transport</span>
              <div class="transport-toggle">
                <button
                  class="transport-btn"
                  :class="{ active: form.transport === 'stdio' }"
                  type="button"
                  @click="form.transport = 'stdio'"
                >
                  Stdio
                </button>
                <button
                  class="transport-btn"
                  :class="{ active: form.transport === 'http' }"
                  type="button"
                  @click="form.transport = 'http'"
                >
                  HTTP
                </button>
              </div>
            </div>

            <label v-if="form.transport === 'stdio'" class="field">
              <span>Command</span>
              <input
                v-model="form.command"
                class="text-input mono-input"
                type="text"
                placeholder="npx"
              />
            </label>

            <label v-else class="field">
              <span>URL</span>
              <input
                v-model="form.url"
                class="text-input mono-input"
                type="text"
                placeholder="https://example.com/mcp"
              />
            </label>

            <label class="field">
              <span>Timeout (seconds)</span>
              <input
                v-model="form.timeoutText"
                class="text-input mono-input"
                type="text"
                inputmode="numeric"
                placeholder="Optional"
              />
            </label>

            <label v-if="form.transport === 'stdio'" class="field field-full">
              <span>Args</span>
              <textarea
                v-model="form.argsText"
                class="textarea-input mono-input"
                rows="3"
                placeholder="-y @modelcontextprotocol/server-filesystem ."
              />
              <small class="field-hint">
                Enter whitespace-separated args or a JSON array of strings when quoting matters.
              </small>
            </label>

            <label v-else class="field">
              <span>Auth</span>
              <select v-model="form.auth" class="select-input">
                <option value="">None</option>
                <option value="azure-default">azure-default</option>
              </select>
            </label>

            <label class="checkbox-row field-full">
              <input v-model="form.gateway" type="checkbox" />
              <span>Expose this server through the Maenifold gateway.</span>
            </label>

            <section v-if="form.transport === 'stdio'" class="field field-full key-value-editor">
              <div class="kv-head">
                <div>
                  <strong>Environment variables</strong>
                  <span>Inline <code>env</code> entries written onto the server config.</span>
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

            <section v-else class="field field-full key-value-editor">
              <div class="kv-head">
                <div>
                  <strong>Headers</strong>
                  <span>Optional request headers stored under <code>headers</code>.</span>
                </div>

                <button class="btn" type="button" @click="addHeaderRow">
                  <Plus :size="14" :stroke-width="2" />
                  <span>Add header</span>
                </button>
              </div>

              <div v-if="headerRows.length > 0" class="kv-list">
                <div v-for="row in headerRows" :key="row.id" class="kv-row">
                  <input
                    v-model="row.key"
                    class="text-input mono-input"
                    type="text"
                    placeholder="Authorization"
                  />
                  <span class="kv-separator">:</span>
                  <input
                    v-model="row.value"
                    class="text-input mono-input"
                    type="text"
                    placeholder="Bearer ..."
                  />
                  <button
                    class="icon-btn icon-btn-danger"
                    type="button"
                    aria-label="Remove header"
                    @click="removeHeaderRow(row.id)"
                  >
                    <Trash2 :size="14" :stroke-width="1.9" />
                  </button>
                </div>
              </div>
              <div v-else class="kv-empty">No custom headers configured.</div>
            </section>
          </div>

          <details v-if="form.transport === 'stdio'" class="advanced-group" open>
            <summary class="advanced-summary">
              <span>Advanced stdio wiring</span>
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
            </div>
          </details>
        </div>

        <footer class="editor-footer">
          <button class="btn" type="button" @click="closeEditor">Cancel</button>
          <button class="btn btn-primary" type="button" :disabled="agentsAdminStore.loading" @click="saveServer">
            <Plus v-if="!isEditing" :size="14" :stroke-width="2" />
            <span>{{ isEditing ? 'Save changes' : 'Add server' }}</span>
          </button>
        </footer>
      </section>
    </div>
  </div>
</template>

<style scoped>
.mcp-servers-panel {
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
.icon-btn,
.transport-btn {
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

.btn,
.transport-btn {
  padding: 0.48rem 0.72rem;
}

.icon-btn {
  width: 2.1rem;
  height: 2.1rem;
  padding: 0;
}

.btn:hover,
.icon-btn:hover,
.transport-btn:hover {
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

.transport-toggle {
  display: flex;
  gap: 0.55rem;
}

.transport-btn {
  min-width: 5.4rem;
}

.transport-btn.active {
  color: var(--text-accent);
  border-color: color-mix(in srgb, var(--text-accent) 35%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
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
  max-width: 36rem;
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

.badge-gateway {
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 10%, var(--bg-main));
  border: 1px solid color-mix(in srgb, var(--text-accent) 35%, var(--border-color));
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

.field > span,
.kv-head strong {
  color: var(--text-primary);
}

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
.textarea-input,
.select-input {
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

.select-input {
  appearance: none;
}

.mono-input {
  font-family: ui-monospace, SFMono-Regular, Consolas, 'Liberation Mono', monospace;
  font-size: 0.84rem;
}

.text-input:focus,
.textarea-input:focus,
.select-input:focus {
  outline: none;
  border-color: var(--text-accent);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--text-accent) 35%, transparent);
}

.checkbox-row {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  padding: 0.2rem 0;
  color: var(--text-secondary);
}

.checkbox-row input {
  width: 1rem;
  height: 1rem;
  accent-color: var(--text-accent);
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
  .mcp-servers-panel {
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
  .transport-toggle,
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
