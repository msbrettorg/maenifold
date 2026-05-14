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
