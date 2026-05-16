import { computed, ref, type ComputedRef, type Ref } from 'vue';
import { toast } from 'vue-sonner';
import type { SessionRef } from '../lib/pipeline-types-node';
import type { PipelineView } from '../lib/pipeline-types-view';
import { createNode, createNodeId } from '../lib/pipeline-node-factory';
import { usePipelineStore } from '../stores/pipeline';
import { usePromptDialogStore } from '../stores/prompt-dialog';

export type Point = { x: number; y: number };
export type ContextMenuState =
  | { mode: 'canvas'; position: Point }
  | { mode: 'node'; nodeId: string };

export type UseComposerContextMenuOptions = {
  designMode: () => boolean;
  view: () => PipelineView | null | undefined;
  firstAgent: ComputedRef<string>;
  firstWorkflow: ComputedRef<string>;
  createNodeAt: (node: ReturnType<typeof createNode>, position: Point) => void;
  editNode: (nodeId: string) => void;
  duplicateNode: (nodeId: string) => void;
  deleteNode: (nodeId: string) => void;
  disconnectNodeEdges: (nodeId: string) => void;
  toFlowPosition: (clientX: number, clientY: number, root: HTMLElement) => Point;
  emitNodeDoubleClick: (nodeId: string) => void;
  emitOpenGraft: () => void;
  emitOpenPromotion: () => void;
};

export function useComposerContextMenu(options: UseComposerContextMenuOptions) {
  const store = usePipelineStore();
  const promptDialogStore = usePromptDialogStore();
  const contextMenu: Ref<ContextMenuState | null> = ref(null);
  const sessionPickerOpen = ref(false);
  const pendingSessionNodePosition = ref<Point | null>(null);

  const contextMenuEnabled = computed(() => contextMenu.value !== null);

  const contextMenuNodeStatus = computed(() => {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return null;
    const nodeId = contextMenu.value.nodeId;
    return options.view()?.nodes.find((node) => node.nodeId === nodeId)?.status ?? null;
  });

  const contextMenuNodeHasOutput = computed(() => {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return false;
    const nodeId = contextMenu.value.nodeId;
    return Boolean(options.view()?.nodes.find((node) => node.nodeId === nodeId)?.output);
  });

  function hasServerCanvasMenuActions(): boolean {
    const view = options.view();
    if (!view) return false;
    return (
      ['active', 'paused', 'stuck'].includes(view.status)
      || view.nodes.some((node) => ['completed', 'abandoned', 'stuck'].includes(node.status))
      || view.nodes.some((node) => node.status === 'completed' && Boolean(node.output))
    );
  }

  function closeContextMenu(): void {
    contextMenu.value = null;
  }

  function closeSessionPicker(): void {
    sessionPickerOpen.value = false;
    pendingSessionNodePosition.value = null;
  }

  function createContextNode(type: 'prompt' | 'sequentialThinking' | 'workflow' | 'tool', toolName?: string): void {
    if (!contextMenu.value || contextMenu.value.mode !== 'canvas') return;

    const agent = options.firstAgent.value;
    const workflow = options.firstWorkflow.value;
    const node = type === 'prompt'
      ? agent ? createNode('prompt', { id: createNodeId('prompt'), agent, prompt: 'Ask an agent' }) : null
      : type === 'sequentialThinking'
        ? agent ? createNode('sequentialThinking', { id: createNodeId('sequentialThinking'), agent, topic: 'Think through a problem', totalThoughts: 5 }) : null
        : type === 'workflow'
          ? workflow ? createNode('workflow', { id: createNodeId('workflow'), workflowId: workflow, agent: '' }) : null
          : toolName ? createNode('tool', { id: createNodeId('tool'), toolName }) : null;

    if (!node) return;
    options.createNodeAt(node, contextMenu.value.position);
    closeContextMenu();
  }

  function handleComposerContextMenu(event: MouseEvent): void {
    const target = event.target;
    if (!(target instanceof Element)) return;
    if (target.closest('.vue-flow__controls, .vue-flow__minimap')) return;

    const flowRoot = event.currentTarget instanceof HTMLElement
      ? event.currentTarget.querySelector<HTMLElement>('.vue-flow')
      : target.closest<HTMLElement>('.vue-flow');
    if (!flowRoot) {
      closeContextMenu();
      return;
    }

    const nodeId = target.closest<HTMLElement>('.vue-flow__node')?.dataset.id;
    if (nodeId) {
      options.editNode(nodeId);
      contextMenu.value = { mode: 'node', nodeId };
      return;
    }

    if (target.closest('.vue-flow__edge, .edge-indicator')) {
      closeContextMenu();
      return;
    }

    if (!options.designMode() && !hasServerCanvasMenuActions()) {
      closeContextMenu();
      return;
    }

    contextMenu.value = {
      mode: 'canvas',
      position: options.toFlowPosition(event.clientX, event.clientY, flowRoot),
    };
  }

  function handleEditNode(): void {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
    options.emitNodeDoubleClick(contextMenu.value.nodeId);
    closeContextMenu();
  }

  function handleDuplicateNode(): void {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
    options.duplicateNode(contextMenu.value.nodeId);
    closeContextMenu();
  }

  function handleDeleteNode(): void {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
    options.deleteNode(contextMenu.value.nodeId);
    closeContextMenu();
  }

  function handleDisconnectEdges(): void {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
    options.disconnectNodeEdges(contextMenu.value.nodeId);
    closeContextMenu();
  }

  function handleServerNodeAction(action: 'restartNode' | 'rewindNode' | 'forkNode' | 'abandonNode'): void {
    if (!contextMenu.value || contextMenu.value.mode !== 'node') return;
    const { nodeId } = contextMenu.value;
    closeContextMenu();

    switch (action) {
      case 'restartNode': void store.restartNode(nodeId); break;
      case 'rewindNode': void store.rewindNodes([nodeId]); break;
      case 'forkNode': {
        const pipelineId = store.currentPipelineId;
        if (!pipelineId) break;
        void (async () => {
          try {
            const requestedName = await promptDialogStore.requestValue({
              title: 'Fork pipeline session',
              description: 'Optional name for the forked pipeline session.',
              placeholder: 'my-forked-session',
              confirmLabel: 'Fork',
            });
            if (requestedName === null) return;
            await store.forkPipeline(pipelineId, nodeId, requestedName.trim() || undefined);
          } catch (error) {
            console.error('Fork pipeline session failed', error);
            toast.error('Fork failed — please retry');
          }
        })();
        break;
      }
      case 'abandonNode': void store.updateNodeStatus(nodeId, 'abandoned'); break;
    }
  }

  function handleCanvasRunPipeline(): void {
    void store.runPipeline();
    closeContextMenu();
  }

  function handleCanvasRewindAll(): void {
    const rewindableIds = options.view()?.nodes
      .filter((node) => ['completed', 'abandoned', 'stuck'].includes(node.status))
      .map((node) => node.nodeId) ?? [];
    if (rewindableIds.length > 0) void store.rewindNodes(rewindableIds);
    closeContextMenu();
  }

  function handleCanvasGraft(): void {
    options.emitOpenGraft();
    closeContextMenu();
  }

  function handleCanvasPromoteToMemory(): void {
    options.emitOpenPromotion();
    closeContextMenu();
  }

  function openSessionPicker(): void {
    if (!contextMenu.value || contextMenu.value.mode !== 'canvas') return;
    pendingSessionNodePosition.value = contextMenu.value.position;
    sessionPickerOpen.value = true;
    closeContextMenu();
  }

  function addSessionNode(sessionRef: SessionRef): void {
    const position = pendingSessionNodePosition.value;
    if (!position) return;

    const node = createNode('prompt', {
      id: createNodeId('prompt'),
      agent: sessionRef.agent,
      prompt: 'Ask an agent',
      sessionRef,
    });

    options.createNodeAt(node, position);
    closeSessionPicker();
  }

  function handleContextMenuOpenChange(open: boolean): void {
    if (!open) closeContextMenu();
  }

  return {
    contextMenu,
    contextMenuEnabled,
    contextMenuNodeStatus,
    contextMenuNodeHasOutput,
    sessionPickerOpen,
    closeContextMenu,
    closeSessionPicker,
    createContextNode,
    handleComposerContextMenu,
    handleEditNode,
    handleDuplicateNode,
    handleDeleteNode,
    handleDisconnectEdges,
    handleServerNodeAction,
    handleCanvasRunPipeline,
    handleCanvasRewindAll,
    handleCanvasGraft,
    handleCanvasPromoteToMemory,
    handleContextMenuOpenChange,
    openSessionPicker,
    addSessionNode,
    createPromptNode: () => createContextNode('prompt'),
    createThinkingNode: () => createContextNode('sequentialThinking'),
    createToolNode: (toolName: string) => createContextNode('tool', toolName),
    createWorkflowNode: () => createContextNode('workflow'),
  };
}
