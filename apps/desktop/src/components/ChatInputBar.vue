<script setup lang="ts">
import { computed, ref } from 'vue';
import ChatAttachmentBar from './ChatAttachmentBar.vue';
import CommandPalette from './CommandPalette.vue';
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

function handleKeyDown(event: KeyboardEvent): void {
  if (showCommandPalette.value && commandPaletteRef.value) {
    if (['ArrowDown', 'ArrowUp', 'Enter', 'Tab', 'Escape'].includes(event.key)) {
      commandPaletteRef.value.handleKeyDown(event);
      return;
    }
  }

  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    emit('send');
  }
}

function handleCommandSelect(command: AvailableCommand): void {
  emit('select-command', command);
}

function handleClose(): void {}
</script>

<template>
  <div class="input-container">
    <CommandPalette
      ref="commandPaletteRef"
      :commands="availableCommands"
      :filter="commandFilter"
      :visible="showCommandPalette"
      @select="handleCommandSelect"
      @close="handleClose"
    />

    <div class="composer">
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

      <span class="composer-control-wrapper" :title="embeddedContextTooltip">
        <button type="button" class="attachment-btn" :disabled="!supportsEmbeddedContext || disabled" @click="emit('insert-embedded-context-hint')">
          @ File
        </button>
      </span>

      <div class="embedded-context-hint" :title="embeddedContextTooltip" :aria-disabled="!supportsEmbeddedContext">
        {{ supportsEmbeddedContext ? 'Embed file context with @README.md or @"notes/file name.md".' : 'Embedded file context is unavailable for this agent.' }}
      </div>

      <div v-if="attachmentError" class="attachment-error">{{ attachmentError }}</div>

      <textarea v-model="text" :placeholder="placeholder" :disabled="disabled" rows="3" @keydown="handleKeyDown" />
    </div>

    <button class="send-btn" :disabled="!modelValue.trim() || disabled" @click="emit('send')">Send</button>
  </div>
</template>

<style scoped>
.input-container {
  position: relative;
  display: flex;
  align-items: flex-end;
  gap: 0.5rem;
  padding: 1rem;
  border-top: 1px solid var(--border-default);
}
.composer { flex: 1; display: flex; flex-direction: column; gap: 0.5rem; }
.composer-control-wrapper { display: inline-flex; }
.attachment-btn {
  padding: 0.35rem 0.75rem;
  border: 1px solid var(--border-default);
  border-radius: 999px;
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--text-sm);
  cursor: pointer;
}
.attachment-btn:disabled,
.send-btn:disabled { cursor: not-allowed; opacity: 0.55; }
.embedded-context-hint { color: var(--text-muted); font-size: var(--text-sm); }
.embedded-context-hint[aria-disabled='true'] { opacity: 0.7; }
.attachment-error { color: var(--text-error); font-size: var(--text-sm); }
textarea {
  flex: 1;
  padding: 0.75rem;
  border: 1px solid var(--border-default);
  border-radius: 6px;
  resize: none;
  font: inherit;
  font-size: var(--text-base);
  transition: box-shadow var(--transition-fast), border-color var(--transition-fast);
}
textarea:focus {
  outline: none;
  border-color: var(--border-focus);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--border-focus) 20%, transparent);
}
.send-btn {
  padding: 0.75rem 1.5rem;
  border: none;
  border-radius: 6px;
  background: var(--border-focus);
  color: var(--text-on-accent);
  font-size: var(--text-base);
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s;
}
.send-btn:hover:not(:disabled) { background: var(--border-focus); }
</style>
