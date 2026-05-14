import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useSchedulesSettings } from '@/composables/useSchedulesSettings';
import { useConfigStore } from '@/stores/config';
import { useSchedulesStore } from '@/stores/schedules';

const callTool = vi.fn();
const connect = vi.fn();

vi.mock('@/composables/useMcpClient', () => ({
  useMcpClient: () => ({
    callTool,
    connect,
    connected: { value: true },
  }),
}));

function seedAgents(): void {
  const config = useConfigStore();
  config.config.agents = { swe: { command: 'maenifold' } };
}

describe('useSchedulesSettings pause-all actions', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    callTool.mockReset();
    connect.mockReset();
    seedAgents();
    callTool.mockImplementation((_tool: string, payload: { action: string }) => {
      if (payload.action === 'list') return Promise.resolve([]);
      if (payload.action === 'verify_installed') return Promise.resolve([]);
      if (payload.action === 'pause_status') return Promise.resolve({ paused: false });
      return Promise.resolve({ message: `${payload.action} done` });
    });
  });

  it('pauseAll calls Schedule with action=pause_all and refreshes', async () => {
    const settings = useSchedulesSettings();

    await settings.pauseAll();

    expect(callTool).toHaveBeenCalledWith('Schedule', { action: 'pause_all' });
    expect(callTool).toHaveBeenCalledWith('Schedule', { action: 'list' });
    expect(callTool).toHaveBeenCalledWith('Schedule', { action: 'pause_status' });
  });

  it('resumeAll calls Schedule with action=resume_all and refreshes', async () => {
    const settings = useSchedulesSettings();

    await settings.resumeAll();

    expect(callTool).toHaveBeenCalledWith('Schedule', { action: 'resume_all' });
    expect(callTool).toHaveBeenCalledWith('Schedule', { action: 'list' });
    expect(callTool).toHaveBeenCalledWith('Schedule', { action: 'pause_status' });
  });

  it('loadSchedules dispatches pause_status and writes to schedulesStore', async () => {
    callTool.mockImplementation((_tool: string, payload: { action: string }) => {
      if (payload.action === 'pause_status') {
        return Promise.resolve({ paused: true, pausedAt: '2026-05-14T00:00:00Z', schedulesPaused: 2 });
      }
      return Promise.resolve([]);
    });
    const settings = useSchedulesSettings();
    const store = useSchedulesStore();

    await settings.loadSchedules();

    expect(callTool).toHaveBeenCalledWith('Schedule', { action: 'pause_status' });
    expect(store.pauseState).toEqual({
      paused: true,
      pausedAt: '2026-05-14T00:00:00Z',
      schedulesPaused: 2,
    });
  });

  it('setPauseAll(true) routes to pauseAll, setPauseAll(false) routes to resumeAll', async () => {
    const settings = useSchedulesSettings();

    await settings.setPauseAll(true);
    await settings.setPauseAll(false);

    expect(callTool).toHaveBeenCalledWith('Schedule', { action: 'pause_all' });
    expect(callTool).toHaveBeenCalledWith('Schedule', { action: 'resume_all' });
  });
});

describe('useSchedulesSettings mcp action form', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    callTool.mockReset();
    connect.mockReset();
    seedAgents();
    callTool.mockImplementation((_tool: string, payload: { action: string }) => {
      if (payload.action === 'list') return Promise.resolve([]);
      if (payload.action === 'verify_installed') return Promise.resolve([]);
      if (payload.action === 'pause_status') return Promise.resolve({ paused: false });
      return Promise.resolve({ message: `${payload.action} done` });
    });
  });

  it('composable normalizes mcp_action and mcp_parameters from job payload', () => {
    const settings = useSchedulesSettings();

    settings.openEditDialog({
      id: 'mcp-job',
      description: 'MCP job',
      schedule: '0 9 * * *',
      agent: 'swe',
      prompt: '',
      enabled: false,
      auto_grant_permissions: false,
      allowed_tools: [],
      mcp_action: 'maenifold.gate.gate_Status',
      mcp_parameters: { root: '/workspace' },
    });

    expect(settings.form.mcpAction).toBe('maenifold.gate.gate_Status');
    expect(settings.form.mcpParameters).toBe(JSON.stringify({ root: '/workspace' }, null, 2));
  });

  it('composable rejects save when mcpAction set but parameters JSON parse fails', async () => {
    const settings = useSchedulesSettings();
    settings.openCreateDialog();
    settings.form.id = 'bad-json';
    settings.form.schedule = 'every weekday at 9am';
    settings.form.agent = 'swe';
    settings.form.prompt = '';
    settings.form.mcpAction = 'maenifold.gate.gate_Status';
    settings.form.mcpParameters = '{';
    settings.previewIsValid.value = true;

    await settings.saveSchedule();

    expect(settings.error.value).toBe('Parameters must be a JSON object.');
    expect(callTool).not.toHaveBeenCalledWith('Schedule', expect.objectContaining({ action: 'create' }));
  });

  it('composable rejects save when mcpParameters parses to non-object', async () => {
    const settings = useSchedulesSettings();
    settings.openCreateDialog();
    settings.form.id = 'array-json';
    settings.form.schedule = 'every weekday at 9am';
    settings.form.agent = 'swe';
    settings.form.prompt = '';
    settings.form.mcpAction = 'maenifold.gate.gate_Status';
    settings.form.mcpParameters = '[1,2,3]';
    settings.previewIsValid.value = true;

    await settings.saveSchedule();

    expect(settings.error.value).toBe('Parameters must be a JSON object.');
    expect(callTool).not.toHaveBeenCalledWith('Schedule', expect.objectContaining({ action: 'create' }));
  });

  it('composable allows save when mcpAction set and parameters parse OK', async () => {
    const settings = useSchedulesSettings();
    settings.openCreateDialog();
    settings.form.id = 'ok-json';
    settings.form.schedule = 'every weekday at 9am';
    settings.form.agent = 'swe';
    settings.form.prompt = '';
    settings.form.mcpAction = 'maenifold.gate.gate_Status';
    settings.form.mcpParameters = '{"root":"/workspace"}';
    settings.previewIsValid.value = true;

    await settings.saveSchedule();

    expect(callTool).toHaveBeenCalledWith('Schedule', expect.objectContaining({
      action: 'create',
      mcp_action: 'maenifold.gate.gate_Status',
      mcp_parameters: { root: '/workspace' },
    }));
  });
});
