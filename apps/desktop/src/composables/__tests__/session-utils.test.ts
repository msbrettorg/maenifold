import { describe, expect, it } from 'vitest';
import {
  createDefaultSessionTitle,
  deriveSessionTitleFromFirstMessage,
  isGeneratedSessionTitle,
  normalizeConfigOptions,
} from '../session-utils';

describe('session-utils', () => {
  it('keeps boolean config options instead of dropping them', () => {
    const [option] = normalizeConfigOptions([
      {
        id: 'confirmEdits',
        name: 'Confirm edits',
        type: 'boolean',
        currentValue: true,
      },
    ]);

    expect(option).toMatchObject({
      id: 'confirmEdits',
      name: 'Confirm edits',
      type: 'select',
      currentValue: 'true',
      rawCurrentValue: true,
      valueType: 'boolean',
    });
    expect(option?.options).toEqual([
      { value: 'true', name: 'Enabled' },
      { value: 'false', name: 'Disabled' },
    ]);
  });

  it('creates sequential default session titles per agent', () => {
    expect(
      createDefaultSessionTitle('Architect', [
        {
          id: 'saved-1',
          agentName: 'Architect',
          sessionId: 'remote-1',
          title: 'Architect Session #1',
          lastUpdated: 1,
          cwd: '/repo',
        },
        {
          id: 'saved-2',
          agentName: 'Researcher',
          sessionId: 'remote-2',
          title: 'Researcher Session #1',
          lastUpdated: 2,
          cwd: '/repo',
        },
      ]),
    ).toBe('Architect Session #2');
  });

  it('derives session titles from the first forty characters of the first message', () => {
    expect(
      deriveSessionTitleFromFirstMessage('1234567890123456789012345678901234567890EXTRA'),
    ).toBe('1234567890123456789012345678901234567890');
  });

  it('recognizes generated placeholder titles', () => {
    expect(isGeneratedSessionTitle('Architect Session #3', 'Architect')).toBe(true);
    expect(isGeneratedSessionTitle('New session - 2026-04-12T19:59:29.593Z', 'Architect')).toBe(
      true,
    );
    expect(isGeneratedSessionTitle('Named investigation thread', 'Architect')).toBe(false);
  });
});
