import { computed } from 'vue';
import type { AgentProbeConfigOption, AgentProbeResult } from '../lib/agent-probe-types-defs';
import {
  formatPipelineAgentOptionLabel,
  getAgentConfigOptionValue,
  getAgentModelValue,
  getAgentThoughtLevelValue,
  getEditableAgentConfigOptions,
  getPipelineAgentCapabilityBadges,
  getAgentModeValue,
  isSessionConfigSelectGroup,
  setAgentConfigOptionValue,
  setAgentModelValue,
  setAgentModeValue,
  setAgentThoughtLevelValue,
} from './pipeline-node-form-helpers';
import { useReadyAgentProbe } from './useReadyAgentProbe';
import type { PipelineNodeAgentConfig, PipelineNodeType } from '../lib/pipeline-types';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useConfigStore } from '../stores/config';

export interface AddNodeFormFieldsProps {
  workflows: string[];
  workflowListId: string;
  nodeType: PipelineNodeType;
  workflowId: string;
  workflowAgent: string;
  workflowAgentConfig: PipelineNodeAgentConfig;
  workflowPrompt: string;
  promptAgent: string;
  promptAgentConfig: PipelineNodeAgentConfig;
  promptTemplate: string;
  thinkingTopic: string;
  thinkingAgent: string;
  thinkingAgentConfig: PipelineNodeAgentConfig;
  thinkingTotalThoughts: string;
  thinkingAnalysisType: string;
  toolName: string;
  toolAgent: string;
  toolAgentConfig: PipelineNodeAgentConfig;
  toolArgs: Record<string, unknown>;
  toolArgsValid: boolean;
  canSubmit: boolean;
  cwd: string;
}

export interface AddNodeFormFieldsEmit {
  (event: 'update:workflowId', value: string): void;
  (event: 'update:workflowAgent', value: string): void;
  (event: 'update:workflowAgentConfig', value: PipelineNodeAgentConfig): void;
  (event: 'update:workflowPrompt', value: string): void;
  (event: 'update:promptAgent', value: string): void;
  (event: 'update:promptAgentConfig', value: PipelineNodeAgentConfig): void;
  (event: 'update:promptTemplate', value: string): void;
  (event: 'update:thinkingTopic', value: string): void;
  (event: 'update:thinkingAgent', value: string): void;
  (event: 'update:thinkingAgentConfig', value: PipelineNodeAgentConfig): void;
  (event: 'update:thinkingTotalThoughts', value: string): void;
  (event: 'update:thinkingAnalysisType', value: string): void;
  (event: 'update:toolName', value: string): void;
  (event: 'update:toolAgent', value: string): void;
  (event: 'update:toolAgentConfig', value: PipelineNodeAgentConfig): void;
  (event: 'update:toolArgs', value: Record<string, unknown>): void;
  (event: 'update:cwd', value: string | undefined): void;
  (event: 'validation', value: { valid: boolean }): void;
  (event: 'submitRequest'): void;
}

export function useAddNodeFormFieldBindings(
  props: AddNodeFormFieldsProps,
  emit: AddNodeFormFieldsEmit,
) {
  const configStore = useConfigStore();
  const capabilitiesStore = useAgentCapabilitiesStore();

  function getProbe(agentName: string): AgentProbeResult | undefined {
    return capabilitiesStore.getProbe(agentName);
  }

  const workflowProbe = computed(() => props.workflowAgent ? getProbe(props.workflowAgent) : undefined);
  const promptProbe = computed(() => props.promptAgent ? getProbe(props.promptAgent) : undefined);
  const thinkingProbe = computed(() => props.thinkingAgent ? getProbe(props.thinkingAgent) : undefined);

  const readyWorkflowProbe = useReadyAgentProbe(workflowProbe, computed(() => props.workflowAgentConfig), (value) => emit('update:workflowAgentConfig', value));
  const readyPromptProbe = useReadyAgentProbe(promptProbe, computed(() => props.promptAgentConfig), (value) => emit('update:promptAgentConfig', value));
  const readyThinkingProbe = useReadyAgentProbe(thinkingProbe, computed(() => props.thinkingAgentConfig), (value) => emit('update:thinkingAgentConfig', value));

  const hasAgentControls = computed(() => props.nodeType !== 'tool');
  const currentAgentName = computed(() => {
    if (props.nodeType === 'workflow') return props.workflowAgent;
    if (props.nodeType === 'prompt') return props.promptAgent;
    return props.thinkingAgent;
  });
  const currentAgentConfig = computed(() => {
    if (props.nodeType === 'workflow') return props.workflowAgentConfig;
    if (props.nodeType === 'prompt') return props.promptAgentConfig;
    return props.thinkingAgentConfig;
  });
  const currentProbe = computed(() => {
    if (props.nodeType === 'workflow') return workflowProbe.value;
    if (props.nodeType === 'prompt') return promptProbe.value;
    return thinkingProbe.value;
  });
  const currentReadyProbe = computed(() => {
    if (props.nodeType === 'workflow') return readyWorkflowProbe.value;
    if (props.nodeType === 'prompt') return readyPromptProbe.value;
    return readyThinkingProbe.value;
  });
  const currentTestPrefix = computed(() => props.nodeType === 'sequentialThinking' ? 'thinking' : props.nodeType);
  const currentEditableOptions = computed(() => getEditableAgentConfigOptions(currentReadyProbe.value));

  function formatAgentOptionLabel(agentName: string): string {
    return formatPipelineAgentOptionLabel(agentName, getProbe(agentName));
  }

  function getCapabilityBadges(agentName: string): string[] {
    return getPipelineAgentCapabilityBadges(getProbe(agentName));
  }

  function isProbeLoading(agentName: string): boolean {
    return capabilitiesStore.probeStatus === 'probing' && !getProbe(agentName);
  }

  function updateCurrentAgentName(event: Event): void {
    const value = (event.target as HTMLSelectElement).value;
    if (props.nodeType === 'workflow') {
      emit('update:workflowAgent', value);
      emit('update:workflowAgentConfig', {});
      return;
    }
    if (props.nodeType === 'prompt') {
      emit('update:promptAgent', value);
      emit('update:promptAgentConfig', {});
      return;
    }
    emit('update:thinkingAgent', value);
    emit('update:thinkingAgentConfig', {});
  }

  function applyCurrentAgentConfig(value: PipelineNodeAgentConfig | undefined): void {
    const nextValue = value ?? {};
    if (props.nodeType === 'workflow') {
      emit('update:workflowAgentConfig', nextValue);
      return;
    }
    if (props.nodeType === 'prompt') {
      emit('update:promptAgentConfig', nextValue);
      return;
    }
    emit('update:thinkingAgentConfig', nextValue);
  }

  function updateCurrentMode(modeId: string): void {
    applyCurrentAgentConfig(setAgentModeValue(currentAgentConfig.value, modeId));
  }

  function updateCurrentModel(event: Event): void {
    applyCurrentAgentConfig(setAgentModelValue(currentAgentConfig.value, (event.target as HTMLSelectElement).value));
  }

  function updateCurrentThoughtLevel(event: Event): void {
    applyCurrentAgentConfig(setAgentThoughtLevelValue(currentAgentConfig.value, (event.target as HTMLSelectElement).value));
  }

  function updateCurrentConfigOption(option: AgentProbeConfigOption, event: Event): void {
    applyCurrentAgentConfig(
      setAgentConfigOptionValue(currentAgentConfig.value, option, (event.target as HTMLSelectElement).value),
    );
  }

  return {
    configStore,
    workflowProbe,
    hasAgentControls,
    currentAgentName,
    currentAgentConfig,
    currentProbe,
    currentReadyProbe,
    currentTestPrefix,
    currentEditableOptions,
    formatAgentOptionLabel,
    getCapabilityBadges,
    isProbeLoading,
    updateCurrentAgentName,
    updateCurrentMode,
    updateCurrentModel,
    updateCurrentThoughtLevel,
    updateCurrentConfigOption,
    isSessionConfigSelectGroup,
    getAgentModeValue,
    getAgentModelValue,
    getAgentThoughtLevelValue,
    getAgentConfigOptionValue,
  };
}
