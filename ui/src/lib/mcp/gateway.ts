import { mcpHttpClient } from './client';
import { extractToolText, isPlainObject, type ToolCallTextResult } from '../settingsSchema';

interface GatewayEnvelope<T> {
  success?: boolean;
  data?: T;
  error?: string | null;
}

function parseJsonText<T>(text: string, label: string): T {
  try {
    return JSON.parse(text) as T;
  } catch (error) {
    throw new Error(
      `${label} returned invalid JSON: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

function unwrapGatewayEnvelope<T>(payload: unknown, label: string): T {
  if (!isPlainObject(payload) || !('success' in payload)) {
    return payload as T;
  }

  const envelope = payload as GatewayEnvelope<T>;
  if (envelope.success === false) {
    throw new Error(envelope.error ?? `${label} failed`);
  }

  return envelope.data as T;
}

export async function executeGateway<T>(
  domain: string,
  command: string,
  parameters: Record<string, unknown>
): Promise<T> {
  const result = await mcpHttpClient.callTool<ToolCallTextResult>('GatewayExecute', {
    domain,
    command,
    parameters,
  });

  const text = extractToolText(result);
  const payload = parseJsonText<unknown>(text, `GatewayExecute ${domain}/${command}`);
  return unwrapGatewayEnvelope<T>(payload, `GatewayExecute ${domain}/${command}`);
}

export async function executeGatewayText(
  domain: string,
  command: string,
  parameters: Record<string, unknown>
): Promise<string> {
  const result = await mcpHttpClient.callTool<ToolCallTextResult>('GatewayExecute', {
    domain,
    command,
    parameters,
  });

  return extractToolText(result);
}

export async function readGatewayResource<T>(uri: string): Promise<T> {
  const payload = await mcpHttpClient.readResource<unknown>(uri);
  const data = unwrapGatewayEnvelope<unknown>(payload, `ReadMcpResource ${uri}`);

  if (typeof data === 'string') {
    return parseJsonText<T>(data, `ReadMcpResource ${uri}`);
  }

  return data as T;
}
