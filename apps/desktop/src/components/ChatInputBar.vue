<script setup lang="ts">
import { toastInfo } from '@/composables/useToast';
import { computed, ref, watch } from 'vue';
import { Square } from 'lucide-vue-next';
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import ChatAttachmentBar from './ChatAttachmentBar.vue';
import CommandPalette from './CommandPalette.vue';
import VoiceInputButton from './VoiceInputButton.vue';
import { useChatShortcuts } from '../composables/useChatShortcuts';
import { useFileDrop } from '../composables/useFileDrop';
import { fileLabel, mimeType, readFileAsBase64, type AttachmentKind } from '../lib/attachment-helpers';
import { useAppShellStore } from '../stores/app-shell';
import type { PendingAttachment } from './ChatAttachmentBar.vue';
import type { AvailableCommand, ContentBlock } from '../lib/types';
import { mergeWithBuiltInSlashCommands } from '../lib/default-commands';

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
  pendingAttachments: PendingAttachment[];
  attachmentError: string | null;
  accent?: 'normal' | 'plan';
  isStreaming?: boolean;
  agentConnected?: boolean;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: string];
  send: [];
  'select-command': [command: AvailableCommand];
  'select-mcp-prompt': [command: AvailableCommand];
  'remove-attachment': [attachmentId: string];
  'insert-embedded-context-hint': [];
  'add-attachments': [attachments: PendingAttachment[]];
  'attachment-error': [message: string | null];
  'recall-last': [];
  'cancel-turn': [];
  'clear-chat': [];
}>();

const commandPaletteRef = ref<InstanceType<typeof CommandPalette> | null>(null);
const clearDialogOpen = ref(false);
const commandPaletteForcedOpen = ref(false);
const appShell = useAppShellStore();
const text = computed({
  get: () => props.modelValue,
  set: (value: string) => emit('update:modelValue', value),
});
const effectiveCommands = computed<AvailableCommand[]>(() =>
  mergeWithBuiltInSlashCommands(props.availableCommands)
);
const showCommandPalette = computed(() => {
  if (!effectiveCommands.value.length) {
    return false;
  }
  if (commandPaletteForcedOpen.value) return true;
  if (!props.modelValue.startsWith('/')) return false;
  return props.modelValue.indexOf(' ') === -1;
});
const commandFilter = computed(() => props.modelValue.startsWith('/') ? props.modelValue.slice(1) : '');

function submit(): void {
  if (props.disabled || !props.modelValue.trim()) {
    return;
  }
  emit('send');
}

const { handleShortcut } = useChatShortcuts({ submit, recallLast: () => emit('recall-last'), cancelTurn: () => emit('cancel-turn'), requestClear: () => { clearDialogOpen.value = true; }, isStreaming: computed(() => props.isStreaming === true), isInputEmpty: computed(() => !props.modelValue.trim()) });

function confirmClearChat(): void { clearDialogOpen.value = false; emit('clear-chat'); }

watch(() => appShell.commandPaletteOpen, (open) => {
  if (!open) return;
  appShell.commandPaletteOpen = false;
  if (!props.modelValue.trim()) {
    commandPaletteForcedOpen.value = false;
    emit('update:modelValue', '/');
    return;
  }
  commandPaletteForcedOpen.value = true;
});

function handleKeyDown(event: KeyboardEvent): void {
  if (event.isComposing || event.keyCode === 229) return;
  if (handleShortcut(event)) return;
  if (showCommandPalette.value) {
    if (event.key === 'Escape') {
      event.preventDefault();
      if (commandPaletteForcedOpen.value) {
        commandPaletteForcedOpen.value = false;
      } else {
        emit('update:modelValue', '');
      }
      return;
    }
    if (commandPaletteRef.value && ['ArrowDown', 'ArrowUp', 'Enter', 'Tab'].includes(event.key)) {
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
  commandPaletteForcedOpen.value = false;
  if (command.name.startsWith('mcp.')) {
    emit('select-mcp-prompt', command);
    return;
  }
  emit('select-command', command);
}

function handleEmbeddedContextClick(): void {
  if (props.disabled) return;
  if (!props.supportsEmbeddedContext) {
    toastInfo('Embedded file context is not yet available for this agent.');
    return;
  }
  emit('insert-embedded-context-hint');
}

function kindForFile(file: File): AttachmentKind {
  if (file.type.startsWith('image/')) return 'image';
  if (file.type.startsWith('audio/')) return 'audio';
  return 'file';
}

function createContentBlock(kind: AttachmentKind, label: string, resolvedMimeType: string, data: string): ContentBlock {
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

async function processFiles(files: File[]): Promise<void> {
  if (props.disabled || !files.length) {
    return;
  }

  emit('attachment-error', null);

  try {
    const attachments = await Promise.all(files.map(async (file) => {
      let kind = kindForFile(file);
      if ((kind === 'image' && !props.supportsImageAttachments) || (kind === 'audio' && !props.supportsAudioAttachments)) kind = 'file';
      const label = fileLabel(file.name);
      const resolvedMimeType = file.type || mimeType(file.name, kind);
      const data = await readFileAsBase64(file);
      const block = createContentBlock(kind, label, resolvedMimeType, data);
      return { id: crypto.randomUUID(), kind, label, block } satisfies PendingAttachment;
    }));
    emit('add-attachments', attachments);
  } catch (error) {
    emit(
      'attachment-error',
      error instanceof Error ? error.message : 'Failed to load attachment(s).',
    );
  }
}

const { dropZoneActive, handleDragOver, handleDragLeave, handleDrop, handlePaste } = useFileDrop({
  onFiles: processFiles,
  enabled: () => !props.disabled,
});

function handleClose(): void { commandPaletteForcedOpen.value = false; }
</script>

<template>
  <div
    class="relative flex items-end gap-2 px-4 py-3 bg-surface"
    :class="{ 'border-t border-warning/30 bg-warning/10': accent === 'plan' }"
    @dragover="handleDragOver"
    @dragleave="handleDragLeave"
    @drop="handleDrop"
  >
    <div
      v-if="dropZoneActive"
      class="absolute inset-0 z-10 flex items-center justify-center bg-brand/10 border-2 border-dashed border-brand rounded pointer-events-none"
    >
      <span class="rounded bg-surface-high px-3 py-2 text-sm font-medium text-text-primary shadow-sm">Drop files to attach</span>
    </div>

    <CommandPalette
      ref="commandPaletteRef"
      :commands="effectiveCommands"
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
        :class="{ 'border-warning/40 focus:border-warning': accent === 'plan' }"
        @keydown="handleKeyDown"
        @paste="handlePaste"
      />
    </div>

    <VoiceInputButton />

    <TooltipProvider v-if="disabled" :delay-duration="150"><Tooltip><TooltipTrigger as-child>
      <Button type="button" variant="ghost" size="icon" class="size-9 text-text-secondary" title="Cancel turn (Esc)" aria-label="Cancel turn (Esc)" @click="emit('cancel-turn')">
        <Square class="size-4 fill-current" aria-hidden="true" />
      </Button>
    </TooltipTrigger><TooltipContent>Cancel turn (Esc)</TooltipContent></Tooltip></TooltipProvider>

    <Button
      class="px-6 py-3 border-none rounded bg-brand text-text-primary text-base font-medium cursor-pointer transition-colors duration-150 hover:not-disabled:bg-brand-hover disabled:cursor-not-allowed disabled:opacity-[0.55]"
      :disabled="!modelValue.trim() || disabled"
      @click="submit"
    >
      <span
        v-if="agentConnected !== undefined"
        class="mr-2 inline-block size-2 rounded-full animate-pulse"
        :class="agentConnected ? 'bg-success' : 'bg-danger'"
        :aria-label="agentConnected ? 'Agent connected' : 'Agent connection lost'"
      ></span>
      Send
    </Button>
    <AlertDialog :open="clearDialogOpen" @update:open="(value: boolean) => { clearDialogOpen = value; }">
      <AlertDialogContent class="sm:max-w-[28rem]"><AlertDialogHeader>
        <AlertDialogTitle>Clear conversation?</AlertDialogTitle>
        <AlertDialogDescription>This will remove all messages from this session. This cannot be undone.</AlertDialogDescription>
      </AlertDialogHeader><AlertDialogFooter>
        <AlertDialogAction variant="destructive" @click="confirmClearChat">Clear</AlertDialogAction><AlertDialogCancel>Cancel</AlertDialogCancel>
      </AlertDialogFooter></AlertDialogContent>
    </AlertDialog>
  </div>
</template>
