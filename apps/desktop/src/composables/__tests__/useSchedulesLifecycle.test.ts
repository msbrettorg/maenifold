import { ref } from 'vue';
import { describe, expect, it, vi } from 'vitest';
import { createSchedulesLifecycle } from '@/composables/schedules/useSchedulesLifecycle';

describe('createSchedulesLifecycle', () => {
  it('deleteSchedule confirms, deletes, and refreshes schedules', async () => {
    const callTool = vi.fn().mockResolvedValue({ message: 'deleted' });
    const loadSchedules = vi.fn().mockResolvedValue(undefined);
    const lifecycle = createSchedulesLifecycle({
      ensureConnected: vi.fn().mockResolvedValue(undefined),
      callTool,
      confirmDialogStore: { confirm: vi.fn().mockResolvedValue(true) },
      actionJobIds: ref(new Set<string>()),
      isSubmitting: ref(false),
      error: ref(''),
      statusMessage: ref(''),
      loadSchedules,
    });

    await lifecycle.deleteSchedule({
      id: 'daily',
      description: 'daily',
      schedule: '0 9 * * *',
      agent: 'swe',
      prompt: 'Run scheduled maintenance',
      enabled: true,
      auto_grant_permissions: false,
      allowed_tools: [],
    });

    expect(callTool).toHaveBeenCalledWith('Schedule', { action: 'delete', id: 'daily' });
    expect(loadSchedules).toHaveBeenCalled();
  });
});
