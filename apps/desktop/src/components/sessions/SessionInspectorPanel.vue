<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import ChatMessageList from '@/components/ChatMessageList.vue';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useMermaid } from '@/composables/useMermaid';
import { useSessionStore } from '@/stores/session';

const props = defineProps<{
  sessionId: string;
}>();

const emit = defineEmits<{
  prefill: [text: string];
  error: [message: string];
}>();

const sessionStore = useSessionStore();
const { sessions } = storeToRefs(sessionStore);
const { renderMermaidBlocks } = useMermaid();
const scrollAreaRef = ref<InstanceType<typeof ScrollArea> | null>(null);
const messagesContainer = ref<HTMLElement | null>(null);
const expandedThoughts = ref<Set<string>>(new Set());
const expandedToolCalls = ref<Set<string>>(new Set());
const isUnmounted = ref(false);

const inspectedSession = computed(() => sessions.value.get(props.sessionId) ?? null);
const messages = computed(() => inspectedSession.value?.messages ?? []);
const currentPlan = computed(() => inspectedSession.value?.currentPlan ?? null);
const pendingPermissionCount = computed(() => inspectedSession.value?.pendingPermissions.length ?? 0);
const isLoading = computed(() => inspectedSession.value?.isLoading ?? false);
const expandedThoughtIds = computed(() => Array.from(expandedThoughts.value));
const expandedToolCallIds = computed(() => Array.from(expandedToolCalls.value));

function resolveScrollViewport(): HTMLElement | null {
  const root = (scrollAreaRef.value as unknown as { $el?: HTMLElement } | null)?.$el ?? null;
  if (!root) return null;
  const viewport = root.querySelector('[data-reka-scroll-area-viewport]') as HTMLElement | null;
  return viewport ?? root;
}
function panelScopeInactive(sessionId: string): boolean {
  return isUnmounted.value || sessionId !== props.sessionId;
}

async function rerenderExpandedMarkdown(): Promise<void> {
  await nextTick();
  if (isUnmounted.value) return;
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

async function cancelSession(): Promise<void> {
  if (!inspectedSession.value) return;
  try {
    sessionStore.setActiveSession(props.sessionId);
    await sessionStore.cancelOperation();
  } catch (error) {
    emit('error', error instanceof Error ? error.message : String(error));
  }
}

watch(messages, async () => {
  const sessionId = props.sessionId;
  await nextTick();
  if (panelScopeInactive(sessionId)) return;
  const viewport = resolveScrollViewport();
  if (viewport) viewport.scrollTop = viewport.scrollHeight;
  if (!isLoading.value && messagesContainer.value) await renderMermaidBlocks(messagesContainer.value);
}, { deep: true });

onMounted(async () => {
  const sessionId = props.sessionId;
  await nextTick();
  if (panelScopeInactive(sessionId)) return;
  if (messagesContainer.value) await renderMermaidBlocks(messagesContainer.value);
});
onBeforeUnmount(() => { isUnmounted.value = true; });
</script>

<template>
  <ScrollArea ref="scrollAreaRef" class="min-h-0 flex-1">
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
        @cancel="cancelSession"
        @prefill="emit('prefill', $event)"
      />
    </div>
  </ScrollArea>
</template>
