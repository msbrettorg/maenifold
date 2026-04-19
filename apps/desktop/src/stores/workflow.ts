import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { useMcpResources } from '../composables/useMcpResources';
import type { ResourceContent } from '../composables/mcpTransport';

interface WorkflowSessionSummary {
  id: string;
  uri: string;
  status: string;
  workflow_id?: string | null;
  current_step?: number;
  total_steps?: number;
  started_at?: string | null;
  updated_at?: string | null;
}

interface WorkflowSessionsResponse {
  sessions: WorkflowSessionSummary[];
  generated_at?: string;
}

interface WorkflowSessionDetail {
  id: string;
  uri: string;
  status: string;
  workflow_id?: string | null;
  current_step?: number;
  total_steps?: number;
  started_at?: string | null;
  updated_at?: string | null;
  history?: Array<{ heading: string; body: string }>;
}

function parseResourceJson<T>(content: ResourceContent, fallback: T): T {
  if (typeof content.text !== 'string') {
    return fallback;
  }
  try {
    return JSON.parse(content.text) as T;
  } catch {
    return fallback;
  }
}

export const useWorkflowStore = defineStore('workflow', () => {
  const resources = useMcpResources();

  const activeSessions = ref<WorkflowSessionSummary[]>([]);
  const selectedSessionId = ref<string | null>(null);
  const selectedSession = ref<WorkflowSessionDetail | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const lastUpdated = ref<string | null>(null);

  const sessionsUnsubscribe = ref<(() => void) | null>(null);
  const detailUnsubscribe = ref<(() => void) | null>(null);
  const subscriberCount = ref(0);

  const hasSessions = computed(() => activeSessions.value.length > 0);

  async function refreshSessions() {
    loading.value = true;
    error.value = null;
    try {
      const resource = await resources.readResource('workflow://sessions');
      const payload = parseResourceJson<WorkflowSessionsResponse>(resource, { sessions: [] });
      activeSessions.value = Array.isArray(payload.sessions) ? payload.sessions : [];
      lastUpdated.value = payload.generated_at ?? new Date().toISOString();
      if (
        selectedSessionId.value
        && !activeSessions.value.some((session) => session.id === selectedSessionId.value)
      ) {
        selectedSessionId.value = null;
        selectedSession.value = null;
      }
      if (!selectedSessionId.value && activeSessions.value.length > 0) {
        selectedSessionId.value = activeSessions.value[0].id;
      }
    } catch (unknownError) {
      error.value = unknownError instanceof Error ? unknownError.message : String(unknownError);
    } finally {
      loading.value = false;
    }
  }

  async function refreshSelectedSession() {
    if (!selectedSessionId.value) {
      selectedSession.value = null;
      return;
    }
    try {
      const resource = await resources.readResource(
        `workflow://session/${encodeURIComponent(selectedSessionId.value)}`,
      );
      selectedSession.value = parseResourceJson<WorkflowSessionDetail>(resource, {
        id: selectedSessionId.value,
        uri: '',
        status: 'unknown',
      });
    } catch (unknownError) {
      error.value = unknownError instanceof Error ? unknownError.message : String(unknownError);
    }
  }

  async function resubscribeSelectedSessionDetail() {
    detailUnsubscribe.value?.();
    detailUnsubscribe.value = null;
    if (!selectedSessionId.value) {
      return;
    }
    detailUnsubscribe.value = await resources.subscribeResource(
      `workflow://session/${encodeURIComponent(selectedSessionId.value)}`,
      () => {
        void refreshSelectedSession();
      },
    );
  }

  async function selectSession(id: string) {
    selectedSessionId.value = id;
    await refreshSelectedSession();
    await resubscribeSelectedSessionDetail();
  }

  async function mount() {
    subscriberCount.value += 1;
    if (subscriberCount.value > 1) {
      return;
    }

    await refreshSessions();
    await refreshSelectedSession();

    sessionsUnsubscribe.value = await resources.subscribeResource('workflow://sessions', () => {
      void refreshSessions().then(() => refreshSelectedSession());
    });
    await resubscribeSelectedSessionDetail();
  }

  function unmount() {
    subscriberCount.value = Math.max(0, subscriberCount.value - 1);
    if (subscriberCount.value > 0) {
      return;
    }

    sessionsUnsubscribe.value?.();
    sessionsUnsubscribe.value = null;
    detailUnsubscribe.value?.();
    detailUnsubscribe.value = null;
  }

  return {
    activeSessions,
    selectedSessionId,
    selectedSession,
    loading,
    error,
    lastUpdated,
    hasSessions,
    refreshSessions,
    refreshSelectedSession,
    selectSession,
    mount,
    unmount,
  };
});
