// Transport factory: chooses a concrete `AcpTransport` for the given agent
// config, applying platform restrictions (mobile and web cannot use stdio).
import type { AgentConfig } from '../types';
import { getTransportKind } from '../types';
import { restrictedTransports } from '../platform';
import type { AcpTransport } from './types';
import { WebSocketTransport } from './websocket';

/**
 * Create and connect a transport for the named agent.
 *
 * For stdio agents this spawns the local subprocess via Tauri (desktop
 * only); for remote agents it opens a WebSocket / HTTP connection from
 * the webview directly.
 *
 * @throws if the requested transport is not supported on the current
 *   platform (e.g. stdio on iOS / Android / web), or if the agent config is
 *   missing required fields.
 */
export async function createTransport(
  agentName: string,
  config: AgentConfig
): Promise<AcpTransport> {
  const kind = getTransportKind(config);

  switch (kind) {
    case 'stdio': {
      if (restrictedTransports()) {
        throw new Error(
          `Agent '${agentName}' uses stdio transport, which is not supported on this platform. Configure a websocket or http transport instead.`
        );
      }
      // Lazy-import the stdio transport so the web bundle doesn't pay for
      // it. Vite splits this into a separate chunk that's only fetched on
      // Tauri desktop.
      const { StdioTransport } = await import('./stdio');
      return StdioTransport.spawn(agentName);
    }
    case 'websocket': {
      if (!config.url) {
        throw new Error(`Agent '${agentName}' is missing 'url' for websocket transport`);
      }
      return WebSocketTransport.connect({
        url: config.url,
        headers: config.headers,
      });
    }
    case 'http': {
      throw new Error(
        `HTTP transport is not yet implemented (agent '${agentName}')`
      );
    }
    default: {
      // Exhaustiveness check.
      const _never: never = kind;
      throw new Error(`Unknown transport kind: ${String(_never)}`);
    }
  }
}

export type { AcpTransport, Unsubscribe } from './types';
export { WebSocketTransport } from './websocket';
