export type AttachmentKind = 'image' | 'audio' | 'file';

const IMAGE_MIME: Record<string, string> = {
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.webp': 'image/webp',
  '.svg': 'image/svg+xml',
  '.bmp': 'image/bmp',
  '.ico': 'image/x-icon',
  '.tif': 'image/tiff',
  '.tiff': 'image/tiff',
};

const AUDIO_MIME: Record<string, string> = {
  '.mp3': 'audio/mpeg',
  '.wav': 'audio/wav',
  '.ogg': 'audio/ogg',
  '.m4a': 'audio/mp4',
  '.aac': 'audio/aac',
  '.flac': 'audio/flac',
  '.opus': 'audio/opus',
  '.weba': 'audio/webm',
};

const FILE_MIME: Record<string, string> = {
  '.csv': 'text/csv',
  '.json': 'application/json',
  '.md': 'text/markdown',
  '.pdf': 'application/pdf',
  '.txt': 'text/plain',
  '.zip': 'application/zip',
};

export function fileLabel(name: string): string {
  const segments = name.replace(/\\/g, '/').split('/');
  return segments[segments.length - 1] || name;
}

export function mimeType(name: string, kind: AttachmentKind): string {
  const index = name.lastIndexOf('.');
  const ext = index >= 0 ? name.slice(index).toLowerCase() : '';
  if (kind === 'image') return IMAGE_MIME[ext] ?? 'image/png';
  if (kind === 'audio') return AUDIO_MIME[ext] ?? 'audio/mpeg';
  return FILE_MIME[ext] ?? 'application/octet-stream';
}

export function readFileAsBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();

    reader.onerror = () => {
      reject(reader.error ?? new Error(`Failed to read ${file.name}.`));
    };

    reader.onload = () => {
      if (typeof reader.result !== 'string') {
        reject(new Error(`Unexpected file reader result for ${file.name}.`));
        return;
      }

      const commaIndex = reader.result.indexOf(',');
      resolve(commaIndex >= 0 ? reader.result.slice(commaIndex + 1) : reader.result);
    };

    reader.readAsDataURL(file);
  });
}
