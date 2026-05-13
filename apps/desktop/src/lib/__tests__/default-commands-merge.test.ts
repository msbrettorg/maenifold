import { describe, expect, it } from 'vitest';
import { mergeWithBuiltInSlashCommands } from '../default-commands';
import type { AvailableCommand } from '../types';

describe('mergeWithBuiltInSlashCommands', () => {
  it('returns the built-in commands in order when no agent commands are advertised', () => {
    const commands = mergeWithBuiltInSlashCommands([]);

    expect(commands.map((command) => command.name)).toEqual([
      'help',
      'clear',
      'sessions',
      'new',
      'quit',
    ]);
  });

  it('appends a net-new agent command after the built-ins', () => {
    const commands = mergeWithBuiltInSlashCommands([
      { name: 'foo', description: 'agent foo' },
    ]);

    expect(commands).toHaveLength(6);
    expect(commands.map((command) => command.name)).toEqual([
      'help',
      'clear',
      'sessions',
      'new',
      'quit',
      'foo',
    ]);
  });

  it('lets an agent command override a built-in command in place', () => {
    const commands = mergeWithBuiltInSlashCommands([
      { name: 'help', description: 'agent override' },
    ]);

    expect(commands).toHaveLength(5);
    expect(commands.map((command) => command.name)).toEqual([
      'help',
      'clear',
      'sessions',
      'new',
      'quit',
    ]);
    expect(commands[0]?.description).toBe('agent override');
  });

  it('applies built-in overrides before appending net-new agent commands', () => {
    const commands = mergeWithBuiltInSlashCommands([
      { name: 'help', description: 'override' },
      { name: 'foo', description: 'extra' },
    ]);

    expect(commands).toHaveLength(6);
    expect(commands.map((command) => command.name)).toEqual([
      'help',
      'clear',
      'sessions',
      'new',
      'quit',
      'foo',
    ]);
    expect(commands[0]?.description).toBe('override');
    expect(commands[5]?.description).toBe('extra');
  });

  it('does not mutate the input array or command objects', () => {
    const input: AvailableCommand[] = [
      { name: 'help', description: 'agent help' },
      { name: 'foo', description: 'agent foo' },
    ];
    const originalEntries = [...input];

    mergeWithBuiltInSlashCommands(input);

    expect(input).toHaveLength(2);
    expect(input).toEqual([
      { name: 'help', description: 'agent help' },
      { name: 'foo', description: 'agent foo' },
    ]);
    expect(input[0]).toBe(originalEntries[0]);
    expect(input[1]).toBe(originalEntries[1]);
  });

  it('keeps the first occurrence when an agent advertises duplicate commands', () => {
    const commands = mergeWithBuiltInSlashCommands([
      { name: 'foo', description: 'first foo' },
      { name: 'foo', description: 'second foo' },
    ]);

    expect(commands).toHaveLength(6);
    expect(commands[5]).toEqual({ name: 'foo', description: 'first foo' });
  });
});
