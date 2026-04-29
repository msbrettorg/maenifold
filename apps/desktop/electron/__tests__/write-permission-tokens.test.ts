import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  consumeWriteToken,
  issueWriteToken,
  resetWritePermissionTokensForTest,
} from '../write-permission-tokens';

describe('write permission tokens', () => {
  beforeEach(() => {
    resetWritePermissionTokensForTest();
    vi.useRealTimers();
  });

  afterEach(() => {
    resetWritePermissionTokensForTest();
    vi.restoreAllMocks();
  });

  it('mints and consumes a token for the matching path', () => {
    const token = issueWriteToken('/tmp/foo');

    expect(consumeWriteToken(token, '/tmp/foo')).toEqual({
      ok: true,
      allowsAlways: false,
    });
  });

  it('rejects expired tokens', () => {
    vi.spyOn(Date, 'now').mockReturnValueOnce(1_000).mockReturnValueOnce(31_001);
    const token = issueWriteToken('/tmp/foo');

    expect(consumeWriteToken(token, '/tmp/foo')).toEqual({
      ok: false,
      reason: 'token expired',
    });
  });

  it('enforces single-use tokens', () => {
    const token = issueWriteToken('/tmp/foo');

    expect(consumeWriteToken(token, '/tmp/foo')).toMatchObject({ ok: true });
    expect(consumeWriteToken(token, '/tmp/foo')).toEqual({
      ok: false,
      reason: 'unknown token',
    });
  });

  it('rejects path mismatches', () => {
    const token = issueWriteToken('/tmp/foo');

    expect(consumeWriteToken(token, '/tmp/bar')).toEqual({
      ok: false,
      reason: 'path mismatch',
    });
  });
});
