// Public aggregator for pipeline node form helpers. The actual implementations
// live in:
//   - pipeline-node-form-types.ts          (PipelineNodeFormValues, DEFAULT_PIPELINE_NODE_THOUGHTS)
//   - pipeline-node-form-agent-config.ts    (agent config getters/setters, probe helpers)
//   - pipeline-node-form-drafts.ts          (draft builders, validators)
// This file owns the form refs composable and select-options helpers, and
// re-exports everything for backwards compatibility.
import { ref, type Ref } from 'vue';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types';
import type {
  SessionConfigSelectGroup,
  SessionConfigSelectOption,
  SessionConfigSelectOptions,
} from '../lib/types';
import {
  DEFAULT_PIPELINE_NODE_THOUGHTS,
  type PipelineNodeFormValues,
} from './pipeline-node-form-types';

export type { PipelineNodeFormValues } from './pipeline-node-form-types';
export { DEFAULT_PIPELINE_NODE_THOUGHTS } from './pipeline-node-form-types';
export {
  buildAgentConfigDefaults,
  clonePipelineNodeAgentConfig,
  formatPipelineAgentOptionLabel,
  getAgentConfigOptionValue,
  getAgentModeValue,
  getAgentModelValue,
  getAgentThoughtLevelValue,
  getEditableAgentConfigOptions,
  getPipelineAgentCapabilityBadges,
  normalizePipelineNodeAgentConfig,
  setAgentConfigOptionValue,
  setAgentModeValue,
  setAgentModelValue,
  setAgentThoughtLevelValue,
} from './pipeline-node-form-agent-config';
export {
  buildPipelineNodeDraft,
  buildWorkflowDraft,
  canSubmitPipelineNodeForm,
  parsePipelineToolArgs,
} from './pipeline-node-form-drafts';

export interface PipelineNodeFormRefs {
  workflowId: Ref<string>;
  workflowAgent: Ref<string>;
  workflowAgentConfig: Ref<PipelineNodeAgentConfig>;
  workflowPrompt: Ref<string>;
  promptAgent: Ref<string>;
  promptAgentConfig: Ref<PipelineNodeAgentConfig>;
  promptTemplate: Ref<string>;
  thinkingTopic: Ref<string>;
  thinkingAgent: Ref<string>;
  thinkingAgentConfig: Ref<PipelineNodeAgentConfig>;
  thinkingTotalThoughts: Ref<string>;
  thinkingAnalysisType: Ref<string>;
  toolName: Ref<string>;
  toolAgent: Ref<string>;
  toolAgentConfig: Ref<PipelineNodeAgentConfig>;
  toolArgs: Ref<Record<string, unknown>>;
  toolArgsValid: Ref<boolean>;
  cwd: Ref<string>;
  getFormValues: () => PipelineNodeFormValues;
}

export function usePipelineNodeFormFields(): PipelineNodeFormRefs {
  const workflowId = ref('');
  const workflowAgent = ref('');
  const workflowAgentConfig = ref<PipelineNodeAgentConfig>({});
  const workflowPrompt = ref('');
  const promptAgent = ref('');
  const promptAgentConfig = ref<PipelineNodeAgentConfig>({});
  const promptTemplate = ref('');
  const thinkingTopic = ref('');
  const thinkingAgent = ref('');
  const thinkingAgentConfig = ref<PipelineNodeAgentConfig>({});
  const thinkingTotalThoughts = ref(DEFAULT_PIPELINE_NODE_THOUGHTS);
  const thinkingAnalysisType = ref('');
  const toolName = ref('');
  const toolAgent = ref('');
  const toolAgentConfig = ref<PipelineNodeAgentConfig>({});
  const toolArgs = ref<Record<string, unknown>>({});
  const toolArgsValid = ref(true);
  const cwd = ref('');

  return {
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
    getFormValues: () => ({
      workflowId: workflowId.value,
      workflowAgent: workflowAgent.value,
      workflowAgentConfig: workflowAgentConfig.value,
      workflowPrompt: workflowPrompt.value,
      promptAgent: promptAgent.value,
      promptAgentConfig: promptAgentConfig.value,
      promptTemplate: promptTemplate.value,
      thinkingTopic: thinkingTopic.value,
      thinkingAgent: thinkingAgent.value,
      thinkingAgentConfig: thinkingAgentConfig.value,
      thinkingTotalThoughts: thinkingTotalThoughts.value,
      thinkingAnalysisType: thinkingAnalysisType.value,
      toolName: toolName.value,
      toolAgent: toolAgent.value,
      toolAgentConfig: toolAgentConfig.value,
      toolArgs: toolArgs.value,
      toolArgsValid: toolArgsValid.value,
      cwd: cwd.value,
    }),
  };
}

export function isSessionConfigSelectGroup(
  option: SessionConfigSelectOption | SessionConfigSelectGroup,
): option is SessionConfigSelectGroup {
  return 'group' in option;
}

export function flattenSessionConfigOptions(
  options: SessionConfigSelectOptions,
): SessionConfigSelectOption[] {
  return options.flatMap((entry) => (isSessionConfigSelectGroup(entry) ? entry.options : [entry]));
}
