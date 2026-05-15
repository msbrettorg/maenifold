import { describe, expect, it } from 'vitest';
import { sanitizeResourceErrorMessage } from '../error-sanitizer';

describe('sanitizeResourceErrorMessage', () => {
  it('maps HTTP-style resource errors to friendly messages', () => {
    expect(sanitizeResourceErrorMessage('backend returned 404 for resource')).toBe('Resource not found');
    expect(sanitizeResourceErrorMessage('HTTP 403 while reading resource')).toBe('Permission denied');
    expect(sanitizeResourceErrorMessage('request failed with status 500')).toBe('Resource server error');
  });

  it('redacts absolute paths before display', () => {
    const message =
      'permission denied reading /Users/brett/private/secret.md, C:\\Users\\bob\\file.txt, and file:///etc/passwd';

    const sanitized = sanitizeResourceErrorMessage(message);

    expect(sanitized).toBe('permission denied reading <redacted>, <redacted>, and <redacted>');
    expect(sanitized).not.toContain('/Users/brett/private/secret.md');
    expect(sanitized).not.toContain('C:\\Users\\bob\\file.txt');
    expect(sanitized).not.toContain('file:///etc/passwd');
  });
});
