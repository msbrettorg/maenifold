// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { DEFAULT_SLASH_COMMANDS } from '../../lib/default-commands';
import type { AvailableCommand } from '../../lib/types';
import { useBuiltInSlashCommands } from '../useBuiltInSlashCommands';

const { toastError } = vi.hoisted(() => ({ toastError: vi.fn() }));

vi.mock('vue-sonner', () => ({ toast: { error: toastError } }));

function command(name: string): AvailableCommand {
  const found = DEFAULT_SLASH_COMMANDS.find((candidate) => candidate.name === name);
  if (!found) throw new Error(`Missing built-in command: ${name}`);
  return found;
}

function setup(options: { agentName?: string | null; cwd?: string | null } = {}) {
  const quit = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(window, 'api', { value: { quit }, configurable: true });
  const sessionStore = {
    clearMessages: vi.fn(),
    createSession: vi.fn().mockResolvedValue(undefined),
  };
  const appShellStore = { setActiveTab: vi.fn() };
  const confirmDialogStore = { confirm: vi.fn().mockResolvedValue(true) };
  const builtInSlash = useBuiltInSlashCommands({
    sessionStore,
    appShellStore,
    confirmDialogStore,
    currentAgentName: options.agentName ?? 'architect',
    currentCwd: options.cwd ?? '/repo',
  });
  return { appShellStore, builtInSlash, confirmDialogStore, quit, sessionStore };
}

describe('useBuiltInSlashCommands', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    Reflect.deleteProperty(window, 'api');
  });

  it('/clear confirmed calls clearMessages and returns true', async () => {
    const { builtInSlash, sessionStore } = setup();
    await expect(builtInSlash.tryHandle(command('clear'), [])).resolves.toBe(true);
    expect(sessionStore.clearMessages).toHaveBeenCalledTimes(1);
  });

  it('/clear cancelled does not call clearMessages and returns true', async () => {
    const { builtInSlash, confirmDialogStore, sessionStore } = setup();
    confirmDialogStore.confirm.mockResolvedValue(false);
    await expect(builtInSlash.tryHandle(command('clear'), [])).resolves.toBe(true);
    expect(sessionStore.clearMessages).not.toHaveBeenCalled();
  });

  it('/new with valid agent and cwd creates a session and returns true', async () => {
    const { builtInSlash, sessionStore } = setup({ agentName: 'sonnet', cwd: '/work' });
    await expect(builtInSlash.tryHandle(command('new'), [])).resolves.toBe(true);
    expect(sessionStore.createSession).toHaveBeenCalledWith('sonnet', '/work');
  });

  it('/new with missing agent returns false and falls through', async () => {
    const { builtInSlash, sessionStore } = setup({ agentName: '' });
    await expect(builtInSlash.tryHandle(command('new'), [])).resolves.toBe(false);
    expect(sessionStore.createSession).not.toHaveBeenCalled();
    expect(toastError).toHaveBeenCalledWith(
      'Cannot start a new session without an active agent and working directory.',
      { duration: 8000 },
    );
  });

  it('/sessions activates the home tab and returns true', async () => {
    const { appShellStore, builtInSlash } = setup();
    await expect(builtInSlash.tryHandle(command('sessions'), [])).resolves.toBe(true);
    expect(appShellStore.setActiveTab).toHaveBeenCalledWith('home');
  });

  it('/quit confirmed calls window.api.quit and returns true', async () => {
    const { builtInSlash, quit } = setup();
    await expect(builtInSlash.tryHandle(command('quit'), [])).resolves.toBe(true);
    expect(quit).toHaveBeenCalledTimes(1);
  });

  it('/quit cancelled does not call quit and returns true', async () => {
    const { builtInSlash, confirmDialogStore, quit } = setup();
    confirmDialogStore.confirm.mockResolvedValue(false);
    await expect(builtInSlash.tryHandle(command('quit'), [])).resolves.toBe(true);
    expect(quit).not.toHaveBeenCalled();
  });

  it('agent-overridden /clear falls through', async () => {
    const { builtInSlash, confirmDialogStore } = setup();
    await expect(builtInSlash.tryHandle(command('clear'), [{ name: 'clear', description: 'agent clear' }])).resolves.toBe(false);
    expect(confirmDialogStore.confirm).not.toHaveBeenCalled();
  });

  it('non-built-in commands fall through', async () => {
    const { builtInSlash } = setup();
    await expect(builtInSlash.tryHandle({ name: 'foo', description: 'agent foo' }, [])).resolves.toBe(false);
  });
});
