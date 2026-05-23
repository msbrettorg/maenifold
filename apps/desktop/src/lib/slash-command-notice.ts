import type { AvailableCommand, SessionState } from './types';
import { isCopilotSession } from '../stores/session-copilot-models';

export interface SlashCommandNotice {
  heading: string;
  body: string;
  agentName?: string;
  upstreamIssueUrl?: string;
}

export function getSlashCommandNotice(
  session: SessionState | null | undefined,
  availableCommands: AvailableCommand[],
): SlashCommandNotice | null {
  if (!isCopilotSession(session) || availableCommands.length > 0) return null;
  const agentName = session?.agentName?.trim() || undefined;
  return {
    heading: 'Slash commands unavailable',
    body: `${agentName ?? 'This agent'} did not advertise slash commands over ACP, so the palette cannot suggest any yet.`,
    agentName,
  };
}
