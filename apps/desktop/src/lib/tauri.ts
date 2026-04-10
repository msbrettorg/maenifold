// Tauri IPC wrapper for ACP communication
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import type {
  AgentConfig,
  AgentsConfig,
  AgentInstance,
  AgentMessage,
  AgentStderr,
  MaenifoldSettings,
  McpServerEntry,
} from './types';

export async function getConfig(): Promise<AgentsConfig> {
  return invoke<AgentsConfig>('get_config');
}

export async function reloadConfig(): Promise<AgentsConfig> {
  return invoke<AgentsConfig>('reload_config');
}

export async function getConfigPath(): Promise<string> {
  return invoke<string>('get_config_path');
}

export async function spawnAgent(name: string): Promise<AgentInstance> {
  return invoke<AgentInstance>('spawn_agent', { name });
}

export async function sendToAgent(agentId: string, message: string): Promise<void> {
  return invoke<void>('send_to_agent', { agentId, message });
}

export async function killAgent(agentId: string): Promise<void> {
  return invoke<void>('kill_agent', { agentId });
}

export async function addAgent(name: string, config: AgentConfig): Promise<AgentsConfig> {
  return invoke<AgentsConfig>('add_agent', { name, config });
}

export async function removeAgent(name: string): Promise<AgentsConfig> {
  return invoke<AgentsConfig>('remove_agent', { name });
}

export async function updateAgent(name: string, config: AgentConfig): Promise<AgentsConfig> {
  return invoke<AgentsConfig>('update_agent', { name, config });
}

export async function addMcpServer(entry: McpServerEntry): Promise<AgentsConfig> {
  return invoke<AgentsConfig>('add_mcp_server', { entry });
}

export async function removeMcpServer(name: string): Promise<AgentsConfig> {
  return invoke<AgentsConfig>('remove_mcp_server', { name });
}

export async function updateMcpServer(name: string, entry: McpServerEntry): Promise<AgentsConfig> {
  return invoke<AgentsConfig>('update_mcp_server', { name, entry });
}

export async function getMaenifoldSettings(): Promise<MaenifoldSettings> {
  return invoke<MaenifoldSettings>('get_maenifold_settings');
}

export async function updateMaenifoldSettings(settings: MaenifoldSettings): Promise<AgentsConfig> {
  return invoke<AgentsConfig>('update_maenifold_settings', { settings });
}

// Event listeners
export async function onAgentMessage(
  callback: (message: AgentMessage) => void
): Promise<UnlistenFn> {
  return listen<AgentMessage>('agent-message', (event) => {
    callback(event.payload);
  });
}

export async function onAgentClosed(
  callback: (agentId: string) => void
): Promise<UnlistenFn> {
  return listen<string>('agent-closed', (event) => {
    callback(event.payload);
  });
}

export async function onConfigChanged(
  callback: (config: AgentsConfig) => void
): Promise<UnlistenFn> {
  return listen<AgentsConfig>('config-changed', (event) => {
    callback(event.payload);
  });
}

export async function onAgentStderr(
  callback: (stderr: AgentStderr) => void
): Promise<UnlistenFn> {
  return listen<AgentStderr>('agent-stderr', (event) => {
    callback(event.payload);
  });
}

export async function getMachineId(): Promise<string> {
  return invoke<string>('get_machine_id');
}
