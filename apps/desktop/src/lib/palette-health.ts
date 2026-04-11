import type { McpServerDiscoveryResult, ServerHealthStatus } from './types';

const AUTH_ERROR_PATTERNS: Record<string, RegExp> = {
  'msx-mcp': /auth|login|token|credential|unauthorized|403/i,
  workiq: /eula|license|accept|consent|unauthorized|403/i,
};

export interface AuthSetupAction {
  label: string;
  description: string;
  toolName: string;
  toolArgs: Record<string, unknown>;
}

export const AUTH_SETUP_ACTIONS: Record<string, AuthSetupAction> = {
  'msx-mcp': {
    label: 'Sign in to MSX',
    description: 'Opens Azure CLI browser login for Dataverse access',
    toolName: 'msx-mcp.msx_login',
    toolArgs: {},
  },
  workiq: {
    label: 'Accept WorkIQ EULA',
    description: 'Accept the Work IQ license agreement',
    toolName: 'workiq.accept_eula',
    toolArgs: { eulaUrl: 'https://github.com/microsoft/work-iq-mcp' },
  },
};

export function deriveServerHealth(
  result: McpServerDiscoveryResult,
  hasStaleCache: boolean,
): ServerHealthStatus {
  if (result.status === 'ok') {
    return 'ready';
  }

  const authPattern = AUTH_ERROR_PATTERNS[result.server];
  if (authPattern && result.error && authPattern.test(result.error)) {
    return 'auth_required';
  }

  if (hasStaleCache) {
    return 'stale';
  }

  return 'unavailable';
}

export function getAuthSetupAction(server: string): AuthSetupAction | null {
  return AUTH_SETUP_ACTIONS[server] ?? null;
}
