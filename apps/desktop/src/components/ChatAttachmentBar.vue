<script setup lang="ts">
import { ref } from 'vue';
import { Image as ImageIcon, Headphones } from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
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

const imageInputRef = ref<HTMLInputElement | null>(null);
const audioInputRef = ref<HTMLInputElement | null>(null);

function fileLabel(name: string): string {
  const segs = name.replace(/\\/g, '/').split('/');
  return segs[segs.length - 1] || name;
}

function mimeType(name: string, kind: 'image' | 'audio'): string {
  const ext = name.lastIndexOf('.') >= 0 ? name.slice(name.lastIndexOf('.')).toLowerCase() : '';
  const known = kind === 'image' ? IMAGE_MIME[ext] : AUDIO_MIME[ext];
  return known ?? (kind === 'image' ? 'image/png' : 'audio/mpeg');
}

function readFileAsBase64(file: File): Promise<string> {
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

function triggerPicker(kind: 'image' | 'audio'): void {
  if (props.isLoading) return;
  if (kind === 'image' && !props.supportsImage) {
    toast.info('Image attachments are not yet available for this agent.');
    return;
  }
  if (kind === 'audio' && !props.supportsAudio) {
    toast.info('Audio attachments are not yet available for this agent.');
    return;
  }
  const target = kind === 'image' ? imageInputRef.value : audioInputRef.value;
  target?.click();
}

async function handleSelection(event: Event, kind: 'image' | 'audio'): Promise<void> {
  const input = event.target as HTMLInputElement | null;
  const files = Array.from(input?.files ?? []);

  if (!files.length) {
    return;
  }

  emit('attachment-error', null);

  try {
    const next = await Promise.all(
      files.map(async (file) => {
        const label = fileLabel(file.name);
        const resolvedMimeType = file.type || mimeType(file.name, kind);
        const data = await readFileAsBase64(file);
        const block: ContentBlock =
          kind === 'image'
            ? { type: 'image', data, mimeType: resolvedMimeType, uri: file.name }
            : { type: 'audio', data, mimeType: resolvedMimeType };
        return { id: crypto.randomUUID(), kind, label, block } satisfies PendingAttachment;
      }),
    );

    emit('add-attachments', next);
  } catch (error) {
    emit(
      'attachment-error',
      error instanceof Error ? error.message : `Failed to load ${kind} attachment(s).`,
    );
  } finally {
    if (input) {
      input.value = '';
    }
  }
}
</script>

<template>
  <div class="flex flex-col gap-2">
    <component
      :is="'input'"
      ref="imageInputRef"
      class="hidden"
      type="file"
      accept=".png,.jpg,.jpeg,.gif,.webp,.svg,.bmp,.ico,.tif,.tiff,image/*"
      multiple
      @change="handleSelection($event, 'image')"
    />
    <component
      :is="'input'"
      ref="audioInputRef"
      class="hidden"
      type="file"
      accept=".mp3,.wav,.ogg,.m4a,.aac,.flac,.opus,.weba,audio/*"
      multiple
      @change="handleSelection($event, 'audio')"
    />
    <div class="flex flex-wrap gap-2" role="toolbar" aria-label="Attachments">
      <span class="inline-flex" :title="imageTooltip">
        <Button
          variant="outline"
          size="sm"
          class="border border-border-subtle rounded bg-bg-surface text-text-primary px-3 py-[0.35rem] text-sm cursor-pointer disabled:opacity-[0.55] disabled:cursor-not-allowed"
          :class="{ 'opacity-[0.55]': !supportsImage }"
          :disabled="isLoading"
          :aria-disabled="!supportsImage || isLoading"
          @click="triggerPicker('image')"
        >
          <ImageIcon class="inline size-4" :stroke-width="1.5" /> Image
        </Button>
      </span>
      <span class="inline-flex" :title="audioTooltip">
        <Button
          variant="outline"
          size="sm"
          class="border border-border-subtle rounded bg-bg-surface text-text-primary px-3 py-[0.35rem] text-sm cursor-pointer disabled:opacity-[0.55] disabled:cursor-not-allowed"
          :class="{ 'opacity-[0.55]': !supportsAudio }"
          :disabled="isLoading"
          :aria-disabled="!supportsAudio || isLoading"
          @click="triggerPicker('audio')"
        >
          <Headphones class="inline size-4" :stroke-width="1.5" /> Audio
        </Button>
      </span>
    </div>
    <div v-if="pendingAttachments.length" class="flex flex-wrap gap-2">
      <div v-for="att in pendingAttachments" :key="att.id" class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded bg-bg-surface border border-border-subtle text-sm">
        <span><component :is="att.kind === 'image' ? ImageIcon : Headphones" class="inline size-4" :stroke-width="1.5" /> {{ att.label }}</span>
        <Button variant="ghost" size="icon-xs" class="border-none bg-transparent text-text-secondary cursor-pointer text-base leading-none p-0 disabled:cursor-not-allowed" :disabled="isLoading" @click="emit('remove-attachment', att.id)">×</Button>
      </div>
    </div>
  </div>
</template>
