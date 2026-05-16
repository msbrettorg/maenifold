import { computed, ref, watch } from 'vue';
import type { PipelineNodeDraftByType } from '../lib/pipeline-node-factory';
import type { PipelineNodeType } from '../lib/pipeline-types-node';
import {
  buildPipelineNodeDraft,
  canSubmitPipelineNodeForm,
  DEFAULT_PIPELINE_NODE_THOUGHTS,
  usePipelineNodeFormFields,
} from './pipeline-node-form-helpers';
import { useNodePaletteStore } from '../stores/node-palette';

interface SubmitNode {
  (type: 'workflow', fields: PipelineNodeDraftByType['workflow']): void;
  (type: 'prompt', fields: PipelineNodeDraftByType['prompt']): void;
  (type: 'sequentialThinking', fields: PipelineNodeDraftByType['sequentialThinking']): void;
  (type: 'tool', fields: PipelineNodeDraftByType['tool']): void;
}

interface UseAddNodeFormOptions {
  submitNode: SubmitNode;
}

export function useAddNodeForm({ submitNode }: UseAddNodeFormOptions) {
  const nodePaletteStore = useNodePaletteStore();
  const workflowListId = 'pipeline-composer-workflows';
  const panelOpen = ref(true);
  const nodeType = ref<PipelineNodeType>('workflow');
  const {
    workflowId,
    workflowAgent,
    workflowAgentConfig,
    workflowPrompt,
    promptAgent,
    promptAgentConfig,
    promptTemplate,
    thinkingTopic,
    thinkingAgent,
    thinkingAgentConfig,
    thinkingTotalThoughts,
    thinkingAnalysisType,
    toolName,
    toolAgent,
    toolAgentConfig,
    toolArgs,
    toolArgsValid,
    cwd,
    getFormValues,
  } = usePipelineNodeFormFields();
  const canAdd = computed(() => canSubmitPipelineNodeForm(nodeType.value, getFormValues()));

  watch(toolName, () => {
    toolArgs.value = {};
    toolArgsValid.value = true;
  });

  watch(workflowAgent, () => {
    workflowAgentConfig.value = {};
  });

  watch(
    toolName,
    (value) => {
      nodePaletteStore.setSelectedTool(value);
    },
    { immediate: true },
  );

  watch(
    nodeType,
    (value) => {
      if (
        value === 'tool'
        && !toolName.value
        && nodePaletteStore.selectedToolName
      ) {
        toolName.value = nodePaletteStore.selectedToolName;
      }
    },
    { immediate: true },
  );


  function resetCurrentDraft(): void {
    switch (nodeType.value) {
      case 'prompt':
        promptAgent.value = '';
        promptAgentConfig.value = {};
        promptTemplate.value = '';
        break;
      case 'sequentialThinking':
        thinkingTopic.value = '';
        thinkingAgent.value = '';
        thinkingAgentConfig.value = {};
        thinkingTotalThoughts.value = DEFAULT_PIPELINE_NODE_THOUGHTS;
        thinkingAnalysisType.value = '';
        break;
      case 'tool':
        // Preserve palette selection and current tool configuration for repeated adds.
        break;
      default:
        workflowId.value = '';
        workflowAgent.value = '';
        workflowAgentConfig.value = {};
        workflowPrompt.value = '';
        break;
    }
  }

  function handleSubmit(): void {
    if (!canAdd.value) return;

    switch (nodeType.value) {
      case 'prompt':
        submitNode('prompt', buildPipelineNodeDraft('prompt', getFormValues()));
        break;
      case 'sequentialThinking':
        submitNode(
          'sequentialThinking',
          buildPipelineNodeDraft('sequentialThinking', getFormValues())
        );
        break;
      case 'tool':
        submitNode('tool', buildPipelineNodeDraft('tool', getFormValues()));
        nodePaletteStore.recordRecentTool(toolName.value);
        break;
      default:
        submitNode('workflow', buildPipelineNodeDraft('workflow', getFormValues()));
        break;
    }

    resetCurrentDraft();
  }

  return {
    workflowListId,
    panelOpen,
    nodeType,
    workflowId,
    workflowAgent,
    workflowAgentConfig,
    workflowPrompt,
    promptAgent,
    promptAgentConfig,
    promptTemplate,
    thinkingTopic,
    thinkingAgent,
    thinkingAgentConfig,
    thinkingTotalThoughts,
    thinkingAnalysisType,
    toolName,
    toolAgent,
    toolAgentConfig,
    toolArgs,
    toolArgsValid,
    canAdd,
    cwd,
    handleSubmit,
  };
}
