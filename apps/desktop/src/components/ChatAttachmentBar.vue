<script setup lang="ts">
import { open } from '@tauri-apps/plugin-dialog';
import { readFile } from '@tauri-apps/plugin-fs';
import type { ContentBlock } from '../lib/types';

export type PendingAttachment = {
  id: string;
  kind: 'image' | 'audio';
  label: string;
  block: ContentBlock;
};

const props = defineProps<{
  isLoading: boolean;
  supportsImage: boolean;
  supportsAudio: boolean;
  imageTooltip: string;
  audioTooltip: string;
  pendingAttachments: PendingAttachment[];
}>();

const emit = defineEmits<{
  'add-attachments': [attachments: PendingAttachment[]];
  'remove-attachment': [id: string];
  'attachment-error': [msg: string | null];
}>();

const IMAGE_MIME: Record<string, string> = {
  '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg',
  '.gif': 'image/gif', '.webp': 'image/webp', '.svg': 'image/svg+xml',
  '.bmp': 'image/bmp', '.ico': 'image/x-icon', '.tif': 'image/tiff', '.tiff': 'image/tiff',
};
const AUDIO_MIME: Record<string, string> = {
  '.mp3': 'audio/mpeg', '.wav': 'audio/wav', '.ogg': 'audio/ogg',
  '.m4a': 'audio/mp4', '.aac': 'audio/aac', '.flac': 'audio/flac',
  '.opus': 'audio/opus', '.weba': 'audio/webm',
};

function toBase64(bytes: Uint8Array): string {
  let binary = '';
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}

function fileLabel(path: string): string {
  const segs = path.replace(/\\/g, '/').split('/');
  return segs[segs.length - 1] || path;
}

function mimeType(path: string, kind: 'image' | 'audio'): string {
  const ext = path.lastIndexOf('.') >= 0 ? path.slice(path.lastIndexOf('.')).toLowerCase() : '';
  const known = kind === 'image' ? IMAGE_MIME[ext] : AUDIO_MIME[ext];
  return known ?? (kind === 'image' ? 'image/png' : 'audio/mpeg');
}

function normSelection(sel: string | string[] | null): string[] {
  if (!sel) return [];
  return Array.isArray(sel) ? sel : [sel];
}

async function addAttachments(kind: 'image' | 'audio'): Promise<void> {
  if (props.isLoading) return;
  if (kind === 'image' && !props.supportsImage) return;
  if (kind === 'audio' && !props.supportsAudio) return;
  emit('attachment-error', null);
  try {
    const sel = await open({
      multiple: true,
      title: kind === 'image' ? 'Select image attachments' : 'Select audio attachments',
      filters: [
        kind === 'image'
          ? { name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp', 'ico', 'tif', 'tiff'] }
          : { name: 'Audio', extensions: ['mp3', 'wav', 'ogg', 'm4a', 'aac', 'flac', 'opus', 'weba'] },
      ],
    });
    const paths = normSelection(sel);
    if (!paths.length) return;
    const next = await Promise.all(
      paths.map(async (path) => {
        const data = await readFile(path);
        const label = fileLabel(path);
        const mime = mimeType(path, kind);
        const block: ContentBlock =
          kind === 'image'
            ? { type: 'image', data: toBase64(data), mimeType: mime, uri: path }
            : { type: 'audio', data: toBase64(data), mimeType: mime };
        return { id: crypto.randomUUID(), kind, label, block } satisfies PendingAttachment;
      }),
    );
    emit('add-attachments', next);
  } catch (e) {
    emit('attachment-error', e instanceof Error ? e.message : `Failed to load ${kind} attachment(s).`);
  }
}
</script>

<template>
  <div class="ab-root">
    <div class="ab-toolbar">
      <span class="ab-wrap" :title="imageTooltip">
        <button type="button" class="ab-btn" :disabled="!supportsImage || isLoading" @click="addAttachments('image')">
          🖼️ Image
        </button>
      </span>
      <span class="ab-wrap" :title="audioTooltip">
        <button type="button" class="ab-btn" :disabled="!supportsAudio || isLoading" @click="addAttachments('audio')">
          🎧 Audio
        </button>
      </span>
    </div>
    <div v-if="pendingAttachments.length" class="ab-pending">
      <div v-for="att in pendingAttachments" :key="att.id" class="ab-chip">
        <span>{{ att.kind === 'image' ? '🖼️' : '🎧' }} {{ att.label }}</span>
        <button type="button" class="ab-remove" :disabled="isLoading" @click="emit('remove-attachment', att.id)">×</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ab-root { display: flex; flex-direction: column; gap: 0.5rem; }
.ab-toolbar { display: flex; flex-wrap: wrap; gap: 0.5rem; }
.ab-wrap { display: inline-flex; }
.ab-btn {
  border: 1px solid var(--border-default);
  border-radius: 999px;
  background: var(--bg-surface);
  color: var(--text-primary);
  padding: 0.35rem 0.75rem;
  font-size: var(--text-sm);
  cursor: pointer;
}
.ab-btn:disabled { opacity: 0.55; cursor: not-allowed; }
.ab-pending { display: flex; flex-wrap: wrap; gap: 0.5rem; }
.ab-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.25rem 0.625rem;
  border-radius: 999px;
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  font-size: var(--text-sm);
}
.ab-remove { border: none; background: transparent; color: var(--text-muted); cursor: pointer; font-size: var(--text-base); line-height: 1; padding: 0; }
.ab-remove:disabled { cursor: not-allowed; }
</style>
