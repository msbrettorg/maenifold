import { describe, expect, it, vi } from 'vitest';
import * as attachmentHelpers from '../attachment-helpers';
import { fileLabel, mimeType, readFileAsBase64 } from '../attachment-helpers';

type FileSelectionResult = {
  accepted: File[];
  rejected: { file: File; reason: string }[];
};

type PreparedAttachment = {
  text?: string;
  blob?: string;
};

const newAttachmentHelpers = attachmentHelpers as unknown as {
  validateFileSelection: (files: File[]) => FileSelectionResult;
  attachmentUri: (id: string, label: string) => string;
  prepareAttachment: (file: File) => Promise<PreparedAttachment>;
};

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

describe('attachment size caps', () => {
  const tenMb = 10 * 1024 * 1024;
  const fiftyMb = 50 * 1024 * 1024;

  function fileOfSize(name: string, size: number): File {
    return new File([new Uint8Array(size)], name, { type: 'application/octet-stream' });
  }

  it('accepts a single file under 10 MB', () => {
    const file = fileOfSize('under.bin', tenMb - 1);

    const result = newAttachmentHelpers.validateFileSelection([file]);

    expect(result.accepted).toEqual([file]);
    expect(result.rejected).toEqual([]);
  });

  it('accepts a single file exactly at the 10 MB boundary', () => {
    const file = fileOfSize('boundary.bin', tenMb);

    const result = newAttachmentHelpers.validateFileSelection([file]);

    expect(result.accepted).toEqual([file]);
    expect(result.rejected).toEqual([]);
  });

  it('rejects a single file over 10 MB with a reason', () => {
    const file = fileOfSize('oversized.bin', tenMb + 1);

    const result = newAttachmentHelpers.validateFileSelection([file]);

    expect(result.accepted).toEqual([]);
    expect(result.rejected).toHaveLength(1);
    expect(result.rejected[0]).toMatchObject({ file });
    expect(result.rejected[0].reason).toMatch(/10\s*MB|too large|size/i);
  });

  it('accepts multiple files totaling under 50 MB', () => {
    const files = [
      fileOfSize('a.bin', tenMb),
      fileOfSize('b.bin', tenMb),
      fileOfSize('c.bin', tenMb),
      fileOfSize('d.bin', tenMb),
      fileOfSize('e.bin', tenMb - 1),
    ];

    const result = newAttachmentHelpers.validateFileSelection(files);

    expect(result.accepted).toEqual(files);
    expect(result.rejected).toEqual([]);
  });

  it('rejects files when the total batch exceeds 50 MB', () => {
    const files = [
      fileOfSize('a.bin', tenMb),
      fileOfSize('b.bin', tenMb),
      fileOfSize('c.bin', tenMb),
      fileOfSize('d.bin', tenMb),
      fileOfSize('e.bin', tenMb),
      fileOfSize('f.bin', 1),
    ];

    const result = newAttachmentHelpers.validateFileSelection(files);

    expect(result.accepted.length).toBeLessThan(files.length);
    expect(result.rejected.length).toBeGreaterThan(0);
    expect(result.accepted.reduce((total, file) => total + file.size, 0)).toBeLessThanOrEqual(fiftyMb);
    expect(result.rejected.some((rejection) => rejection.reason.match(/50\s*MB|batch|total/i))).toBe(true);
  });

  it('returns no rejections for an empty file list', () => {
    const result = newAttachmentHelpers.validateFileSelection([]);

    expect(result.accepted).toEqual([]);
    expect(result.rejected).toEqual([]);
  });
});

describe('attachment URI uniqueness', () => {
  it('produces different URIs for files with the same basename when ids differ', () => {
    const first = newAttachmentHelpers.attachmentUri('first-id', 'report.pdf');
    const second = newAttachmentHelpers.attachmentUri('second-id', 'report.pdf');

    expect(first).not.toBe(second);
  });

  it('uses the file attachment URI prefix', () => {
    expect(newAttachmentHelpers.attachmentUri('id-1', 'report.pdf')).toMatch(/^file:\/\/attachment\//);
  });

  it('includes the encoded label as the final path segment', () => {
    const uri = newAttachmentHelpers.attachmentUri('id-1', 'quarterly report.pdf');

    expect(uri.split('/').at(-1)).toBe(encodeURIComponent('quarterly report.pdf'));
  });

  it('percent-encodes special characters in filenames', () => {
    const label = 'budget & forecast #1.csv';
    const uri = newAttachmentHelpers.attachmentUri('id-1', label);

    expect(uri).toContain(encodeURIComponent(label));
    expect(uri).not.toContain(label);
  });
});

describe('attachment textual blob dispatcher', () => {
  const oneMb = 1024 * 1024;

  async function expectTextAttachment(file: File): Promise<void> {
    const result = await newAttachmentHelpers.prepareAttachment(file);

    expect(result).toHaveProperty('text');
    expect(result).not.toHaveProperty('blob');
  }

  async function expectBlobAttachment(file: File): Promise<void> {
    const result = await newAttachmentHelpers.prepareAttachment(file);

    expect(result).toHaveProperty('blob');
    expect(result).not.toHaveProperty('text');
  }

  it('uses text content for a .txt file under 1 MB', async () => {
    await expectTextAttachment(new File(['hello'], 'note.txt', { type: 'text/plain' }));
  });

  it('uses text content for a .csv file under 1 MB', async () => {
    await expectTextAttachment(new File(['a,b\n1,2'], 'data.csv', { type: 'text/csv' }));
  });

  it('uses text content for a .json file under 1 MB', async () => {
    await expectTextAttachment(new File(['{"ok":true}'], 'data.json', { type: 'application/json' }));
  });

  it('uses text content for a .md file under 1 MB', async () => {
    await expectTextAttachment(new File(['# Notes'], 'notes.md', { type: 'text/markdown' }));
  });

  it('uses blob content for a .pdf file', async () => {
    await expectBlobAttachment(new File(['%PDF-1.7'], 'report.pdf', { type: 'application/pdf' }));
  });

  it('uses blob content for a .png file', async () => {
    await expectBlobAttachment(new File([new Uint8Array([137, 80, 78, 71])], 'image.png', { type: 'image/png' }));
  });

  it('uses blob content for a .txt file over 1 MB', async () => {
    const file = new File([new Uint8Array(oneMb + 1)], 'large.txt', { type: 'text/plain' });

    await expectBlobAttachment(file);
  });
});
