import { describe, expect, it, vi } from 'vitest';
import { fileLabel, mimeType, readFileAsBase64 } from '../attachment-helpers';

describe('attachment helpers', () => {
  it('reads files as base64 payloads', async () => {
    const file = new File(['hello'], 'note.txt', { type: 'text/plain' });

    await expect(readFileAsBase64(file)).resolves.toBe('aGVsbG8=');
  });

  it('rejects file reader failures', async () => {
    const OriginalFileReader = globalThis.FileReader;

    class FailingFileReader {
      error = new Error('reader failed');
      result: string | ArrayBuffer | null = null;
      onerror: (() => void) | null = null;
      onload: (() => void) | null = null;

      readAsDataURL(): void {
        this.onerror?.();
      }
    }

    vi.stubGlobal('FileReader', FailingFileReader);

    await expect(readFileAsBase64(new File(['x'], 'bad.bin'))).rejects.toThrow('reader failed');

    vi.stubGlobal('FileReader', OriginalFileReader);
  });

  it('falls back to MIME types from extensions', () => {
    expect(mimeType('PHOTO.JPG', 'image')).toBe('image/jpeg');
    expect(mimeType('track.flac', 'audio')).toBe('audio/flac');
    expect(mimeType('report.pdf', 'file')).toBe('application/pdf');
    expect(mimeType('data.csv', 'file')).toBe('text/csv');
    expect(mimeType('archive.unknown', 'file')).toBe('application/octet-stream');
  });

  it('uses the final path segment as the display label', () => {
    expect(fileLabel('C:\\fakepath\\report.pdf')).toBe('report.pdf');
    expect(fileLabel('/downloads/data.csv')).toBe('data.csv');
  });
});
