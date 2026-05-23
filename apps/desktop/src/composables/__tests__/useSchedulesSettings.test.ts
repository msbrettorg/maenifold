import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useSchedulesSettings } from '@/composables/useSchedulesSettings';
import { useConfigStore } from '@/stores/config';
import { useSchedulesStore } from '@/stores/schedules';

const gatewayExecute = vi.fn();
const connect = vi.fn();

vi.mock('@/composables/useMcpClient', () => ({
  useMcpClient: () => ({
    gatewayExecute,
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
    gatewayExecute.mockReset();
    connect.mockReset();
    seedAgents();
    gatewayExecute.mockImplementation((_domain: string, _command: string, payload: { action: string }) => {
      if (payload.action === 'list') return Promise.resolve([]);
      if (payload.action === 'verify_installed') return Promise.resolve([]);
      if (payload.action === 'pause_status') return Promise.resolve({ paused: false });
      return Promise.resolve({ message: `${payload.action} done` });
    });
  });

  it('pauseAll calls Schedule with action=pause_all and refreshes', async () => {
    const settings = useSchedulesSettings();

    await settings.pauseAll();

    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.system', 'Schedule', { action: 'pause_all' });
    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.system', 'Schedule', { action: 'list' });
    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.system', 'Schedule', { action: 'pause_status' });
  });

  it('resumeAll calls Schedule with action=resume_all and refreshes', async () => {
    const settings = useSchedulesSettings();

    await settings.resumeAll();

    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.system', 'Schedule', { action: 'resume_all' });
    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.system', 'Schedule', { action: 'list' });
    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.system', 'Schedule', { action: 'pause_status' });
  });

  it('loadSchedules dispatches pause_status and writes to schedulesStore', async () => {
    gatewayExecute.mockImplementation((_domain: string, _command: string, payload: { action: string }) => {
      if (payload.action === 'pause_status') {
        return Promise.resolve({ paused: true, pausedAt: '2026-05-14T00:00:00Z', schedulesPaused: 2 });
      }
      return Promise.resolve([]);
    });
    const settings = useSchedulesSettings();
    const store = useSchedulesStore();

    await settings.loadSchedules();

    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.system', 'Schedule', { action: 'pause_status' });
    expect(store.pauseState).toEqual({
      paused: true,
      pausedAt: '2026-05-14T00:00:00Z',
      schedulesPaused: 2,
    });
  });

  it('loadSchedules surfaces list errors but tolerates verify_installed failures', async () => {
    const jobs = [{
      id: 'daily',
      description: 'daily',
      schedule: '0 9 * * *',
      schedule_human: 'every day at 9am',
      agent: 'swe',
      prompt: 'Run scheduled maintenance',
      enabled: true,
      auto_grant_permissions: false,
      allowed_tools: [],
    }];
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    gatewayExecute.mockImplementation((_domain: string, _command: string, payload: { action: string }) => {
      if (payload.action === 'list') return Promise.resolve(jobs);
      if (payload.action === 'verify_installed') return Promise.reject(new Error('launchctl denied'));
      if (payload.action === 'pause_status') return Promise.resolve({ paused: false });
      return Promise.resolve({});
    });
    const settings = useSchedulesSettings();
    const store = useSchedulesStore();
    store.setInstallStatuses([{ id: 'daily', installed: true, enabledInConfig: true, discrepancy: false }]);

    try {
      await settings.loadSchedules();

      expect(settings.jobs.value).toEqual(jobs);
      expect(settings.error.value).toBe('');
      expect(store.installStatuses).toEqual({});
      expect(consoleError).toHaveBeenCalledWith(
        'Schedule verify_installed probe failed',
        expect.any(Error),
      );

      gatewayExecute.mockImplementation((_domain: string, _command: string, payload: { action: string }) => {
        if (payload.action === 'list') return Promise.reject(new Error('list unavailable'));
        return Promise.resolve({});
      });
      await settings.loadSchedules();

      expect(settings.error.value).toBe('list unavailable');
    } finally {
      consoleError.mockRestore();
    }
  });

  it('setPauseAll(true) routes to pauseAll, setPauseAll(false) routes to resumeAll', async () => {
    const settings = useSchedulesSettings();

    await settings.setPauseAll(true);
    await settings.setPauseAll(false);

    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.system', 'Schedule', { action: 'pause_all' });
    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.system', 'Schedule', { action: 'resume_all' });
  });
});

describe('useSchedulesSettings mcp action form', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    gatewayExecute.mockReset();
    connect.mockReset();
    seedAgents();
    gatewayExecute.mockImplementation((_domain: string, _command: string, payload: { action: string }) => {
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
    expect(gatewayExecute).not.toHaveBeenCalledWith('maenifold.system', 'Schedule', expect.objectContaining({ action: 'create' }));
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
    expect(gatewayExecute).not.toHaveBeenCalledWith('maenifold.system', 'Schedule', expect.objectContaining({ action: 'create' }));
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

    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.system', 'Schedule', expect.objectContaining({
      action: 'create',
      mcp_action: 'maenifold.gate.gate_Status',
      mcp_parameters: { root: '/workspace' },
    }));
  });
});
