<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { resolve } from '@tauri-apps/api/path';
import { useSessionStore } from '../stores/session';
import { useMermaid } from '../composables/useMermaid';
import { isAbsolutePath } from '../lib/acp-meta';
import ModePicker from './ModePicker.vue';
import ModelPicker from './ModelPicker.vue';
import ConfigOptionPicker from './ConfigOptionPicker.vue';
import ChatInputBar from './ChatInputBar.vue';
import ChatMessageList from './ChatMessageList.vue';
import type { AvailableCommand, ContentBlock } from '../lib/types';

type PendingAttachment = { id: string; kind: 'image' | 'audio'; label: string; block: ContentBlock };
const EMBEDDED_CONTEXT_MENTION = /(^|[\s(])@(?:"([^"\n]+)"|([^\s@]+))/g;
const sessionStore = useSessionStore();
const { renderMermaidBlocks } = useMermaid();
const inputText = ref('');
const activeCommandHint = ref<string | null>(null);
const messagesContainer = ref<HTMLElement | null>(null);
const pendingAttachments = ref<PendingAttachment[]>([]);
const attachmentError = ref<string | null>(null);
const expandedThoughts = ref<Set<string>>(new Set());
const expandedToolCalls = ref<Set<string>>(new Set());
const messages = computed(() => sessionStore.messageList);
const isLoading = computed(() => sessionStore.isLoading);
const currentSession = computed(() => sessionStore.currentSession);
const availableModes = computed(() => sessionStore.availableModes);
const currentModeId = computed(() => sessionStore.currentModeId);
const availableModels = computed(() => sessionStore.availableModels);
const currentModelId = computed(() => sessionStore.currentModelId);
const configOptions = computed(() => sessionStore.configOptions);
const hasConfigOptions = computed(() => sessionStore.hasConfigOptions);
const availableCommands = computed(() => sessionStore.availableCommands);
const currentPlan = computed(() => sessionStore.currentPlan);
const pendingPermissionCount = computed(() => sessionStore.pendingPermissionCount);
const capabilities = computed(() => sessionStore.agentPromptCapabilities);
const supportsImageAttachments = computed(() => capabilities.value.image === true);
const supportsAudioAttachments = computed(() => capabilities.value.audio === true);
const supportsEmbeddedContext = computed(() => capabilities.value.embeddedContext === true);
const imageAttachmentTooltip = computed(() => supportsImageAttachments.value ? 'Attach image' : 'This agent does not support image attachments.');
const audioAttachmentTooltip = computed(() => supportsAudioAttachments.value ? 'Attach audio' : 'This agent does not support audio attachments.');
const embeddedContextTooltip = computed(() => supportsEmbeddedContext.value ? 'Type @README.md or @"notes/file name.md" to embed a file.' : 'This agent does not support embedded file context.');
const inputPlaceholder = computed(() => activeCommandHint.value ?? 'Type your message...');
const expandedThoughtIds = computed(() => Array.from(expandedThoughts.value));
const expandedToolCallIds = computed(() => Array.from(expandedToolCalls.value));
watch(messages, async () => {
  await nextTick();
  if (!messagesContainer.value) return;
  messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
  if (!isLoading.value) await renderMermaidBlocks(messagesContainer.value);
}, { deep: true });
onMounted(async () => { await nextTick(); if (messagesContainer.value) await renderMermaidBlocks(messagesContainer.value); });
watch(inputText, (text) => { if (!text.startsWith('/')) activeCommandHint.value = null; });
function setInputText(value: string): void { inputText.value = value; }

async function handleSend(): Promise<void> {
  const text = inputText.value.trim();
  if (!text || isLoading.value) return;
  attachmentError.value = null;
  try {
    const attachments = [
      ...pendingAttachments.value.map((attachment) => attachment.block),
      ...(await buildEmbeddedContextAttachments(text)),
    ];
    await sessionStore.sendPrompt(text, attachments);
    inputText.value = '';
    pendingAttachments.value = [];
  } catch (error) {
    attachmentError.value = error instanceof Error ? error.message : 'Failed to send prompt.';
    console.error('Failed to send prompt:', error);
  }
}
function handleCommandSelect(command: AvailableCommand): void {
  const meta = command._meta;
  if (meta && typeof meta === 'object' && 'clientAction' in meta) {
    const action = meta.clientAction as string;
    if (action === 'setConfigOption' && meta.configId && meta.value) {
      handleConfigOptionChange(String(meta.configId), String(meta.value));
      return;
    }
    if (action === 'setMode' && meta.modeId) {
      handleModeChange(String(meta.modeId));
      return;
    }
    if (action === 'clearMessages') {
      sessionStore.clearMessages();
      return;
    }
  }
  inputText.value = `/${command.name} `;
  activeCommandHint.value = command.input?.hint ?? null;
}
function handleCancel(): void { sessionStore.cancelOperation(); }
async function handleModeChange(modeId: string): Promise<void> {
  try { await sessionStore.setMode(modeId); } catch (error) { console.error('Failed to change mode:', error); }
}
async function handleModelChange(modelId: string): Promise<void> {
  try { await sessionStore.setModel(modelId); } catch (error) { console.error('Failed to change model:', error); }
}
async function handleConfigOptionChange(configId: string, value: string): Promise<void> {
  try { await sessionStore.setConfigOption(configId, value); } catch (error) { console.error('Failed to change config option:', error); }
}
async function rerenderExpandedMarkdown(): Promise<void> {
  await nextTick();
  if (messagesContainer.value) await renderMermaidBlocks(messagesContainer.value);
}
async function toggleThought(messageId: string): Promise<void> {
  if (expandedThoughts.value.has(messageId)) {
    expandedThoughts.value.delete(messageId);
    return;
  }
  expandedThoughts.value.add(messageId);
  await rerenderExpandedMarkdown();
}
async function toggleToolCall(toolCallId: string): Promise<void> {
  if (expandedToolCalls.value.has(toolCallId)) {
    expandedToolCalls.value.delete(toolCallId);
    return;
  }
  expandedToolCalls.value.add(toolCallId);
  await rerenderExpandedMarkdown();
}
function getFileLabel(path: string): string { const parts = path.replace(/\\/g, '/').split('/'); return parts[parts.length - 1] || path; }
function addPendingAttachments(attachments: PendingAttachment[]): void { pendingAttachments.value = [...pendingAttachments.value, ...attachments]; }
function removePendingAttachment(attachmentId: string): void { pendingAttachments.value = pendingAttachments.value.filter((item) => item.id !== attachmentId); }
function setAttachmentError(message: string | null): void { attachmentError.value = message; }
function insertEmbeddedContextHint(): void {
  if (!supportsEmbeddedContext.value || isLoading.value) return;
  const snippet = '@"path/to/file"';
  inputText.value = inputText.value.trim().length ? `${inputText.value} ${snippet}` : `${snippet} `;
}
function isPathLikeMention(candidate: string): boolean {
  return candidate.includes('/') || candidate.includes('\\') || candidate.includes('.') || candidate.startsWith('~');
}
function normalizeMentionPath(candidate: string): string { return candidate.trim().replace(/[),.;!?]+$/, ''); }

async function buildEmbeddedContextAttachments(text: string): Promise<ContentBlock[]> {
  if (!supportsEmbeddedContext.value || !currentSession.value?.cwd) {
    return [];
  }
  const attachments: ContentBlock[] = [];
  const seenPaths = new Set<string>();
  for (const match of text.matchAll(EMBEDDED_CONTEXT_MENTION)) {
    const rawCandidate = match[2] ?? match[3];
    if (!rawCandidate) continue;
    const candidate = normalizeMentionPath(rawCandidate);
    if (!candidate || !isPathLikeMention(candidate)) continue;
    const resolvedPath = isAbsolutePath(candidate) ? candidate : await resolve(currentSession.value.cwd, candidate);
    if (seenPaths.has(resolvedPath)) continue;
    try {
      attachments.push({ type: 'resource', resource: { uri: resolvedPath, text: await readTextFile(resolvedPath), mimeType: 'text/plain' } });
      seenPaths.add(resolvedPath);
    } catch (error) {
      throw new Error(`Failed to embed file context for ${getFileLabel(candidate)}: ${error instanceof Error ? error.message : 'Unable to read file'}`);
    }
  }
  return attachments;
}
</script>

<template>
  <div class="chat-view">
    <div class="chat-header">
      <h2>{{ currentSession?.title || 'Chat' }}</h2>
      <div class="header-right">
        <template v-if="hasConfigOptions">
          <ConfigOptionPicker
            v-for="option in configOptions"
            :key="option.id"
            :option="option"
            @change="handleConfigOptionChange"
          />
        </template>
        <template v-else>
          <ModelPicker v-if="availableModels.length" :models="availableModels" :current-model-id="currentModelId" @change="handleModelChange" />
          <ModePicker v-if="availableModes.length" :modes="availableModes" :current-mode-id="currentModeId" @change="handleModeChange" />
        </template>
        <span class="agent-name">{{ currentSession?.agentName }}</span>
      </div>
    </div>

    <div ref="messagesContainer" class="messages-container">
      <ChatMessageList
        :messages="messages"
        :current-plan="currentPlan"
        :expanded-thought-ids="expandedThoughtIds"
        :expanded-tool-call-ids="expandedToolCallIds"
        :pending-permission-count="pendingPermissionCount"
        :is-loading="isLoading"
        @toggle-thought="toggleThought"
        @toggle-tool-call="toggleToolCall"
        @cancel="handleCancel"
      />
    </div>

    <ChatInputBar
      :model-value="inputText"
      :placeholder="inputPlaceholder"
      :disabled="isLoading"
      :available-commands="availableCommands"
      :supports-image-attachments="supportsImageAttachments"
      :supports-audio-attachments="supportsAudioAttachments"
      :supports-embedded-context="supportsEmbeddedContext"
      :image-attachment-tooltip="imageAttachmentTooltip"
      :audio-attachment-tooltip="audioAttachmentTooltip"
      :embedded-context-tooltip="embeddedContextTooltip"
      :pending-attachments="pendingAttachments"
      :attachment-error="attachmentError"
      @update:model-value="setInputText"
      @send="handleSend"
      @select-command="handleCommandSelect"
      @add-attachments="addPendingAttachments"
      @attachment-error="setAttachmentError"
      @remove-attachment="removePendingAttachment"
      @insert-embedded-context-hint="insertEmbeddedContextHint"
    />
  </div>
</template>

<style scoped>
.chat-view { display: flex; flex-direction: column; height: 100%; }
.chat-header { display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 1rem; border-bottom: 1px solid var(--border-color); }
.chat-header h2 {
  margin: 0;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  font-size: var(--text-xl);
}
.header-right { display: flex; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: 0.75rem; }
.agent-name { font-size: var(--text-sm); color: var(--text-accent); }
.messages-container { flex: 1; overflow-y: auto; padding: 1rem; }
.messages-container :deep(.mermaid-rendered) { margin: 0.75rem 0; }
.messages-container :deep(.mermaid-rendered svg) { display: block; max-width: 100%; border-radius: 6px; }
.messages-container :deep(pre.mermaid-error) { border: 1px solid color-mix(in srgb, var(--border-error) 45%, transparent); }
.messages-container :deep(.mermaid-error-message) { margin-top: 0.5rem; color: color-mix(in srgb, var(--text-on-accent) 76%, var(--text-error)); font-size: var(--text-sm); }
</style>
