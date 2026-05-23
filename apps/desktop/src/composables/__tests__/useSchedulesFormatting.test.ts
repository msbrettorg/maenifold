import { describe, expect, it } from 'vitest';
import {
  allowedToolsLabel,
  permissionLabel,
  permissionVariant,
  scheduleLabel,
  statusFor,
} from '@/composables/schedules/useSchedulesFormatting';

describe('useSchedulesFormatting helpers', () => {
  it('returns permission labels and variants from schedule permissions', () => {
    expect(permissionLabel({ auto_grant_permissions: true })).toBe('Auto-grant on');
    expect(permissionVariant({ auto_grant_permissions: true })).toBe('destructive');
    expect(allowedToolsLabel({ auto_grant_permissions: false, allowed_tools: ['ReadMemory', 'SearchMemories'] }))
      .toBe('ReadMemory, SearchMemories');
  });

  it('derives schedule labels and run status badges', () => {
    expect(scheduleLabel({
      id: 'daily',
      schedule: '0 9 * * *',
      schedule_human: 'every day at 9am',
      agent: 'swe',
      prompt: 'Run scheduled maintenance',
      enabled: true,
      auto_grant_permissions: false,
      allowed_tools: [],
    })).toBe('every day at 9am');
    expect(statusFor({
      id: 'daily',
      schedule: '0 9 * * *',
      agent: 'swe',
      prompt: 'Run scheduled maintenance',
      enabled: true,
      auto_grant_permissions: false,
      allowed_tools: [],
      last_run: { status: 'failed', timestamp: '2026-05-14T00:00:00Z', error: 'boom' },
    })).toEqual({ label: 'Failed', variant: 'destructive' });
  });
});
