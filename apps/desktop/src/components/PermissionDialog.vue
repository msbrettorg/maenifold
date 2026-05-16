<script setup lang="ts">
// Lifted from formulahendry/acp-ui (MIT) — see LICENSE-acp-ui; Maenifold diverges below.
// https://github.com/formulahendry/acp-ui
// Adaptation notes: Maenifold adaptation: replace scoped CSS with Tailwind design tokens and shadcn-vue Button.
// Maenifold extension: queue messaging + entry details body. Upstream acp-ui PermissionDialog lacks these.

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { computed } from 'vue';
import type { PermissionRequest } from '../lib/types';

const props = withDefaults(defineProps<{
  request: PermissionRequest;
  totalPending?: number;
}>(), {
  totalPending: 1,
});

const emit = defineEmits<{
  select: [optionId: string];
  cancel: [];
}>();

function handleSelect(optionId: string) {
  emit('select', optionId);
}

function handleCancel() {
  emit('cancel');
}

function lockEscape(event: KeyboardEvent) {
  event.preventDefault();
  event.stopPropagation?.();
  event.stopImmediatePropagation?.();
}

function optionVariant(kind: string): 'default' | 'destructive' | 'outline' {
  if (kind.startsWith('allow')) return 'default';
  if (kind.startsWith('reject')) return 'destructive';
  return 'outline';
}

const isFsWriteRequest = computed(() => props.request.toolCall.kind === 'fs/write_text_file');
const fsWriteRawInput = computed(() => {
  const rawInput = props.request.toolCall.rawInput;
  return rawInput && typeof rawInput === 'object' ? rawInput as Record<string, unknown> : {};
});
const fsWritePath = computed(() => {
  const path = fsWriteRawInput.value.path;
  return typeof path === 'string' ? path : '';
});
const fsWriteContentPreview = computed(() => {
  const content = fsWriteRawInput.value.content;
  const text = typeof content === 'string' ? content : formatPayload(content);
  return text.length > 200 ? `${text.slice(0, 200)}…` : text;
});

function optionTestId(kind: string, name: string): string | undefined {
  if (!isFsWriteRequest.value) return undefined;
  if (kind === 'reject_once') return 'permission-deny-button';
  if (kind === 'allow_always' && name.toLowerCase().includes('path')) {
    return 'permission-allow-path-button';
  }
  if (kind === 'reject_always') return 'permission-deny-always-button';
  return undefined;
}

function dialogTitle(): string {
  if (props.totalPending > 1) {
    return `Permission 1 of ${props.totalPending} — ${props.request.toolCall.title}`;
  }
  return props.request.toolCall.title;
}

function formatPayload(value: unknown): string {
  if (value === undefined || value === null) return '';
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function contentBlockText(block: unknown): string {
  if (!block || typeof block !== 'object') return formatPayload(block);
  const record = block as Record<string, unknown>;
  if (typeof record.text === 'string') return record.text;
  if (typeof record.content === 'string') return record.content;
  return formatPayload(record);
}
</script>

<template>
  <Dialog :open="true" :modal="true">
    <DialogContent
      class="permission-dialog w-[90%] max-w-md overflow-hidden rounded-lg border border-border-subtle bg-bg-surface p-0 shadow-xl"
      :show-close-button="false"
      :disable-portal="true"
      :force-mount="true"
      @pointer-down-outside="(event) => event.preventDefault()"
      @interact-outside="(event) => event.preventDefault()"
      @escape-key-down="lockEscape"
    >
      <DialogHeader class="dialog-header flex items-start gap-2 border-b border-border-subtle bg-surface px-4 py-4">
        <div class="flex items-center gap-2">
          <span class="icon text-2xl">🔐</span>
          <DialogTitle class="m-0 text-lg font-semibold text-text-primary">
            {{ dialogTitle() }}
          </DialogTitle>
        </div>
        <DialogDescription
          v-if="totalPending > 1"
          class="queue-hint text-sm text-text-secondary"
        >
          {{ totalPending }} pending
        </DialogDescription>
      </DialogHeader>

      <div class="dialog-content p-4">
        <div class="tool-info flex flex-col gap-1">
          <span class="tool-title text-base font-semibold text-text-primary">{{ request.toolCall.title }}</span>
          <span class="tool-kind text-sm capitalize text-text-secondary">{{ request.toolCall.kind }}</span>
        </div>

        <div v-if="request.toolCall.locations?.length" class="locations mt-3 rounded border border-border-subtle bg-surface-high px-3 py-2">
          <div
            v-for="(loc, index) in request.toolCall.locations"
            :key="index"
            class="location break-all py-0.5 font-mono text-xs text-text-primary"
          >
            📁 {{ loc.path }}
          </div>
        </div>

        <div
          v-if="isFsWriteRequest"
          class="entry-details mt-3 max-h-[50vh] overflow-auto rounded border border-border-subtle bg-surface-high p-3 text-sm text-text-primary"
        >
          <div class="mb-3">
            <div class="mb-1 font-semibold text-text-secondary">Target path</div>
            <pre
              data-testid="fs-write-target-path"
              class="m-0 whitespace-pre-wrap break-words font-mono text-xs"
            >{{ fsWritePath }}</pre>
          </div>
          <div>
            <div class="mb-1 font-semibold text-text-secondary">Content preview</div>
            <pre
              data-testid="fs-write-content-preview"
              class="m-0 whitespace-pre-wrap break-words font-mono text-xs"
            >{{ fsWriteContentPreview }}</pre>
          </div>
        </div>

        <div v-else class="entry-details mt-3 max-h-[50vh] overflow-auto rounded border border-border-subtle bg-surface-high p-3 text-sm text-text-primary">
          <div v-if="request.toolCall.rawInput !== undefined" class="mb-3">
            <div class="mb-1 font-semibold text-text-secondary">rawInput</div>
            <pre class="m-0 whitespace-pre-wrap break-words font-mono text-xs">{{ formatPayload(request.toolCall.rawInput) }}</pre>
          </div>
          <div v-if="request.toolCall.rawOutput !== undefined" class="mb-3">
            <div class="mb-1 font-semibold text-text-secondary">rawOutput</div>
            <pre class="m-0 whitespace-pre-wrap break-words font-mono text-xs">{{ formatPayload(request.toolCall.rawOutput) }}</pre>
          </div>
          <div v-if="request.toolCall.content?.length">
            <div class="mb-1 font-semibold text-text-secondary">content</div>
            <div
              v-for="(block, index) in request.toolCall.content"
              :key="index"
              class="mb-2 rounded border border-border-subtle bg-bg-surface p-2"
            >
              <div class="mb-1 font-mono text-xs text-text-secondary">{{ block.type }}</div>
              <pre class="m-0 whitespace-pre-wrap break-words font-mono text-xs">{{ contentBlockText(block) }}</pre>
            </div>
          </div>
        </div>
      </div>

      <div class="dialog-actions flex flex-wrap gap-2 border-t border-border-subtle bg-surface px-4 py-4">
        <Button
          v-for="option in request.options"
          :key="option.optionId"
          :class="['option-btn min-w-28 flex-1', `option-${option.kind}`]"
          :variant="optionVariant(option.kind)"
          :data-testid="optionTestId(option.kind, option.name)"
          :autofocus="isFsWriteRequest && option.kind === 'reject_once'"
          @click="handleSelect(option.optionId)"
        >
          {{ option.name }}
        </Button>
        <Button class="cancel-btn min-w-28 flex-1" variant="outline" @click="handleCancel">
          Cancel
        </Button>
      </div>
    </DialogContent>
  </Dialog>
</template>
