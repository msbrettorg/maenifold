<script setup lang="ts">
import { computed, ref } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import ChatAttachmentBar from './ChatAttachmentBar.vue';
import CommandPalette from './CommandPalette.vue';
import VoiceInputButton from './VoiceInputButton.vue';
import type { AvailableCommand, ContentBlock } from '../lib/types';

type PendingAttachmentView = {
  id: string;
  kind: 'image' | 'audio';
  label: string;
  block: ContentBlock;
};

const props = defineProps<{
  modelValue: string;
  placeholder: string;
  disabled: boolean;
  availableCommands: AvailableCommand[];
  supportsImageAttachments: boolean;
  supportsAudioAttachments: boolean;
  supportsEmbeddedContext: boolean;
  imageAttachmentTooltip: string;
  audioAttachmentTooltip: string;
  embeddedContextTooltip: string;
  pendingAttachments: PendingAttachmentView[];
  attachmentError: string | null;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: string];
  send: [];
  'select-command': [command: AvailableCommand];
  'remove-attachment': [attachmentId: string];
  'insert-embedded-context-hint': [];
  'add-attachments': [attachments: PendingAttachmentView[]];
  'attachment-error': [message: string | null];
}>();

const commandPaletteRef = ref<InstanceType<typeof CommandPalette> | null>(null);
const text = computed({
  get: () => props.modelValue,
  set: (value: string) => emit('update:modelValue', value),
});
const showCommandPalette = computed(() => {
  if (!props.availableCommands.length || !props.modelValue.startsWith('/')) {
    return false;
  }
  return props.modelValue.indexOf(' ') === -1;
});
const commandFilter = computed(() => props.modelValue.startsWith('/') ? props.modelValue.slice(1) : '');

function submit(): void {
  if (props.disabled || !props.modelValue.trim()) {
    return;
  }
  emit('send');
}

function handleKeyDown(event: KeyboardEvent): void {
  if (showCommandPalette.value && commandPaletteRef.value) {
    if (['ArrowDown', 'ArrowUp', 'Enter', 'Tab', 'Escape'].includes(event.key)) {
      commandPaletteRef.value.handleKeyDown(event);
      return;
    }
  }

  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    submit();
  }
}

function handleCommandSelect(command: AvailableCommand): void {
  emit('select-command', command);
}

function handleEmbeddedContextClick(): void {
  if (props.disabled) return;
  if (!props.supportsEmbeddedContext) {
    toast.info('Embedded file context is not yet available for this agent.');
    return;
  }
  emit('insert-embedded-context-hint');
}

function handleClose(): void {}
</script>

<template>
  <div class="relative flex items-end gap-2 px-4 py-3 bg-surface">
    <CommandPalette
      ref="commandPaletteRef"
      :commands="availableCommands"
      :filter="commandFilter"
      :visible="showCommandPalette"
      @select="handleCommandSelect"
      @close="handleClose"
    />

    <div class="flex-1 flex flex-col gap-2">
      <ChatAttachmentBar
        :is-loading="disabled"
        :supports-image="supportsImageAttachments"
        :supports-audio="supportsAudioAttachments"
        :image-tooltip="imageAttachmentTooltip"
        :audio-tooltip="audioAttachmentTooltip"
        :pending-attachments="pendingAttachments"
        @add-attachments="emit('add-attachments', $event)"
        @remove-attachment="emit('remove-attachment', $event)"
        @attachment-error="emit('attachment-error', $event)"
      />

      <span class="inline-flex" :title="embeddedContextTooltip">
        <Button
          variant="outline"
          size="sm"
          class="px-3 py-[0.35rem] border border-border-subtle rounded bg-surface-high text-text-primary text-sm cursor-pointer disabled:cursor-not-allowed disabled:opacity-[0.55]"
          :class="{ 'opacity-[0.55]': !supportsEmbeddedContext }"
          :disabled="disabled"
          :aria-disabled="!supportsEmbeddedContext || disabled"
          @click="handleEmbeddedContextClick"
        >
          @ File
        </Button>
      </span>

      <div
        class="text-text-secondary text-sm aria-disabled:opacity-70"
        :title="embeddedContextTooltip"
        :aria-disabled="!supportsEmbeddedContext"
      >
        {{ supportsEmbeddedContext ? 'Embed file context with @README.md or @"notes/file name.md".' : 'Embedded file context is unavailable for this agent.' }}
      </div>

      <div v-if="attachmentError" class="text-danger text-sm">{{ attachmentError }}</div>

      <Textarea
        v-model="text"
        :placeholder="placeholder"
        :disabled="disabled"
        rows="3"
        aria-label="Message input"
        class="flex-1 p-3 border border-border-subtle rounded bg-surface text-text-primary resize-none font-[inherit] text-base transition-[border-color,box-shadow] duration-150 placeholder:text-text-secondary focus:outline-none focus:border-brand focus:shadow-none"
        @keydown="handleKeyDown"
      />
    </div>

    <VoiceInputButton />

    <Button
      class="px-6 py-3 border-none rounded bg-brand text-text-primary text-base font-medium cursor-pointer transition-colors duration-150 hover:not-disabled:bg-brand-hover disabled:cursor-not-allowed disabled:opacity-[0.55]"
      :disabled="!modelValue.trim() || disabled"
      @click="submit"
    >Send</Button>
  </div>
</template>
