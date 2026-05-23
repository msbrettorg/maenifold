import { computed, type ComputedRef } from 'vue';
import { normalizeAuthMethods, type AppAuthMethod } from '../lib/auth-methods';
import type {
  AgentInfo,
  AvailableCommand,
  ChatMessage,
  PermissionRequest,
  PlanEntry,
  PromptCapabilities,
  SavedSession,
  SessionConfigOption,
  SessionState,
} from '../lib/types';
import {
  overlayReasoningEffortOptions,
  type SessionCopilotReasoningEffortMap,
} from './session-copilot-models';

interface SessionGetterSource {
  activeSession: ComputedRef<SessionState | null>;
  activeSessionId: ComputedRef<string | null>;
  copilotModelReasoningEfforts: SessionCopilotReasoningEffortMap;
}

export function createSessionGetters({
  activeSession,
  activeSessionId,
  copilotModelReasoningEfforts,
}: SessionGetterSource) {
  const currentSession = computed<SavedSession | null>(
    () => activeSession.value?.session ?? null
  );
  const messages = computed<ChatMessage[]>(() => activeSession.value?.messages ?? []);
  const isConnected = computed<boolean>(
    () => activeSession.value?.isConnected ?? false
  );
  const isLoading = computed<boolean>(() => activeSession.value?.isLoading ?? false);
  const isConnecting = computed<boolean>(
    () => activeSession.value?.isConnecting ?? false
  );
  const error = computed<string | null>(() => activeSession.value?.error ?? null);
  const pendingPermission = computed<PermissionRequest | null>(
    () => activeSession.value?.pendingPermissions[0] ?? null
  );
  const pendingPermissions = computed<PermissionRequest[]>(
    () => activeSession.value?.pendingPermissions ?? []
  );
  const pendingPermissionCount = computed<number>(
    () => activeSession.value?.pendingPermissions.length ?? 0
  );
  const supportsSessionList = computed<boolean>(
    () => activeSession.value?.supportsSessionList ?? false
  );
  const agentInfo = computed<AgentInfo | null>(
    () => activeSession.value?.agentInfo ?? null
  );
  const agentPromptCapabilities = computed<PromptCapabilities>(
    () => activeSession.value?.agentPromptCapabilities ?? {}
  );
  const pendingAuthMethods = computed<AppAuthMethod[]>(
    () => normalizeAuthMethods(activeSession.value?.pendingAuthMethods ?? [])
  );
  const pendingAuthAgentName = computed<string>(
    () => activeSession.value?.pendingAuthAgentName ?? ''
  );
  const configOptions = computed<SessionConfigOption[]>(
    () => {
      const configOptions = activeSession.value?.configOptions ?? [];
      const localId = activeSessionId.value;
      const reasoningEfforts = localId
        ? copilotModelReasoningEfforts.value.get(localId)
        : null;
      return overlayReasoningEffortOptions(configOptions, reasoningEfforts);
    }
  );
  const hasConfigOptions = computed<boolean>(
    () => activeSession.value?.hasConfigOptions ?? false
  );
  const availableCommands = computed<AvailableCommand[]>(
    () => activeSession.value?.availableCommands ?? []
  );
  const currentPlan = computed<PlanEntry[] | null>(
    () => activeSession.value?.currentPlan ?? null
  );
  const startupPhase = computed<string>(
    () => activeSession.value?.startupPhase ?? ''
  );
  const startupLogs = computed<string[]>(() => activeSession.value?.startupLogs ?? []);
  const startupElapsed = computed<number>(
    () => activeSession.value?.startupElapsed ?? 0
  );
  const hasActiveSession = computed(() => currentSession.value !== null);
  const messageList = computed(() => messages.value);

  return {
    currentSession,
    messages,
    isConnected,
    isLoading,
    isConnecting,
    error,
    pendingPermission,
    pendingPermissions,
    pendingPermissionCount,
    supportsSessionList,
    agentInfo,
    agentPromptCapabilities,
    pendingAuthMethods,
    pendingAuthAgentName,
    configOptions,
    hasConfigOptions,
    availableCommands,
    currentPlan,
    startupPhase,
    startupLogs,
    startupElapsed,
    hasActiveSession,
    messageList,
  };
}
