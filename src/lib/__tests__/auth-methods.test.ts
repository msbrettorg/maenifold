import { describe, expect, it } from 'vitest';
import {
  normalizeAuthMethods,
  toSelectableAuthMethod,
} from '../auth-methods';

describe('auth method normalization', () => {
  it('preserves flat auth methods for the current auth dialog UX', () => {
    expect(
      normalizeAuthMethods([
        {
          id: 'browser',
          name: 'Browser Authentication',
          description: 'Open a browser window',
        },
      ])
    ).toEqual([
      {
        id: 'browser',
        name: 'Browser Authentication',
        description: 'Open a browser window',
      },
    ]);
  });

  it('rejects unsupported auth method variants explicitly', () => {
    expect(
      toSelectableAuthMethod({
        kind: 'oauth',
        displayName: 'OAuth',
      })
    ).toBeNull();
  });
});
