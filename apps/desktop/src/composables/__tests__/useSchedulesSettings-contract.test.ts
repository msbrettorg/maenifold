import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { createApp } from 'vue';
import { useSchedulesSettings } from '@/composables/useSchedulesSettings';
import { useConfigStore } from '@/stores/config';

const projectRoot = process.cwd();
const mainComposablePath = resolve(projectRoot, 'src/composables/useSchedulesSettings.ts');
const helperModulePaths = [
  'src/composables/schedules/useSchedulesValidation.ts',
  'src/composables/schedules/useSchedulesFormatting.ts',
  'src/composables/schedules/useSchedulesCrud.ts',
  'src/composables/schedules/useSchedulesLifecycle.ts',
].map((relativePath) => resolve(projectRoot, relativePath));

const callTool = vi.fn();
const connect = vi.fn();

vi.mock('@/composables/useMcpClient', () => ({
  useMcpClient: () => ({
    callTool,
    connect,
    connected: { value: true },
  }),
}));

function countLines(filePath: string): number {
  const lines = readFileSync(filePath, 'utf8').split(/\r?\n/);
  if (lines.at(-1) === '') lines.pop();
  return lines.length;
}

function seedAgents(): void {
  const config = useConfigStore();
  config.config.agents = { swe: { command: 'maenifold' } };
}

function instantiateComposable() {
  const host = document.createElement('div');
  let result!: ReturnType<typeof useSchedulesSettings>;
  const app = createApp({
    setup() {
      result = useSchedulesSettings();
      return () => null;
    },
  });
  app.use(createPinia());
  app.mount(host);
  return {
    result,
    cleanup: () => app.unmount(),
  };
}

describe('useSchedulesSettings split contract', () => {
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

  it('useSchedulesSettings.ts stays ≤300 LOC', () => {
    const lineCount = countLines(mainComposablePath);

    expect(
      lineCount,
      `useSchedulesSettings.ts is ${lineCount} LOC; T-336.8.1 split contract requires ≤300.`,
    ).toBeLessThanOrEqual(300);
  });

  it('composables/schedules/ has all 4 helper modules', () => {
    expect(helperModulePaths.every((filePath) => existsSync(filePath))).toBe(true);
  });

  it('each helper stays ≤300 LOC', () => {
    for (const filePath of helperModulePaths) {
      expect(countLines(filePath), `${filePath} exceeds 300 LOC.`).toBeLessThanOrEqual(300);
    }
  });

  it('useSchedulesSettings() returns the locked public shape', () => {
    const { result, cleanup } = instantiateComposable();

    try {
      expect(Object.keys(result).sort()).toMatchInlineSnapshot(`
        [
          "agentNames",
          "allowedToolsLabel",
          "canSubmit",
          "deleteSchedule",
          "dialogDescription",
          "dialogMode",
          "dialogOpen",
          "dialogTitle",
          "error",
          "form",
          "formatLastRun",
          "formatPausedAt",
          "installBadgeFor",
          "installStatusFor",
          "isActionInProgress",
          "isLoading",
          "isPausedAll",
          "isSubmitting",
          "jobs",
          "loadSchedules",
          "openCreateDialog",
          "openEditDialog",
          "pauseAll",
          "pauseState",
          "permissionLabel",
          "permissionVariant",
          "previewIsValid",
          "repairSchedule",
          "resumeAll",
          "runSchedule",
          "saveSchedule",
          "scheduleLabel",
          "schedulePlatform",
          "setPauseAll",
          "statusFor",
          "statusMessage",
        ]
      `);
    } finally {
      cleanup();
    }
  });
});
