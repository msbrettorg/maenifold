import { toValue, type MaybeRefOrGetter } from 'vue';
import { toast } from 'vue-sonner';
import { DEFAULT_SLASH_COMMANDS } from '../lib/default-commands';
import type { AvailableCommand } from '../lib/types';
import type { AppShellTab } from '../stores/app-shell';

type SessionStoreLike = {
  clearMessages(): void;
  createSession(agentName: string, cwd: string): Promise<void>;
};

type AppShellStoreLike = {
  setActiveTab(tab: AppShellTab): void;
};

type ConfirmDialogStoreLike = {
  confirm(title: string, description?: string): Promise<boolean>;
};

type BuiltInSlashCommandDeps = {
  sessionStore: SessionStoreLike;
  appShellStore: AppShellStoreLike;
  confirmDialogStore: ConfirmDialogStoreLike;
  currentAgentName: MaybeRefOrGetter<string | null | undefined>;
  currentCwd: MaybeRefOrGetter<string | null | undefined>;
};

const BUILT_IN_COMMAND_NAMES = new Set(DEFAULT_SLASH_COMMANDS.map((command) => command.name));

export function useBuiltInSlashCommands(deps: BuiltInSlashCommandDeps) {
  function isLocallyHandled(command: AvailableCommand, agentCommands: AvailableCommand[]): boolean {
    return BUILT_IN_COMMAND_NAMES.has(command.name) &&
      !agentCommands.some((agentCommand) => agentCommand.name === command.name);
  }

  async function handleNew(): Promise<boolean> {
    const agentName = toValue(deps.currentAgentName)?.trim();
    const cwd = toValue(deps.currentCwd)?.trim();
    if (!agentName || !cwd) {
      toast.error('Cannot start a new session without an active agent and working directory.');
      return false;
    }
    await deps.sessionStore.createSession(agentName, cwd);
    return true;
  }

  async function handleQuit(): Promise<boolean> {
    const confirmed = await deps.confirmDialogStore.confirm(
      'Quit maenifold?',
      'All in-progress conversations will end.',
    );
    if (confirmed) await window.api.quit();
    return true;
  }

  async function tryHandle(command: AvailableCommand, agentCommands: AvailableCommand[]): Promise<boolean> {
    if (!isLocallyHandled(command, agentCommands)) return false;
    if (command.name === 'clear') {
      const confirmed = await deps.confirmDialogStore.confirm(
        'Clear conversation?',
        'All messages in this session will be removed.',
      );
      if (confirmed) deps.sessionStore.clearMessages();
      return true;
    }
    if (command.name === 'new') return handleNew();
    if (command.name === 'sessions') {
      deps.appShellStore.setActiveTab('home');
      return true;
    }
    if (command.name === 'quit') return handleQuit();
    return false;
  }

  return { tryHandle };
}
