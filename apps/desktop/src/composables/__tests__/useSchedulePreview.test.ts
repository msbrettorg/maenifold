// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { useSchedulePreview } from '../useSchedulePreview';

const gatewayExecute = vi.fn();
const connect = vi.fn();
const connected = { value: true };

vi.mock('@/composables/useMcpClient', () => ({
  useMcpClient: () => ({
    gatewayExecute,
    connect,
    connected,
  }),
}));

describe('useSchedulePreview', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    gatewayExecute.mockReset();
    connect.mockReset();
    connected.value = true;
    gatewayExecute.mockResolvedValue({
      action: 'preview',
      cron: '0 9 * * 1-5',
      description: 'At 09:00 AM, Monday through Friday',
      nextFires: ['2026-04-29T14:00:00Z'],
      timezone: 'UTC',
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('debounces preview calls', async () => {
    const expression = ref('every weekday at 9am');
    const preview = useSchedulePreview(expression);

    await vi.advanceTimersByTimeAsync(299);
    expect(gatewayExecute).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(1);

    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.system', 'Schedule', {
      action: 'preview',
      schedule: 'every weekday at 9am',
    });
    expect(preview.cron.value).toBe('0 9 * * 1-5');
    expect(preview.previewIsValid.value).toBe(true);
  });

  it('keeps only the latest debounced expression', async () => {
    const expression = ref('daily at 9am');
    useSchedulePreview(expression);
    expression.value = 'daily at 10am';

    await vi.advanceTimersByTimeAsync(300);

    expect(gatewayExecute).toHaveBeenCalledTimes(1);
    expect(gatewayExecute).toHaveBeenCalledWith('maenifold.system', 'Schedule', {
      action: 'preview',
      schedule: 'daily at 10am',
    });
  });

  it('surfaces preview errors', async () => {
    gatewayExecute.mockRejectedValue(new Error('invalid cron'));
    const expression = ref('bad schedule');
    const preview = useSchedulePreview(expression);

    await vi.advanceTimersByTimeAsync(300);

    expect(preview.error.value).toBe('invalid cron');
    expect(preview.nextFires.value).toEqual([]);
    expect(preview.previewIsValid.value).toBe(false);
  });
});
