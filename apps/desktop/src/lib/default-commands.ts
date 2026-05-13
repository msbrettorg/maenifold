import type { AvailableCommand } from './types';

export const DEFAULT_SLASH_COMMANDS: AvailableCommand[] = [
  { name: 'help', description: 'Show available commands and usage hints.' },
  { name: 'clear', description: 'Clear the current conversation.' },
  { name: 'sessions', description: 'List and switch between active sessions.' },
  { name: 'new', description: 'Start a new session with the current agent.' },
  { name: 'quit', description: 'Close the current session.' },
];

export function mergeWithBuiltInSlashCommands(agentCommands: AvailableCommand[]): AvailableCommand[] {
  const firstAgentCommands = new Map<string, AvailableCommand>();
  for (const command of agentCommands) {
    if (!firstAgentCommands.has(command.name)) {
      firstAgentCommands.set(command.name, command);
    }
  }

  const builtInNames = new Set(DEFAULT_SLASH_COMMANDS.map((command) => command.name));
  const mergedBuiltIns = DEFAULT_SLASH_COMMANDS.map((command) =>
    firstAgentCommands.get(command.name) ?? command
  );
  const netNewAgentCommands = [...firstAgentCommands.values()].filter(
    (command) => !builtInNames.has(command.name)
  );

  return [...mergedBuiltIns, ...netNewAgentCommands];
}
