<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useSessionStore } from '../stores/session';
import { useAgentStore } from '../stores/agent';
import { useMermaid } from '../composables/useMermaid';
import { useMcpClient } from '../composables/useMcpClient';
import { isAbsolutePath } from '../lib/acp-meta';
import ConfigOptionPicker from './ConfigOptionPicker.vue';
import SessionConfigPanel from './SessionConfigPanel.vue';
import ChatInputBar from './ChatInputBar.vue';
import ChatMessageList from './ChatMessageList.vue';
import { Button } from '@/components/ui/button';
import { ScrollArea } from './ui/scroll-area';
import { Card } from './ui/card';
import { Settings } from 'lucide-vue-next';
import type { PendingAttachment } from './ChatAttachmentBar.vue';
import type { AvailableCommand, ContentBlock } from '../lib/types';

type FileTextResponse = { path: string; content: string };
const EMBEDDED_CONTEXT_MENTION = /(^|[\s(])@(?:"([^"\n]+)"|([^\s@]+))/g;
const sessionStore = useSessionStore();
const agentStore = useAgentStore();
const { renderMermaidBlocks } = useMermaid();
const mcpClient = useMcpClient();
const inputText = ref('');
const activeCommandHint = ref<string | null>(null);
const scrollAreaRef = ref<InstanceType<typeof ScrollArea> | null>(null);
const messagesContainer = ref<HTMLElement | null>(null);
const pendingAttachments = ref<PendingAttachment[]>([]);
const attachmentError = ref<string | null>(null);
const expandedThoughts = ref<Set<string>>(new Set());
const expandedToolCalls = ref<Set<string>>(new Set());
const showConfig = ref(false);
const messages = computed(() => sessionStore.messageList);
const isLoading = computed(() => sessionStore.isLoading);
const currentSession = computed(() => sessionStore.currentSession);
const sessionCwd = computed(() => currentSession.value?.cwd ?? '/');
const isPlanMode = computed(() => sessionStore.activeSession?.currentModeId === 'plan');
const configOptions = computed(() => sessionStore.configOptions);
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
const inputPlaceholder = computed(() => activeCommandHint.value ?? (isPlanMode.value ? 'Ask a question (read-only)...' : 'Type your message...'));
const agentSessionStatus = computed(() => {
  if (
    currentSession.value?.sessionId
    && currentSession.value.sessionId === agentStore.selectedSession?.sessionId
    && agentStore.selectedSession
  ) {
    return agentStore.selectedSession.status;
  }
  return null;
});
const agentSessionStatusLabel = computed(() => {
  const status = agentSessionStatus.value;
  if (!status) return '';
  if (status === 'resumable') return 'resumable';
  if (status === 'listed') return 'listed';
  return status;
});
const expandedThoughtIds = computed(() => Array.from(expandedThoughts.value));
const expandedToolCallIds = computed(() => Array.from(expandedToolCalls.value));

function resolveScrollViewport(): HTMLElement | null {
  const root = (scrollAreaRef.value as unknown as { $el?: HTMLElement } | null)?.$el ?? null;
  if (!root) return null;
  const viewport = root.querySelector('[data-reka-scroll-area-viewport]') as HTMLElement | null;
  return viewport ?? root;
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

function handleCancel(): void { void sessionStore.cancelOperation(); }

watch(messages, async () => {
  await nextTick();
  const viewport = resolveScrollViewport();
  if (viewport) viewport.scrollTop = viewport.scrollHeight;
  if (!isLoading.value && messagesContainer.value) await renderMermaidBlocks(messagesContainer.value);
}, { deep: true });

onMounted(async () => {
  await nextTick();
  if (messagesContainer.value) await renderMermaidBlocks(messagesContainer.value);
  void agentStore.start();
});
onBeforeUnmount(() => {
  void mcpClient.disconnect();
  void agentStore.stop();
});
watch(
  () => currentSession.value?.sessionId ?? null,
  (sessionId) => {
    void agentStore.refreshSelectedSession(sessionId);
  },
  { immediate: true },
);
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
    const success = await sessionStore.sendPrompt(text, attachments);
    if (success) {
      inputText.value = '';
      pendingAttachments.value = [];
    }
  } catch (error) {
    attachmentError.value = error instanceof Error ? error.message : 'Failed to send prompt.';
  }
}
function handleCommandSelect(command: AvailableCommand): void {
  inputText.value = `/${command.name} `;
  activeCommandHint.value = command.input?.hint ?? null;
}
async function handleConfigOptionChange(configId: string, value: string): Promise<void> {
  try {
    await sessionStore.setConfigOption(configId, value);
  } catch {
    attachmentError.value = 'Failed to change config option.';
  }
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

async function fetchEmbeddedContextFile(path: string, cwd?: string): Promise<FileTextResponse> {
  if (!mcpClient.connected.value) {
    await mcpClient.connect();
  }
  return await mcpClient.callTool<FileTextResponse>('ReadTextFile', { path, cwd });
}

async function buildEmbeddedContextAttachments(text: string): Promise<ContentBlock[]> {
  if (!supportsEmbeddedContext.value) {
    return [];
  }
  const attachments: ContentBlock[] = [];
  const seenPaths = new Set<string>();
  const cwd = currentSession.value?.cwd;
  for (const match of text.matchAll(EMBEDDED_CONTEXT_MENTION)) {
    const rawCandidate = match[2] ?? match[3];
    if (!rawCandidate) continue;
    const candidate = normalizeMentionPath(rawCandidate);
    if (!candidate || !isPathLikeMention(candidate)) continue;
    if (!isAbsolutePath(candidate) && !cwd) {
      throw new Error(`Relative embedded file paths require an active working directory: ${candidate}`);
    }
    const dedupeKey = `${cwd ?? ''}:${candidate}`;
    if (seenPaths.has(dedupeKey)) continue;
    try {
      const file = await fetchEmbeddedContextFile(candidate, cwd);
      attachments.push({
        type: 'resource',
        resource: {
          uri: file.path,
          text: file.content,
          mimeType: 'text/plain',
        },
      });
      seenPaths.add(dedupeKey);
    } catch (error) {
      throw new Error(`Failed to embed file context for ${getFileLabel(candidate)}: ${error instanceof Error ? error.message : 'Unable to read file'}`);
    }
  }
  return attachments;
}
</script>

<template>
  <Card class="flex flex-col h-full rounded-none border-0 bg-bg-app shadow-none gap-0 py-0" role="region" aria-label="Chat">
    <div class="flex items-center justify-between gap-4 px-4 py-3 bg-surface border-b border-border-subtle">
      <h2 class="m-0 overflow-hidden whitespace-nowrap text-ellipsis text-md font-semibold text-text-primary">
        {{ currentSession?.title || 'Chat' }}
      </h2>
      <div class="flex flex-wrap items-center justify-end gap-3">
        <ConfigOptionPicker
          v-for="option in configOptions"
          :key="option.id"
          :option="option"
          @change="handleConfigOptionChange"
        />
        <span class="text-sm text-text-secondary">
          {{ currentSession?.agentName }}
          <span v-if="agentSessionStatusLabel">({{ agentSessionStatusLabel }})</span>
        </span>
        <Button
          variant="ghost"
          size="icon"
          class="size-7 text-text-secondary"
          :aria-label="showConfig ? 'Hide session config' : 'Show session config'"
          :aria-expanded="showConfig"
          @click="showConfig = !showConfig"
        >
          <Settings class="w-4 h-4" />
        </Button>
      </div>
    </div>

    <SessionConfigPanel
      v-if="showConfig && currentSession"
      :agent-name="currentSession.agentName"
      :cwd="sessionCwd"
      :is-read-only="true"
    />

    <div
      v-if="isPlanMode"
      class="border-b border-warning/30 bg-warning/10 px-4 py-2 text-sm font-semibold tracking-wide text-warning"
    >
      PLAN MODE — no execution will occur.
    </div>

    <ScrollArea ref="scrollAreaRef" class="flex-1 min-h-0">
      <div ref="messagesContainer" class="p-4">
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
          @prefill="setInputText"
        />
      </div>
    </ScrollArea>

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
      :accent="isPlanMode ? 'plan' : 'normal'"
      @update:model-value="setInputText"
      @send="handleSend"
      @select-command="handleCommandSelect"
      @add-attachments="addPendingAttachments"
      @attachment-error="setAttachmentError"
      @remove-attachment="removePendingAttachment"
      @insert-embedded-context-hint="insertEmbeddedContextHint"
    />
  </Card>
</template>
