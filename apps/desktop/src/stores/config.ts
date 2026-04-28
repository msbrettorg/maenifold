// Maenifold config store. Reads/writes ~/maenifold/maenifold.json directly via Electron IPC.
//
// Constitutionally `maenifold.json` is desktop-owned (preferences) — going through MCP would
// be circular: the daemon's transport mutating the daemon's config. The Settings tab is the
// sole owner; the daemon picks up changes via its file watcher (api/config_watch.rs).
//
// Reference: https://pinia.vuejs.org/core-concepts/  (Setup Stores)
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { toast } from 'vue-sonner';
import type { AgentConfig, AgentsConfig } from '../lib/types';

interface ConfigFileApi {
  read(): Promise<{ path: string; content: string; exists: boolean }>;
  write(content: string): Promise<{ path: string }>;
  path(): Promise<string>;
  onChanged(handler: () => void): () => void;
}

function getConfigApi(): ConfigFileApi | null {
  if (typeof window === 'undefined' || !window.electronAPI) return null;
  const api = window.electronAPI;
  if (!api.configRead || !api.configWrite || !api.configPath || !api.onConfigChanged) {
    return null;
  }
  return {
    read: () => api.configRead!(),
    write: (content) => api.configWrite!(content),
    path: () => api.configPath!(),
    onChanged: (handler) => api.onConfigChanged!(handler),
  };
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function parseAgentsConfig(content: string): AgentsConfig {
  if (content.trim().length === 0) return { mcpServers: [], agents: {} };
  const parsed: unknown = JSON.parse(content);
  if (!isRecord(parsed)) {
    throw new Error('maenifold.json root must be a JSON object');
  }
  const config: AgentsConfig = {
    mcpServers: Array.isArray(parsed.mcpServers)
      ? (parsed.mcpServers as AgentsConfig['mcpServers'])
      : [],
    agents: normalizeAgents(parsed.agents),
  };
  if (parsed.settings !== undefined) {
    config.settings = parsed.settings as AgentsConfig['settings'];
  }
  return config;
}

function normalizeAgents(agents: unknown): Record<string, AgentConfig> {
  if (Array.isArray(agents)) {
    const out: Record<string, AgentConfig> = {};
    for (const entry of agents) {
      if (!isRecord(entry) || typeof entry.name !== 'string' || entry.name.length === 0) continue;
      out[entry.name] = entryToAgent(entry);
    }
    return out;
  }
  if (isRecord(agents)) {
    const out: Record<string, AgentConfig> = {};
    for (const [name, entry] of Object.entries(agents)) {
      if (!isRecord(entry)) continue;
      out[name] = entryToAgent({ name, ...entry });
    }
    return out;
  }
  return {};
}

function entryToAgent(entry: Record<string, unknown>): AgentConfig {
  const argsArray = Array.isArray(entry.args)
    ? entry.args.filter((arg): arg is string => typeof arg === 'string' && arg.length > 0)
    : [];
  const command = typeof entry.command === 'string' ? entry.command : '';
  const fullCommand = argsArray.length > 0 ? [command, ...argsArray].filter(Boolean).join(' ') : command;
  const rest = Object.fromEntries(
    Object.entries(entry).filter(([key]) => key !== 'name' && key !== 'args' && key !== 'command'),
  );
  return { ...rest, command: fullCommand } as AgentConfig;
}

// Reverse `entryToAgent`: explode the joined command into command+args for on-disk storage.
function agentToEntry(name: string, agent: AgentConfig): Record<string, unknown> {
  const tokens = (agent.command ?? '').trim().split(/\s+/).filter(Boolean);
  const [command = '', ...args] = tokens;
  const rest: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(agent)) {
    if (key === 'command') continue;
    rest[key] = value;
  }
  const entry: Record<string, unknown> = { name, command, ...rest };
  if (args.length > 0) entry.args = args;
  return entry;
}

function serializeConfig(config: AgentsConfig): string {
  const out: Record<string, unknown> = {};
  if (config.mcpServers !== undefined) out.mcpServers = config.mcpServers;
  out.agents = Object.entries(config.agents).map(([name, agent]) => agentToEntry(name, agent));
  if (config.settings !== undefined) out.settings = config.settings;
  return `${JSON.stringify(out, null, 2)}\n`;
}

export const useConfigStore = defineStore('config', () => {
  const config = ref<AgentsConfig>({ mcpServers: [], agents: {} });
  const configPath = ref<string>('');
  const loading = ref(false);
  const error = ref<string | null>(null);

  let unsubscribeWatcher: (() => void) | null = null;

  const agentNames = computed(() => Object.keys(config.value.agents));
  const hasAgents = computed(() => agentNames.value.length > 0);

  async function loadConfig(): Promise<void> {
    const api = getConfigApi();
    if (!api) {
      error.value = 'Config file IPC unavailable (Electron preload missing)';
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      const result = await api.read();
      configPath.value = result.path;
      config.value = parseAgentsConfig(result.content);
    } catch (e) {
      const message = getErrorMessage(e);
      error.value = message;
      toast.error(`Config load failed: ${message}`);
    } finally {
      loading.value = false;
    }
  }

  async function reload(): Promise<void> {
    await loadConfig();
  }

  // Persist current in-memory config to disk; daemon picks up via api/config_watch.rs.
  async function save(): Promise<void> {
    const api = getConfigApi();
    if (!api) throw new Error('Config file IPC unavailable');
    const serialized = serializeConfig(config.value);
    await api.write(serialized);
  }

  // Replace the entire config and persist; the file watcher will echo back to ensure consistency.
  async function writeRaw(rawText: string): Promise<void> {
    const api = getConfigApi();
    if (!api) throw new Error('Config file IPC unavailable');
    await api.write(rawText);
    config.value = parseAgentsConfig(rawText);
  }

  function getAgent(name: string): AgentConfig | undefined {
    return config.value.agents[name];
  }

  async function addAgent(name: string, agent: AgentConfig): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      config.value.agents[name] = agent;
      await save();
    } catch (e) {
      const message = getErrorMessage(e);
      error.value = message;
      throw new Error(message);
    } finally {
      loading.value = false;
    }
  }

  function updateFromEvent(newConfig: AgentsConfig): void {
    config.value = newConfig;
  }

  function clearError(): void {
    error.value = null;
  }

  async function setupHotReload(): Promise<void> {
    teardown();
    const api = getConfigApi();
    if (!api) return;
    unsubscribeWatcher = api.onChanged(() => {
      void loadConfig();
    });
  }

  function teardown(): void {
    unsubscribeWatcher?.();
    unsubscribeWatcher = null;
  }

  return {
    config,
    configPath,
    loading,
    error,
    agentNames,
    hasAgents,
    loadConfig,
    reload,
    save,
    writeRaw,
    getAgent,
    setupHotReload,
    updateFromEvent,
    clearError,
    addAgent,
    teardown,
  };
});
