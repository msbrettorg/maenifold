import { computed, ref, watch, type Ref } from 'vue';
import { createNode, createNodeId } from '../lib/pipeline-node-factory';
import type { EdgeType, PipelineEdge, PipelineNode, PipelineNodeType } from '../lib/pipeline-types';
import {
  buildPipelineNodeDraft,
  canSubmitPipelineNodeForm,
  usePipelineNodeFormFields,
} from './pipeline-node-form-helpers';

interface UseGraftNodesOptions {
  existingNodeIds: Ref<string[]>;
  busy: Ref<boolean>;
  onSubmit: (nodes: PipelineNode[], edges: PipelineEdge[]) => void;
}

export function useGraftNodes({ existingNodeIds, busy, onSubmit }: UseGraftNodesOptions) {
  const workflowListId = 'graft-nodes-workflows';
  const nodeType = ref<PipelineNodeType>('workflow');
  const nodeId = ref('');
  const {
    workflowId,
    workflowAgent,
    workflowPrompt,
    promptAgent,
    promptTemplate,
    thinkingTopic,
    thinkingAgent,
    thinkingTotalThoughts,
    thinkingAnalysisType,
    toolName,
    toolArgs,
    toolArgsValid,
    getFormValues,
  } = usePipelineNodeFormFields();
  const connectFrom = ref('');
  const connectTo = ref('');
  const edgeType = ref<EdgeType>('success');
  const formError = ref('');

  function resolveNodeId(requestedId: string): string {
    if (requestedId) {
      return requestedId;
    }

    let generatedId = createNodeId(nodeType.value);
    while (existingNodeIds.value.includes(generatedId)) {
      generatedId = createNodeId(nodeType.value);
    }

    return generatedId;
  }
  const trim = (value: string) => value.trim();
  const canSubmit = computed(() => canSubmitPipelineNodeForm(nodeType.value, getFormValues()));

  watch(toolName, () => {
    toolArgs.value = {};
    toolArgsValid.value = false;
  });

  function buildNode(requestedId: string): PipelineNode {
    const id = resolveNodeId(requestedId);

    switch (nodeType.value) {
      case 'prompt':
        return createNode('prompt', { id, ...buildPipelineNodeDraft('prompt', getFormValues()) });
      case 'sequentialThinking':
        return createNode('sequentialThinking', {
          id,
          ...buildPipelineNodeDraft('sequentialThinking', getFormValues()),
        });
      case 'tool':
        return createNode('tool', { id, ...buildPipelineNodeDraft('tool', getFormValues()) });
      default:
        return createNode('workflow', { id, ...buildPipelineNodeDraft('workflow', getFormValues()) });
    }
  }

  function handleSubmit() {
    if (!canSubmit.value || busy.value) return;

    try {
      const requestedId = trim(nodeId.value);
      if (requestedId && existingNodeIds.value.includes(requestedId)) {
        throw new Error('Node ID already exists in this pipeline.');
      }

      const node = buildNode(requestedId);
      const edges: PipelineEdge[] = [];

      if (connectFrom.value) {
        edges.push({ source: connectFrom.value, target: node.id, type: edgeType.value });
      }
      if (connectTo.value) {
        edges.push({ source: node.id, target: connectTo.value, type: edgeType.value });
      }

      formError.value = '';
      onSubmit([node], edges);
    } catch (error) {
      formError.value = error instanceof Error ? error.message : 'Failed to prepare graft payload.';
    }
  }

  return {
    workflowListId,
    nodeType,
    nodeId,
    workflowId,
    workflowAgent,
    workflowPrompt,
    promptAgent,
    promptTemplate,
    thinkingTopic,
    thinkingAgent,
    thinkingTotalThoughts,
    thinkingAnalysisType,
    toolName,
    toolArgs,
    toolArgsValid,
    connectFrom,
    connectTo,
    edgeType,
    formError,
    canSubmit,
    handleSubmit,
  };
}
