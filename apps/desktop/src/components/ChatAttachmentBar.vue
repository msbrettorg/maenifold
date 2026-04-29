<script setup lang="ts">
import { ref } from 'vue';
import { Image as ImageIcon, Headphones, Paperclip } from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { fileLabel, mimeType, readFileAsBase64, type AttachmentKind } from '../lib/attachment-helpers';
import type { ContentBlock } from '../lib/types';

export type PendingAttachment = {
  id: string;
  kind: AttachmentKind;
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

const imageInputRef = ref<HTMLInputElement | null>(null);
const audioInputRef = ref<HTMLInputElement | null>(null);
const fileInputRef = ref<HTMLInputElement | null>(null);

function triggerPicker(kind: AttachmentKind): void {
  if (props.isLoading) return;
  if (kind === 'image' && !props.supportsImage) {
    toast.info('Image attachments are not yet available for this agent.');
    return;
  }
  if (kind === 'audio' && !props.supportsAudio) {
    toast.info('Audio attachments are not yet available for this agent.');
    return;
  }
  const target = kind === 'image'
    ? imageInputRef.value
    : kind === 'audio'
      ? audioInputRef.value
      : fileInputRef.value;
  target?.click();
}

async function handleSelection(event: Event, kind: AttachmentKind): Promise<void> {
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
        const block: ContentBlock = createContentBlock(kind, label, resolvedMimeType, data);
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

function createContentBlock(
  kind: AttachmentKind,
  label: string,
  resolvedMimeType: string,
  data: string,
): ContentBlock {
  if (kind === 'image') {
    return { type: 'image', data, mimeType: resolvedMimeType, uri: label };
  }
  if (kind === 'audio') {
    return { type: 'audio', data, mimeType: resolvedMimeType };
  }
  return {
    type: 'resource',
    resource: {
      uri: `file://${label}`,
      mimeType: resolvedMimeType,
      blob: data,
    },
  };
}

function iconFor(kind: AttachmentKind): typeof ImageIcon {
  if (kind === 'image') return ImageIcon;
  if (kind === 'audio') return Headphones;
  return Paperclip;
}
</script>

<template>
  <div class="flex flex-col gap-2">
    <label class="sr-only">
      Image attachment
      <input
        ref="imageInputRef"
        class="sr-only"
        type="file"
        accept=".png,.jpg,.jpeg,.gif,.webp,.svg,.bmp,.ico,.tif,.tiff,image/*"
        multiple
        @change="handleSelection($event, 'image')"
      />
    </label>
    <label class="sr-only">
      Audio attachment
      <input
        ref="audioInputRef"
        class="sr-only"
        type="file"
        accept=".mp3,.wav,.ogg,.m4a,.aac,.flac,.opus,.weba,audio/*"
        multiple
        @change="handleSelection($event, 'audio')"
      />
    </label>
    <label class="sr-only">
      File attachment
      <input
        ref="fileInputRef"
        class="sr-only"
        type="file"
        accept="*/*"
        multiple
        @change="handleSelection($event, 'file')"
      />
    </label>
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
      <span class="inline-flex" title="Attach any file">
        <Button
          variant="outline"
          size="sm"
          class="border border-border-subtle rounded bg-bg-surface text-text-primary px-3 py-[0.35rem] text-sm cursor-pointer disabled:opacity-[0.55] disabled:cursor-not-allowed"
          :disabled="isLoading"
          @click="triggerPicker('file')"
        >
          <Paperclip class="inline size-4" :stroke-width="1.5" /> File
        </Button>
      </span>
    </div>
    <div v-if="pendingAttachments.length" class="flex flex-wrap gap-2">
      <div v-for="att in pendingAttachments" :key="att.id" class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded bg-bg-surface border border-border-subtle text-sm">
        <span><component :is="iconFor(att.kind)" class="inline size-4" :stroke-width="1.5" /> {{ att.label }}</span>
        <Button variant="ghost" size="icon-xs" class="border-none bg-transparent text-text-secondary cursor-pointer text-base leading-none p-0 disabled:cursor-not-allowed" :disabled="isLoading" @click="emit('remove-attachment', att.id)">×</Button>
      </div>
    </div>
  </div>
</template>
