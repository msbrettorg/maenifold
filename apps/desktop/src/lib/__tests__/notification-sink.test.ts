import { beforeEach, describe, expect, it, vi } from 'vitest';

const toastFns = vi.hoisted(() => ({
  toastSuccess: vi.fn(),
  toastInfo: vi.fn(),
  toastWarning: vi.fn(),
  toastError: vi.fn(),
  toastCritical: vi.fn(),
}));

vi.mock('@/composables/useToast', () => toastFns);

describe('notification sink', () => {
  beforeEach(() => {
    vi.resetModules();
    Object.values(toastFns).forEach((fn) => fn.mockReset());
  });

  it('defaults to delegating to the toast helpers', async () => {
    const { getNotificationSink } = await import('../notification-sink');

    getNotificationSink().success('success');
    getNotificationSink().info('info');
    getNotificationSink().warning('warning');
    getNotificationSink().error('error');
    getNotificationSink().critical('critical');

    expect(toastFns.toastSuccess).toHaveBeenCalledWith('success');
    expect(toastFns.toastInfo).toHaveBeenCalledWith('info');
    expect(toastFns.toastWarning).toHaveBeenCalledWith('warning');
    expect(toastFns.toastError).toHaveBeenCalledWith('error');
    expect(toastFns.toastCritical).toHaveBeenCalledWith('critical');
  });

  it('setNotificationSink replaces the active sink', async () => {
    const { getNotificationSink, setNotificationSink } = await import('../notification-sink');
    const customSink = {
      success: vi.fn(),
      info: vi.fn(),
      warning: vi.fn(),
      error: vi.fn(),
      critical: vi.fn(),
    };

    setNotificationSink(customSink);

    expect(getNotificationSink()).toBe(customSink);
  });

  it('resetNotificationSink restores the default sink', async () => {
    const { getNotificationSink, setNotificationSink, resetNotificationSink } = await import('../notification-sink');
    const customSink = {
      success: vi.fn(),
      info: vi.fn(),
      warning: vi.fn(),
      error: vi.fn(),
      critical: vi.fn(),
    };

    setNotificationSink(customSink);
    resetNotificationSink();
    getNotificationSink().success('restored');

    expect(getNotificationSink()).not.toBe(customSink);
    expect(toastFns.toastSuccess).toHaveBeenCalledWith('restored');
  });
});
