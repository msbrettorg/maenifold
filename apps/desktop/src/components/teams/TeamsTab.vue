<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, type Component } from 'vue';
import { MessagesSquare } from 'lucide-vue-next';
import EmptyState from '@/components/EmptyState.vue';
import ChatView from '@/components/ChatView.vue';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { McpCapabilityUnsupportedError, useMcpResources } from '@/composables/useMcpResources';
import type { SavedSession } from '@/lib/types';
import { useAppShellStore } from '@/stores/app-shell';
import { useSessionStore } from '@/stores/session';

type TeamsMonitorState = 'connected' | 'disconnected' | 'error';

interface TeamsMonitorStatus {
  state: TeamsMonitorState;
  label: string;
  detail: string;
}

interface TeamsStatusPayload {
  status?: unknown;
  connected?: unknown;
  error?: unknown;
  detail?: unknown;
  message?: unknown;
}

const STATUS_URI = 'teams://status';
const props = withDefaults(defineProps<{
  emptyIcon?: Component;
}>(), {
  emptyIcon: MessagesSquare,
});
const mcpResources = useMcpResources();
const appShellStore = useAppShellStore();
const sessionStore = useSessionStore();
const currentSession = computed(() => sessionStore.currentSession);
const savedSessions = computed(() => sessionStore.savedSessions);
const status = ref<TeamsMonitorStatus>({
  state: 'disconnected',
  label: 'Disconnected',
  detail: 'Open Settings to configure Teams monitor access.',
});
const isRefreshingStatus = ref(false);
const resumingSessionId = ref<string | null>(null);
let unsubscribeStatusUpdates: (() => void) | null = null;

function isTeamsSession(session: SavedSession | null): session is SavedSession {
  if (!session) {
    return false;
  }

  const searchTerms = [session.agentName, session.title, session.cwd]
    .filter((value): value is string => Boolean(value))
    .map((value) => value.toLowerCase());
  return searchTerms.some((value) => value.includes('teams'));
}

function defaultStatus(state: TeamsMonitorState, detail?: string): TeamsMonitorStatus {
  if (state === 'connected') {
    return {
      state,
      label: 'Connected',
      detail: detail ?? 'Teams monitor is connected and ready to surface chat sessions.',
    };
  }

  if (state === 'error') {
    return {
      state,
      label: 'Error',
      detail: detail ?? 'Unable to read Teams monitor status right now.',
    };
  }

  return {
    state,
    label: 'Disconnected',
    detail: detail ?? 'Open Settings to configure Teams monitor access.',
  };
}

function parseStatusPayload(text?: string): TeamsMonitorStatus {
  if (!text) {
    return defaultStatus('disconnected');
  }

  try {
    const payload = JSON.parse(text) as TeamsStatusPayload;
    const explicitStatus = typeof payload.status === 'string' ? payload.status.toLowerCase() : null;
    const detail = typeof payload.detail === 'string'
      ? payload.detail
      : typeof payload.message === 'string'
        ? payload.message
        : typeof payload.error === 'string'
          ? payload.error
          : undefined;

    if (explicitStatus === 'connected' || payload.connected === true) {
      return defaultStatus('connected', detail);
    }

    if (explicitStatus === 'error' || typeof payload.error === 'string') {
      return defaultStatus('error', detail);
    }

    if (explicitStatus === 'disconnected' || payload.connected === false) {
      return defaultStatus('disconnected', detail);
    }
  } catch {
    const normalized = text.trim().toLowerCase();
    if (normalized.includes('connected')) {
      return defaultStatus('connected');
    }
    if (normalized.includes('error')) {
      return defaultStatus('error', text.trim());
    }
  }

  return defaultStatus('disconnected');
}

async function refreshStatus(): Promise<void> {
  isRefreshingStatus.value = true;
  try {
    const resource = await mcpResources.readResource(STATUS_URI);
    status.value = parseStatusPayload(resource.text);
  } catch (error) {
    status.value = defaultStatus(
      'error',
      error instanceof Error ? error.message : 'Unable to read Teams monitor status.',
    );
  } finally {
    isRefreshingStatus.value = false;
  }
}

const teamsSessions = computed(() =>
  savedSessions.value
    .filter(isTeamsSession)
    .sort((left, right) => right.lastUpdated - left.lastUpdated),
);

const activeTeamsSession = computed(() => isTeamsSession(currentSession.value) ? currentSession.value : null);

const statusDotClass = computed(() => {
  if (status.value.state === 'connected') {
    return 'bg-brand';
  }
  if (status.value.state === 'error') {
    return 'bg-danger';
  }
  return 'bg-text-tertiary';
});

function formatUpdatedAt(timestamp: number): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(timestamp);
}

function openSettings(): void {
  appShellStore.setActiveTab('settings');
}

async function resumeTeamsSession(session: SavedSession): Promise<void> {
  resumingSessionId.value = session.id;
  try {
    await sessionStore.resumeSession(session);
  } finally {
    resumingSessionId.value = null;
  }
}

onMounted(() => {
  void refreshStatus();
  void mcpResources.subscribeResource(STATUS_URI, () => {
    void refreshStatus();
  }).then((unsubscribe) => {
    unsubscribeStatusUpdates = unsubscribe;
  }).catch((error) => {
    if (error instanceof McpCapabilityUnsupportedError) {
      return;
    }
    status.value = defaultStatus(
      'error',
      error instanceof Error ? error.message : 'Unable to subscribe to Teams monitor updates.',
    );
  });
});

onBeforeUnmount(() => {
  unsubscribeStatusUpdates?.();
  unsubscribeStatusUpdates = null;
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col bg-bg-app">
    <div class="flex items-start justify-between gap-4 px-4 py-4">
      <div class="grid gap-1">
        <h2 class="text-md font-semibold text-text-primary">Teams Monitor</h2>
        <p class="text-sm text-text-secondary">{{ status.detail }}</p>
      </div>
      <Badge variant="outline" class="border-border-subtle bg-bg-app text-text-primary">
        <span class="mr-2 inline-flex size-2 rounded-full" :class="statusDotClass" />
        {{ isRefreshingStatus ? 'Refreshing…' : status.label }}
      </Badge>
    </div>

    <div v-if="activeTeamsSession" class="min-h-0 flex-1">
      <ChatView />
    </div>

    <div v-else-if="teamsSessions.length > 0" class="flex min-h-0 flex-1 flex-col px-4 pb-4">
      <div class="grid gap-1 pb-3">
        <h3 class="text-sm font-medium text-text-primary">Teams-originated sessions</h3>
        <p class="text-xs text-text-secondary">
          Resume a Teams conversation to continue using the existing chat workflow.
        </p>
      </div>

      <ScrollArea class="min-h-0 flex-1">
        <div class="grid gap-2 pr-4">
          <Button
            v-for="session in teamsSessions"
            :key="session.id"
            variant="outline"
            class="h-auto items-start justify-between px-3 py-3 text-left"
            :loading="resumingSessionId === session.id"
            @click="resumeTeamsSession(session)"
          >
            <span class="grid gap-1">
              <span class="text-sm font-medium text-text-primary">{{ session.title }}</span>
              <span class="text-xs text-text-secondary">
                {{ session.agentName }} · {{ formatUpdatedAt(session.lastUpdated) }}
              </span>
            </span>
          </Button>
        </div>
      </ScrollArea>
    </div>

    <div v-else class="flex min-h-0 flex-1 items-center justify-center px-6 pb-6">
      <EmptyState
        :icon="props.emptyIcon"
        :title="status.state === 'connected' ? 'No Teams conversations yet' : 'Teams monitor unavailable'"
        :description="status.state === 'connected'
          ? 'Teams-originated sessions will appear here after the monitor captures activity.'
          : 'Open Settings to configure Teams monitor access and start collecting conversations.'"
        action-label="Open Settings"
        @action="openSettings"
      />
    </div>
  </div>
</template>
