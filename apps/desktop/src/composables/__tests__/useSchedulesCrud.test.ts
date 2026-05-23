import { computed, reactive, ref } from 'vue';
import { describe, expect, it, vi } from 'vitest';
import { createSchedulesCrud } from '@/composables/schedules/useSchedulesCrud';
import type { ScheduleForm } from '@/composables/useSchedulesSettings';

function createForm(): ScheduleForm {
  return reactive({
    id: '',
    schedule: '',
    agent: '',
    prompt: '',
    description: '',
    enabled: true,
    autoGrantPermissions: false,
    allowedTools: '',
    mcpAction: '',
    mcpParameters: '',
  });
}

describe('createSchedulesCrud', () => {
  it('saveSchedule creates a schedule with parsed allowlist and mcp parameters', async () => {
    const callTool = vi.fn((_tool: string, payload: { action: string }) => {
      if (payload.action === 'create') return Promise.resolve({ message: 'created' });
      if (payload.action === 'list') return Promise.resolve([]);
      if (payload.action === 'verify_installed') return Promise.resolve([]);
      if (payload.action === 'pause_status') return Promise.resolve({ paused: false });
      return Promise.resolve({});
    });
    const form = createForm();
    form.id = 'daily';
    form.schedule = 'every weekday at 9am';
    form.agent = 'swe';
    form.prompt = '';
    form.allowedTools = 'ReadMemory\nSearchMemories';
    form.mcpAction = 'maenifold.gate.gate_Status';
    form.mcpParameters = '{"root":"/workspace"}';

    const crud = createSchedulesCrud({
      configStore: { loading: false, loadConfig: vi.fn().mockResolvedValue(undefined) },
      agentNames: ref(['swe']),
      ensureConnected: vi.fn().mockResolvedValue(undefined),
      callTool,
      schedulesStore: { setInstallStatuses: vi.fn(), setPauseState: vi.fn() },
      jobs: ref([]),
      isLoading: ref(false),
      isSubmitting: ref(false),
      error: ref(''),
      statusMessage: ref(''),
      dialogOpen: ref(true),
      dialogMode: ref<'create' | 'edit'>('create'),
      previewIsValid: ref(true),
      form,
      schedulePlatform: { supportsEnabledSchedules: true },
      canSubmit: computed(() => true),
      parseCurrentMcpParameters: () => ({ ok: true, value: { root: '/workspace' } }),
    });

    await crud.saveSchedule();

    expect(callTool).toHaveBeenCalledWith('Schedule', {
      action: 'create',
      id: 'daily',
      schedule: 'every weekday at 9am',
      agent: 'swe',
      prompt: '',
      enabled: true,
      auto_grant_permissions: false,
      allowed_tools: ['ReadMemory', 'SearchMemories'],
      mcp_action: 'maenifold.gate.gate_Status',
      mcp_parameters: { root: '/workspace' },
    });
  });
});
