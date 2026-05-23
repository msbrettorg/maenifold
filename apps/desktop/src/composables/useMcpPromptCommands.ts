import { getCurrentScope, onScopeDispose, ref, type Ref } from 'vue';
import type { GetPromptResult, Prompt, PromptArgument } from '@modelcontextprotocol/sdk/types';
import type { AvailableCommand } from '../lib/types';
import { McpCapabilityUnsupportedError } from './useMcpResources';
import { useMcpPrompts } from './useMcpPrompts';
import { getSharedMcpClient } from './usePipelineMcp';

/**
 * Maps MCP prompt lists into AvailableCommand[] entries, grouped per server.
 *
 * Today the renderer talks to a single MCP endpoint (the maenifold daemon at
 * `__default__`) which aggregates upstream MCP servers but does not surface
 * per-source identity in `prompts/list`. Callers therefore pass a single
 * `serverName` that names the visible endpoint (e.g. `'maenifold'`).
 *
 * When per-source enumeration becomes available — by extending `useMcpPrompts`
 * to query `maenifold.json` mcpServers and connect per-server — `useMcpPromptCommands`
 * will accept multiple `McpPromptCommandGroup` rows, one per source, without
 * further changes to its public API. Tracked as T-303.18.3.
 */
const MCP_PROMPT_META_KEY = 'maenifold.mcpPrompt';

export interface McpPromptCommandGroup {
  serverName: string;
  prompts: Prompt[];
}

export interface McpPromptCommandMeta {
  kind: 'mcp-prompt';
  serverName: string;
  promptName: string;
  arguments: PromptArgument[];
}

export interface McpPromptCommandsClient {
  commands: Ref<AvailableCommand[]>;
  refresh: () => Promise<void>;
}

function normalizeCommandPart(value: string): string {
  const normalized = value.trim().replace(/[^a-zA-Z0-9._-]+/g, '-').replace(/^-+|-+$/g, '');
  return normalized || 'default';
}

function promptInputHint(args: PromptArgument[]): string | undefined {
  if (args.length === 0) return undefined;
  return args.map((arg) => `<${arg.name}>`).join(' ');
}

export function toMcpPromptCommands(groups: McpPromptCommandGroup[]): AvailableCommand[] {
  return groups.flatMap((group) => {
    const serverName = normalizeCommandPart(group.serverName);
    return group.prompts.map((prompt) => {
      const args = prompt.arguments ?? [];
      const meta: McpPromptCommandMeta = {
        kind: 'mcp-prompt',
        serverName,
        promptName: prompt.name,
        arguments: args,
      };
      const hint = promptInputHint(args);

      return {
        name: `mcp.${serverName}.${prompt.name}`,
        description: prompt.description ?? `Run MCP prompt ${prompt.name} from ${serverName}.`,
        input: hint ? { hint } : undefined,
        _meta: {
          group: `MCP prompts / ${serverName}`,
          [MCP_PROMPT_META_KEY]: meta,
        },
      };
    });
  });
}

export function getMcpPromptCommandMeta(command: AvailableCommand): McpPromptCommandMeta | null {
  const meta = command._meta?.[MCP_PROMPT_META_KEY];
  if (!meta || typeof meta !== 'object') return null;
  const candidate = meta as Partial<McpPromptCommandMeta>;
  if (candidate.kind !== 'mcp-prompt') return null;
  if (typeof candidate.serverName !== 'string' || typeof candidate.promptName !== 'string') return null;
  return {
    kind: 'mcp-prompt',
    serverName: candidate.serverName,
    promptName: candidate.promptName,
    arguments: Array.isArray(candidate.arguments) ? candidate.arguments : [],
  };
}

export function hasMcpPromptArguments(command: AvailableCommand): boolean {
  return (getMcpPromptCommandMeta(command)?.arguments.length ?? 0) > 0;
}

export function isMcpPromptCommand(command: AvailableCommand): boolean {
  return command.name.startsWith('mcp.') && getMcpPromptCommandMeta(command) !== null;
}

export function parseGetPromptResult(raw: unknown): GetPromptResult {
  if (typeof raw !== 'object' || raw === null) {
    throw new Error('MCP prompt request returned non-object');
  }
  const result = raw as Partial<GetPromptResult>;
  if (!Array.isArray(result.messages)) {
    throw new Error('MCP prompt request returned malformed result (missing messages array)');
  }
  return result as GetPromptResult;
}

export function useMcpPromptCommands(options: { baseUrl?: string; serverName?: string } = {}): McpPromptCommandsClient {
  const prompts = useMcpPrompts(options.baseUrl);
  const mcpClient = getSharedMcpClient(options.baseUrl);
  const commands = ref<AvailableCommand[]>([]);

  async function refresh(): Promise<void> {
    try {
      if (!mcpClient.connected.value) await mcpClient.connect();
      const promptList = await prompts.listPrompts();
      commands.value = toMcpPromptCommands([{
        serverName: options.serverName ?? 'default',
        prompts: promptList,
      }]);
    } catch (error) {
      if (error instanceof McpCapabilityUnsupportedError) {
        commands.value = [];
        return;
      }
      console.error('Failed to refresh MCP prompt commands', error);
      commands.value = [];
    }
  }

  const cleanup = prompts.onPromptListChanged(() => {
    void refresh();
  });
  if (getCurrentScope()) {
    onScopeDispose(cleanup);
  }
  void refresh();

  return { commands, refresh };
}
