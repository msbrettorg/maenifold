/**
 * Shared MCP renderer types.
 *
 * Transport implementation lives in useMcpClient via the official
 * @modelcontextprotocol/sdk StreamableHTTPClientTransport. This file remains as
 * the stable type surface consumed by stores and components.
 */

import type { McpAppUiMetadata } from '../lib/types';

// ---------------------------------------------------------------------------
// JSON-RPC notification type used by existing renderer listeners
// ---------------------------------------------------------------------------

export interface JsonRpcNotification {
  jsonrpc?: '2.0';
  method: string;
  params?: unknown;
}

// ---------------------------------------------------------------------------
// Tool-call result types
// ---------------------------------------------------------------------------

export interface ToolContent {
  type?: string;
  text?: string;
  [key: string]: unknown;
}

export interface ToolCallResult {
  content?: ToolContent[];
  structuredContent?: Record<string, unknown>;
  isError?: boolean;
  _meta?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface ToolDefinition {
  name: string;
  description?: string;
  inputSchema?: unknown;
  appUi?: McpAppUiMetadata | null;
  _meta?: Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Resource result types
// ---------------------------------------------------------------------------

export interface ResourceContent {
  uri: string;
  mimeType?: string;
  text?: string;
  blob?: string;
}

export interface ResourceDescriptor {
  uri: string;
  name?: string;
  title?: string;
  description?: string;
  mimeType?: string;
}

export interface ResourceTemplateDescriptor {
  uriTemplate: string;
  name?: string;
  title?: string;
  description?: string;
  mimeType?: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

export function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
