import type { AvailableCommand } from './types';

export const DEFAULT_SLASH_COMMANDS: AvailableCommand[] = [
  { name: 'help', description: 'Show available commands and usage hints.' },
  { name: 'clear', description: 'Clear the current conversation.' },
  { name: 'sessions', description: 'List and switch between active sessions.' },
];
