<script setup lang="ts">
import { ref, computed, nextTick, onMounted, watch } from 'vue';
import { marked } from 'marked';
import { useSessionStore } from '../stores/session';
import { useMermaid } from '../composables/useMermaid';
import ModePicker from './ModePicker.vue';
import ModelPicker from './ModelPicker.vue';
import ConfigOptionPicker from './ConfigOptionPicker.vue';
import CommandPalette from './CommandPalette.vue';
import PlanView from './PlanView.vue';
import type { AvailableCommand, ChatMessage, ContentBlock } from '../lib/types';

const sessionStore = useSessionStore();
const inputText = ref('');
const activeCommandHint = ref<string | null>(null);
const messagesContainer = ref<HTMLElement | null>(null);
const commandPaletteRef = ref<InstanceType<typeof CommandPalette> | null>(null);
const { renderMermaidBlocks } = useMermaid();

// Track expanded thought sections by message id
const expandedThoughts = ref<Set<string>>(new Set());

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

// Slash command state
const showCommandPalette = computed(() => {
  if (availableCommands.value.length === 0) return false;
  const text = inputText.value;
  // Show palette when input starts with "/" and cursor is after it
  if (!text.startsWith('/')) return false;
  // Don't show if there's a space (command already entered)
  const spaceIndex = text.indexOf(' ');
  return spaceIndex === -1;
});

const commandFilter = computed(() => {
  if (!inputText.value.startsWith('/')) return '';
  return inputText.value.slice(1); // Remove the leading "/"
});

// Auto-scroll to bottom when new messages arrive
watch(messages, async () => {
  await nextTick();
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
    await renderMermaidBlocks(messagesContainer.value);
  }
}, { deep: true });

onMounted(async () => {
  await nextTick();
  if (messagesContainer.value) {
    await renderMermaidBlocks(messagesContainer.value);
  }
});

watch(inputText, (text) => {
  if (!text.startsWith('/')) {
    activeCommandHint.value = null;
  }
});

async function handleSend() {
  const text = inputText.value.trim();
  if (!text || isLoading.value) return;
  
  inputText.value = '';
  try {
    await sessionStore.sendPrompt(text);
  } catch (e) {
    console.error('Failed to send prompt:', e);
  }
}

function handleKeyDown(event: KeyboardEvent) {
  // Let CommandPalette handle navigation keys when visible
  if (showCommandPalette.value && commandPaletteRef.value) {
    if (['ArrowDown', 'ArrowUp', 'Enter', 'Tab', 'Escape'].includes(event.key)) {
      commandPaletteRef.value.handleKeyDown(event);
      return;
    }
  }
  
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    handleSend();
  }
}

function handleCommandSelect(command: AvailableCommand) {
  inputText.value = `/${command.name} `;
  activeCommandHint.value = command.input?.hint ?? null;
}

function handleCommandClose() {
  // Just dismiss, keep the text
}

function handleCancel() {
  sessionStore.cancelOperation();
}

async function handleModeChange(modeId: string) {
  try {
    await sessionStore.setMode(modeId);
  } catch (e) {
    console.error('Failed to change mode:', e);
  }
}

async function handleModelChange(modelId: string) {
  try {
    await sessionStore.setModel(modelId);
  } catch (e) {
    console.error('Failed to change model:', e);
  }
}

async function handleConfigOptionChange(configId: string, value: string) {
  try {
    await sessionStore.setConfigOption(configId, value);
  } catch (e) {
    console.error('Failed to change config option:', e);
  }
}

function isThoughtExpanded(messageId: string): boolean {
  return expandedThoughts.value.has(messageId);
}

function toggleThought(messageId: string): void {
  if (expandedThoughts.value.has(messageId)) {
    expandedThoughts.value.delete(messageId);
  } else {
    expandedThoughts.value.add(messageId);
  }
}

function renderMarkdown(content: string): string {
  return marked.parse(content, { async: false }) as string;
}

function getToolIcon(kind: string): string {
  switch (kind) {
    case 'read': return '📖';
    case 'edit': return '✏️';
    case 'delete': return '🗑️';
    case 'move': return '📦';
    case 'search': return '🔍';
    case 'execute': return '▶️';
    case 'think': return '💭';
    case 'fetch': return '🌐';
    default: return '🔧';
  }
}

function getStatusIcon(status: string): string {
  switch (status) {
    case 'pending': return '⏳';
    case 'in_progress': return '⚙️';
    case 'completed': return '✓';
    case 'failed': return '✗';
    default: return '';
  }
}

function getMessageToolCalls(message: ChatMessage) {
  return message.toolCalls ?? [];
}

function getMessageBlocks(message: ChatMessage): ContentBlock[] {
  return message.blocks ?? [];
}

function shouldRenderLegacyContent(message: ChatMessage): boolean {
  return Boolean(message.content) && !getMessageBlocks(message).some((block) => block.type === 'text');
}

function getImageSrc(block: ContentBlock): string {
  return block.type === 'image' ? `data:${block.mimeType};base64,${block.data}` : '';
}

function getAudioSrc(block: ContentBlock): string {
  return block.type === 'audio' ? `data:${block.mimeType};base64,${block.data}` : '';
}

function getResourceLinkLabel(block: ContentBlock): string {
  if (block.type !== 'resource_link') {
    return '';
  }

  return block.name || block.title || block.uri;
}

function getResourceUri(block: ContentBlock): string {
  return block.type === 'resource' ? block.resource.uri : '';
}

function getResourceBody(block: ContentBlock): string {
  if (block.type !== 'resource') {
    return '';
  }

  return 'text' in block.resource ? block.resource.text : block.resource.blob;
}

function isToolOnlyAssistantMessage(message: ChatMessage): boolean {
  return (
    message.role === 'assistant' &&
    !message.content &&
    !getMessageBlocks(message).length &&
    getMessageToolCalls(message).length > 0
  );
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
          <ModelPicker 
            v-if="availableModels.length > 0"
            :models="availableModels"
            :current-model-id="currentModelId"
            @change="handleModelChange"
          />
          <ModePicker 
            v-if="availableModes.length > 0"
            :modes="availableModes"
            :current-mode-id="currentModeId"
            @change="handleModeChange"
          />
        </template>
        <span class="agent-name">{{ currentSession?.agentName }}</span>
      </div>
    </div>
    
    <div ref="messagesContainer" class="messages-container">
        <PlanView v-if="currentPlan" :entries="currentPlan" />
        <div 
          v-for="message in messages" 
          :key="message.id"
          :class="[
            'message',
            `message-${message.role}`,
            { 'message-tool-only': isToolOnlyAssistantMessage(message) },
          ]"
        >
        <div class="message-header">
          <span class="role">{{ message.role === 'user' ? 'You' : 'Assistant' }}</span>
        </div>
        
        <!-- Agent thinking section (collapsible) - shown first to explain reasoning -->
        <div v-if="message.thought && message.role === 'assistant'" class="thought-section">
          <button class="thought-toggle" @click="toggleThought(message.id)">
            <span class="thought-icon">💭</span>
            <span class="thought-label">{{ isThoughtExpanded(message.id) ? 'Hide Thinking' : 'Show Thinking' }}</span>
            <span class="thought-chevron">{{ isThoughtExpanded(message.id) ? '▲' : '▼' }}</span>
          </button>
          <div v-if="isThoughtExpanded(message.id)" class="thought-content">
            <div v-html="renderMarkdown(message.thought)" />
          </div>
        </div>
        
        <!-- Tool calls for this message (shown after thinking) -->
        <div v-if="getMessageToolCalls(message).length" class="tool-calls-section">
          <div 
            v-for="tc in getMessageToolCalls(message)" 
            :key="tc.toolCallId"
            :class="['tool-call-inline', `tool-${tc.status}`]"
          >
            <span class="tool-icon">{{ getToolIcon(tc.kind) }}</span>
            <span class="tool-name">{{ tc.title }}</span>
            <span v-if="tc.locations?.length" class="tool-location">
              {{ tc.locations[0].path }}
            </span>
            <span :class="['tool-status', `status-${tc.status}`]">
              {{ getStatusIcon(tc.status) }}
            </span>
          </div>
        </div>
        
        <div
          v-for="(block, index) in getMessageBlocks(message)"
          :key="`${message.id}-block-${index}`"
          class="message-content-block"
        >
          <div
            v-if="block.type === 'text'"
            class="message-content"
            v-html="renderMarkdown(block.text)"
          />
          <img
            v-else-if="block.type === 'image'"
            class="message-image"
            :src="getImageSrc(block)"
            :alt="block.uri ?? 'Message image'"
          />
          <audio
            v-else-if="block.type === 'audio'"
            class="message-audio"
            controls
            :src="getAudioSrc(block)"
          />
          <a
            v-else-if="block.type === 'resource_link'"
            class="resource-link-chip"
            :href="block.uri"
            target="_blank"
            rel="noreferrer noopener"
          >
            <span>{{ getResourceLinkLabel(block) }}</span>
            <span class="resource-link-uri">{{ block.uri }}</span>
          </a>
          <div v-else-if="block.type === 'resource'" class="resource-block">
            <div class="resource-label">{{ getResourceUri(block) }}</div>
            <pre class="resource-content"><code>{{ getResourceBody(block) }}</code></pre>
          </div>
        </div>

        <div
          v-if="shouldRenderLegacyContent(message)"
          class="message-content"
          v-html="renderMarkdown(message.content)"
        />
      </div>
      
      <!-- Loading indicator -->
      <div v-if="isLoading" class="loading-indicator">
        <span class="spinner"></span>
        <span>Thinking...</span>
        <button class="cancel-btn" @click="handleCancel">Cancel</button>
      </div>
    </div>
    
    <div class="input-container">
      <CommandPalette
        ref="commandPaletteRef"
        :commands="availableCommands"
        :filter="commandFilter"
        :visible="showCommandPalette"
        @select="handleCommandSelect"
        @close="handleCommandClose"
      />
      <textarea
        v-model="inputText"
        :placeholder="activeCommandHint ?? 'Type your message...'"
        :disabled="isLoading"
        @keydown="handleKeyDown"
        rows="3"
      />
      <button 
        class="send-btn"
        :disabled="!inputText.trim() || isLoading"
        @click="handleSend"
      >
        Send
      </button>
    </div>
  </div>
</template>

<style scoped>
.chat-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.chat-header {
  padding: 1rem;
  border-bottom: 1px solid var(--border-color, #e0e0e0);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.chat-header h2 {
  margin: 0;
  font-size: 1.1rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.header-right {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.75rem;
}

.agent-name {
  font-size: 0.875rem;
  color: var(--text-accent, #0066cc);
}

.messages-container {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
}

.message {
  margin-bottom: 1rem;
  padding: 0.75rem;
  border-radius: 8px;
}

.message-user {
  background: var(--bg-user, #e3f2fd);
  margin-left: 2rem;
}

.message-assistant {
  background: var(--bg-assistant, #f5f5f5);
  margin-right: 2rem;
}

.message-header {
  margin-bottom: 0.5rem;
}

.role {
  font-weight: 600;
  font-size: 0.875rem;
  color: var(--text-secondary, #666);
}

/* Tool calls inline styles */
.tool-calls-section {
  margin-bottom: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.tool-call-inline {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.625rem;
  border-radius: 4px;
  font-size: 0.8rem;
  background: rgba(0, 0, 0, 0.04);
  border-left: 2px solid var(--border-color);
}

.tool-pending {
  border-left-color: #f59e0b;
}

.tool-in_progress {
  border-left-color: #3b82f6;
  background: rgba(59, 130, 246, 0.08);
}

.tool-completed {
  border-left-color: #10b981;
  background: rgba(16, 185, 129, 0.08);
}

.tool-failed {
  border-left-color: #ef4444;
  background: rgba(239, 68, 68, 0.08);
}

.tool-icon {
  font-size: 0.875rem;
}

.tool-name {
  font-weight: 500;
  color: var(--text-primary);
}

.tool-location {
  flex: 1;
  color: var(--text-muted);
  font-size: 0.75rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tool-status {
  font-size: 0.75rem;
  font-weight: 600;
}

.status-pending { color: #f59e0b; }
.status-in_progress { color: #3b82f6; }
.status-completed { color: #10b981; }
.status-failed { color: #ef4444; }

.message-content {
  line-height: 1.5;
  overflow-wrap: break-word;
  word-wrap: break-word;
}

.message-content-block + .message-content-block {
  margin-top: 0.75rem;
}

.message-content :deep(p) {
  margin: 0.5rem 0;
}

.message-content :deep(ol),
.message-content :deep(ul) {
  margin: 0.5rem 0;
  padding-left: 1.5rem;
}

.message-content :deep(li) {
  margin: 0.25rem 0;
}

.message-content :deep(pre) {
  background: var(--bg-code, #282c34);
  color: var(--text-code, #abb2bf);
  padding: 0.75rem;
  border-radius: 4px;
  overflow-x: auto;
}

.message-content :deep(code) {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 0.9rem;
}

.messages-container :deep(.mermaid-rendered) {
  margin: 0.75rem 0;
}

.messages-container :deep(.mermaid-rendered svg) {
  display: block;
  max-width: 100%;
  border-radius: 6px;
}

.messages-container :deep(pre.mermaid-error) {
  border: 1px solid rgba(239, 68, 68, 0.45);
}

.messages-container :deep(.mermaid-error-message) {
  margin-top: 0.5rem;
  color: #fca5a5;
  font-size: 0.8rem;
}

.message-image {
  display: block;
  max-width: min(100%, 32rem);
  max-height: 24rem;
  border-radius: 6px;
  border: 1px solid var(--border-color, #e0e0e0);
}

.message-audio {
  width: min(100%, 28rem);
}

.resource-link-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.375rem 0.75rem;
  border-radius: 999px;
  background: var(--bg-hover, #f0f4f8);
  border: 1px solid var(--border-color, #d0d7de);
  color: var(--text-accent, #0066cc);
  text-decoration: none;
  font-size: 0.85rem;
  font-weight: 500;
}

.resource-link-chip:hover {
  text-decoration: underline;
}

.resource-link-uri {
  max-width: 18rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-muted, #666);
  font-weight: 400;
}

.resource-block {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.resource-label {
  font-size: 0.8rem;
  color: var(--text-muted, #666);
  word-break: break-all;
}

.resource-content {
  margin: 0;
  background: var(--bg-code, #282c34);
  color: var(--text-code, #abb2bf);
  padding: 0.75rem;
  border-radius: 4px;
  overflow-x: auto;
}

.loading-indicator {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem;
  color: var(--text-muted, #666);
}

.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid var(--border-color, #ccc);
  border-top-color: var(--text-accent, #0066cc);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.cancel-btn {
  margin-left: auto;
  padding: 0.25rem 0.5rem;
  border: 1px solid var(--border-color, #ccc);
  border-radius: 4px;
  background: transparent;
  font-size: 0.8rem;
  cursor: pointer;
}

.input-container {
  position: relative;
  display: flex;
  gap: 0.5rem;
  padding: 1rem;
  border-top: 1px solid var(--border-color, #e0e0e0);
}

textarea {
  flex: 1;
  padding: 0.75rem;
  border: 1px solid var(--border-color, #ccc);
  border-radius: 6px;
  font-size: 1rem;
  font-family: inherit;
  resize: none;
}

textarea:focus {
  outline: none;
  border-color: var(--text-accent, #0066cc);
}

.send-btn {
  padding: 0.75rem 1.5rem;
  background: var(--bg-primary, #0066cc);
  color: white;
  border: none;
  border-radius: 6px;
  font-size: 1rem;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s;
}

.send-btn:hover:not(:disabled) {
  background: var(--bg-primary-hover, #0052a3);
}

.send-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Agent Thinking Section */
.thought-section {
  margin-bottom: 0.75rem;
  border: 1px solid var(--border-color, #e0e0e0);
  border-radius: 8px;
  overflow: hidden;
}

.thought-toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.5rem 0.75rem;
  background: var(--bg-hover, #f5f5f5);
  border: none;
  cursor: pointer;
  font-size: 0.85rem;
  color: var(--text-muted, #666);
  text-align: left;
  transition: background 0.15s ease;
}

.thought-toggle:hover {
  background: var(--bg-user, #e3f2fd);
}

.thought-icon {
  font-size: 1rem;
  flex-shrink: 0;
}

.thought-label {
  flex: 1;
  font-weight: 500;
}

.thought-chevron {
  font-size: 0.7rem;
  color: var(--text-muted, #999);
}

.thought-content {
  padding: 0.75rem 1rem 0.75rem 1.25rem;
  background: var(--bg-main, #fafafa);
  border-top: 1px solid var(--border-color, #e0e0e0);
  font-size: 0.9rem;
  color: var(--text-muted, #666);
  font-style: italic;
  line-height: 1.5;
}

.thought-content :deep(p) {
  margin: 0 0 0.5rem 0;
}

.thought-content :deep(p:last-child) {
  margin-bottom: 0;
}

.thought-content :deep(code) {
  background: var(--bg-hover, #f0f0f0);
  padding: 0.125rem 0.25rem;
  border-radius: 3px;
  font-size: 0.85em;
}
</style>
