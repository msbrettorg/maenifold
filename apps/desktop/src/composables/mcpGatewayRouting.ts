import type { ToolCallResult } from './mcpTransport';

export interface GatewayRoute {
  domain: string;
  command: string;
}

const gatewayPassthroughTools = new Set([
  'GetHelp',
  'ReadMcpResource',
  'GatewayIndex',
  'GatewayExecute',
  'GatewayBatch',
  'GatewayIntent',
  'DiscoverMcpTools',
  'DiscoverMcpPrompts',
  'GetMcpPrompt',
  'gate_Create',
]);

export function isGatewayPassthroughTool(name: string): boolean {
  return gatewayPassthroughTools.has(name);
}

export function gatewayExecuteArguments(route: GatewayRoute, parameters: Record<string, unknown>) {
  return { domain: route.domain, command: route.command, parameters };
}

export function parseGatewayDomains(result: ToolCallResult): Array<{ name: string }> {
  const parsed = parseGatewayJson(result, 'GatewayIndex');
  const domains = readArrayProperty(parsed, 'domains', 'GatewayIndex');
  return domains.map((domain) => readNamedEntry(domain, 'GatewayIndex domain'));
}

export function parseGatewayCommands(result: ToolCallResult, domain: string): Array<{ name: string }> {
  const parsed = parseGatewayJson(result, `GatewayIndex ${domain}`);
  const commands = readArrayProperty(parsed, 'commands', `GatewayIndex ${domain}`);
  return commands.map((command) => readNamedEntry(command, `GatewayIndex ${domain} command`));
}

function parseGatewayJson(result: ToolCallResult, label: string): unknown {
  const text = result.content?.[0]?.text;
  if (result.isError) {
    throw new Error(typeof text === 'string' && text.trim().length > 0 ? text : `${label} failed.`);
  }
  if (typeof text !== 'string' || text.trim().length === 0) {
    throw new Error(`${label} returned no text content.`);
  }
  return JSON.parse(text);
}

function readArrayProperty(value: unknown, property: string, label: string): unknown[] {
  if (!isRecord(value) || !Array.isArray(value[property])) {
    throw new Error(`${label} returned malformed ${property}.`);
  }
  return value[property];
}

function readNamedEntry(value: unknown, label: string): { name: string } {
  if (!isRecord(value) || typeof value.name !== 'string' || value.name.trim().length === 0) {
    throw new Error(`${label} is missing a name.`);
  }
  return { name: value.name };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}
