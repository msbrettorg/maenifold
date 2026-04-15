import type { PipelineView, ViewNode } from '../lib/pipeline-types';
import { useAppShellStore } from '../stores/app-shell';
import { useSessionStore } from '../stores/session';

type EnterInspect = (nodeId: string, event: MouseEvent | TouchEvent) => void;

function normalizeSessionId(value?: string | null): string | null {
  if (!value) return null;
  const trimmedValue = value.trim();
  return trimmedValue.length > 0 ? trimmedValue : null;
}

function getLiveAgentSessionId(node: ViewNode | undefined): string | null {
  if (!node || node.status !== 'active' || node.type === 'tool') return null;
  return normalizeSessionId(node.provenance?.detail.sessionId)
    ?? normalizeSessionId(node.acpSessionId)
    ?? null;
}

export function usePipelineNodeSessionLauncher(
  view: () => PipelineView | null | undefined,
  enterInspect: EnterInspect,
) {
  const sessionStore = useSessionStore();
  const appShellStore = useAppShellStore();

  async function handleNodeDoubleClick(
    nodeId: string,
    event: MouseEvent | TouchEvent,
  ): Promise<void> {
    const viewNode = view()?.nodes.find((candidate) => candidate.nodeId === nodeId);
    const liveSessionId = getLiveAgentSessionId(viewNode);
    if (!liveSessionId) {
      enterInspect(nodeId, event);
      return;
    }

    try {
      const result = await sessionStore.focusOrResumeRemoteSession(liveSessionId);
      if (result.status === 'activated' || result.status === 'resumed') {
        appShellStore.openAgents();
      }
    } catch (error) {
      sessionStore.setError(
        error instanceof Error
          ? `Failed to reconnect ACP session: ${error.message}`
          : 'Failed to reconnect ACP session.',
      );
    }
  }

  return { handleNodeDoubleClick };
}
