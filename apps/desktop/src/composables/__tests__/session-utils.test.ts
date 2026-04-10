import { describe, expect, it } from 'vitest';
import { normalizeConfigOptions } from '../session-utils';

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
});
