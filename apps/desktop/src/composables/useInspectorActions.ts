import { ref, type Ref } from 'vue';
import { usePipelineStore } from '../stores/pipeline';
import type { ViewNode } from '../lib/pipeline-types';

type PipelineStore = ReturnType<typeof usePipelineStore>;
type ReadableRef<T> = Readonly<Ref<T>>;

interface UseInspectorActionsOptions {
  store: PipelineStore;
  selectedNodeId: ReadableRef<string | null>;
  selectedViewNode: ReadableRef<ViewNode | null>;
  conclusionText: Ref<string>;
}

export function useInspectorActions({
  store,
  selectedNodeId,
  selectedViewNode,
  conclusionText,
}: UseInspectorActionsOptions) {
  const busyAction = ref<string | null>(null);

  async function runBusyAction<T>(action: string, operation: () => Promise<T>): Promise<T | undefined> {
    if (busyAction.value !== null) return undefined;
    busyAction.value = action;

    try {
      return await operation();
    } finally {
      busyAction.value = null;
    }
  }

  async function completeNode(): Promise<void> {
    if (!selectedNodeId.value) return;

    await runBusyAction('complete', async () => {
      const conclusion = conclusionText.value.trim();
      await store.updateNodeStatus(selectedNodeId.value!, 'completed', conclusion || undefined);
      conclusionText.value = '';
    });
  }

  async function restartNode(): Promise<void> {
    if (!selectedViewNode.value) return;
    await runBusyAction('restart', () => store.restartNode(selectedViewNode.value!.nodeId));
  }

  async function rewindNode(): Promise<void> {
    if (!selectedViewNode.value) return;
    await runBusyAction('rewind', () => store.rewindNodes([selectedViewNode.value!.nodeId]));
  }

  async function abandonNode(): Promise<void> {
    if (!selectedViewNode.value) return;
    await runBusyAction('abandon', () => store.updateNodeStatus(selectedViewNode.value!.nodeId, 'abandoned'));
  }

  return {
    busyAction,
    runBusyAction,
    completeNode,
    restartNode,
    rewindNode,
    abandonNode,
  };
}
