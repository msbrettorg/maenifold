<script setup lang="ts">
import { computed } from 'vue';
import { useAddNodeForm } from '../composables/useAddNodeForm';
import type { AgentProbeConfigOption, AgentProbeResult } from '../lib/agent-probe-types';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useConfigStore } from '../stores/config';
import type { PipelineNodeDraftByType } from '../lib/pipeline-node-factory';
import AgentProbePreview from './AgentProbePreview.vue';
import ToolPaletteFields from './ToolPaletteFields.vue';
import {
  buildAgentConfigDefaults,
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
} from '../composables/pipeline-node-form-helpers';

interface SubmitNode {
  (type: 'workflow', fields: PipelineNodeDraftByType['workflow']): void;
  (type: 'prompt', fields: PipelineNodeDraftByType['prompt']): void;
  (type: 'sequentialThinking', fields: PipelineNodeDraftByType['sequentialThinking']): void;
  (type: 'tool', fields: PipelineNodeDraftByType['tool']): void;
}

const props = defineProps<{
  workflows: string[];
  submitNode: SubmitNode;
}>();

const {
  workflowListId,
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
  handleSubmit,
} = useAddNodeForm({ submitNode: props.submitNode });

const addButtonLabel = computed(() => (nodeType.value === 'tool' ? 'Add Tool' : 'Add Node'));
const configStore = useConfigStore();
const capabilitiesStore = useAgentCapabilitiesStore();

function getProbe(agentName: string) {
  return capabilitiesStore.getProbe(agentName);
}

function formatAgentOptionLabel(agentName: string): string {
  return formatPipelineAgentOptionLabel(agentName, getProbe(agentName));
}

function getCapabilityBadges(agentName: string): string[] {
  return getPipelineAgentCapabilityBadges(getProbe(agentName));
}

function isProbeLoading(agentName: string): boolean {
  return capabilitiesStore.probeStatus === 'probing' && !getProbe(agentName);
}

function handlePromptAgentChange(event: Event): void {
  promptAgent.value = (event.target as HTMLSelectElement).value;
  promptAgentConfig.value = {};
}

function handleThinkingAgentChange(event: Event): void {
  thinkingAgent.value = (event.target as HTMLSelectElement).value;
  thinkingAgentConfig.value = {};
}

function ensurePromptDefaults(probe: AgentProbeResult | undefined): void {
  if (probe?.state === 'ready' && Object.keys(promptAgentConfig.value).length === 0) {
    promptAgentConfig.value = buildAgentConfigDefaults(probe) ?? {};
  }
}

function ensureWorkflowDefaults(probe: AgentProbeResult | undefined): void {
  if (probe?.state === 'ready' && Object.keys(workflowAgentConfig.value).length === 0) {
    workflowAgentConfig.value = buildAgentConfigDefaults(probe) ?? {};
  }
}

function ensureThinkingDefaults(probe: AgentProbeResult | undefined): void {
  if (probe?.state === 'ready' && Object.keys(thinkingAgentConfig.value).length === 0) {
    thinkingAgentConfig.value = buildAgentConfigDefaults(probe) ?? {};
  }
}

function updatePromptMode(modeId: string): void {
  promptAgentConfig.value = setAgentModeValue(promptAgentConfig.value, modeId) ?? {};
}

function updateWorkflowMode(modeId: string): void {
  workflowAgentConfig.value = setAgentModeValue(workflowAgentConfig.value, modeId) ?? {};
}

function updateThinkingMode(modeId: string): void {
  thinkingAgentConfig.value = setAgentModeValue(thinkingAgentConfig.value, modeId) ?? {};
}

function updateWorkflowModel(event: Event): void {
  workflowAgentConfig.value = setAgentModelValue(
    workflowAgentConfig.value,
    (event.target as HTMLSelectElement).value,
  ) ?? {};
}

function updatePromptModel(event: Event): void {
  promptAgentConfig.value = setAgentModelValue(
    promptAgentConfig.value,
    (event.target as HTMLSelectElement).value,
  ) ?? {};
}

function updateWorkflowThoughtLevel(event: Event): void {
  workflowAgentConfig.value = setAgentThoughtLevelValue(
    workflowAgentConfig.value,
    (event.target as HTMLSelectElement).value,
  ) ?? {};
}

function updateThinkingModel(event: Event): void {
  thinkingAgentConfig.value = setAgentModelValue(
    thinkingAgentConfig.value,
    (event.target as HTMLSelectElement).value,
  ) ?? {};
}

function updatePromptThoughtLevel(event: Event): void {
  promptAgentConfig.value = setAgentThoughtLevelValue(
    promptAgentConfig.value,
    (event.target as HTMLSelectElement).value,
  ) ?? {};
}

function updateThinkingThoughtLevel(event: Event): void {
  thinkingAgentConfig.value = setAgentThoughtLevelValue(
    thinkingAgentConfig.value,
    (event.target as HTMLSelectElement).value,
  ) ?? {};
}

function updateWorkflowConfigOption(option: AgentProbeConfigOption, event: Event): void {
  workflowAgentConfig.value = setAgentConfigOptionValue(
    workflowAgentConfig.value,
    option,
    (event.target as HTMLSelectElement).value,
  ) ?? {};
}

function updatePromptConfigOption(option: AgentProbeConfigOption, event: Event): void {
  promptAgentConfig.value = setAgentConfigOptionValue(
    promptAgentConfig.value,
    option,
    (event.target as HTMLSelectElement).value,
  ) ?? {};
}

function updateThinkingConfigOption(option: AgentProbeConfigOption, event: Event): void {
  thinkingAgentConfig.value = setAgentConfigOptionValue(
    thinkingAgentConfig.value,
    option,
    (event.target as HTMLSelectElement).value,
  ) ?? {};
}

function promptProbe(): AgentProbeResult | undefined {
  return promptAgent.value ? getProbe(promptAgent.value) : undefined;
}

function workflowProbe(): AgentProbeResult | undefined {
  return workflowAgent.value ? getProbe(workflowAgent.value) : undefined;
}

function thinkingProbe(): AgentProbeResult | undefined {
  return thinkingAgent.value ? getProbe(thinkingAgent.value) : undefined;
}

function readyWorkflowProbe(): AgentProbeResult | undefined {
  const probe = workflowProbe();
  ensureWorkflowDefaults(probe);
  return probe?.state === 'ready' ? probe : undefined;
}

function readyPromptProbe(): AgentProbeResult | undefined {
  const probe = promptProbe();
  ensurePromptDefaults(probe);
  return probe?.state === 'ready' ? probe : undefined;
}

function readyThinkingProbe(): AgentProbeResult | undefined {
  const probe = thinkingProbe();
  ensureThinkingDefaults(probe);
  return probe?.state === 'ready' ? probe : undefined;
}
</script>

<template>
  <form class="composer-toolbar" @submit.prevent="handleSubmit">
    <div class="toolbar-row toolbar-row-top">
      <label class="toolbar-type-field">
        <span>Node type</span>
        <select v-model="nodeType" class="field-input" aria-label="Node type">
          <option value="workflow">Workflow</option>
          <option value="prompt">Prompt</option>
          <option value="sequentialThinking">Sequential Thinking</option>
          <option value="tool">Tool</option>
        </select>
      </label>
      <div class="toolbar-actions">
        <button type="submit" class="btn btn-primary" :disabled="!canAdd">{{ addButtonLabel }}</button>
      </div>
    </div>

    <div class="toolbar-section">
      <template v-if="nodeType === 'workflow'">
        <label>
          <span>Workflow ID</span>
          <input
            v-model.trim="workflowId"
            class="field-input"
            :list="workflowListId"
            type="text"
            placeholder="Workflow ID"
          />
        </label>
        <datalist :id="workflowListId">
          <option v-for="workflow in workflows" :key="workflow" :value="workflow" />
        </datalist>
        <label>
          <span>Agent</span>
          <select v-model="workflowAgent" class="field-input" data-testid="workflow-agent-select">
            <option value="">Select agent…</option>
            <option v-for="name in configStore.agentNames" :key="name" :value="name">
              {{ formatAgentOptionLabel(name) }}
            </option>
          </select>
        </label>
        <div v-if="workflowAgent" class="field-wide">
          <div v-if="getCapabilityBadges(workflowAgent).length" class="agent-capabilities">
            <span v-for="badge in getCapabilityBadges(workflowAgent)" :key="badge" class="agent-capability">{{ badge }}</span>
          </div>
          <AgentProbePreview compact :agent-name="workflowAgent" :probe="getProbe(workflowAgent)" />
        </div>
        <div v-if="workflowAgent" class="field-wide agent-config-panel">
          <template v-if="readyWorkflowProbe()">
            <div v-if="readyWorkflowProbe()?.modeOptions.length" class="agent-mode-badges">
              <button
                v-for="mode in readyWorkflowProbe()?.modeOptions"
                :key="mode.id"
                type="button"
                class="mode-badge"
                :class="{ selected: getAgentModeValue(workflowAgentConfig, readyWorkflowProbe()) === mode.id }"
                :data-testid="`workflow-mode-badge-${mode.id}`"
                @click="updateWorkflowMode(mode.id)"
              >
                {{ mode.name }}
              </button>
            </div>
            <div class="agent-config-grid">
              <label v-if="readyWorkflowProbe()?.modelOptions.length">
                <span>Model</span>
                <select
                  class="field-input"
                  data-testid="workflow-model-select"
                  :value="getAgentModelValue(workflowAgentConfig, readyWorkflowProbe())"
                  @change="updateWorkflowModel"
                >
                  <option
                    v-for="model in readyWorkflowProbe()?.modelOptions"
                    :key="model.modelId"
                    :value="model.modelId"
                  >
                    {{ model.name }}
                  </option>
                </select>
              </label>
              <label v-if="readyWorkflowProbe()?.thoughtLevelConfigOption">
                <span>{{ readyWorkflowProbe()?.thoughtLevelConfigOption?.name }}</span>
                <select
                  class="field-input"
                  data-testid="workflow-thought-level-select"
                  :value="getAgentThoughtLevelValue(workflowAgentConfig, readyWorkflowProbe())"
                  @change="updateWorkflowThoughtLevel"
                >
                  <template
                    v-for="entry in readyWorkflowProbe()?.thoughtLevelConfigOption?.options ?? []"
                    :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"
                  >
                    <optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name">
                      <option
                        v-for="groupOption in entry.options"
                        :key="groupOption.value"
                        :value="groupOption.value"
                      >
                        {{ groupOption.name }}
                      </option>
                    </optgroup>
                    <option v-else :value="entry.value">{{ entry.name }}</option>
                  </template>
                </select>
              </label>
              <label
                v-for="option in getEditableAgentConfigOptions(readyWorkflowProbe())"
                :key="option.id"
              >
                <span>{{ option.name }}</span>
                <select
                  class="field-input"
                  :data-testid="`workflow-config-${option.id}`"
                  :value="getAgentConfigOptionValue(workflowAgentConfig, option)"
                  @change="updateWorkflowConfigOption(option, $event)"
                >
                  <template
                    v-for="entry in option.options"
                    :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"
                  >
                    <optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name">
                      <option
                        v-for="groupOption in entry.options"
                        :key="groupOption.value"
                        :value="groupOption.value"
                      >
                        {{ groupOption.name }}
                      </option>
                    </optgroup>
                    <option v-else :value="entry.value">{{ entry.name }}</option>
                  </template>
                </select>
              </label>
            </div>
          </template>
        </div>
        <label class="field-wide">
          <span>Prompt</span>
          <textarea
            v-model.trim="workflowPrompt"
            class="field-textarea"
            rows="3"
            placeholder="Optional workflow prompt"
          />
        </label>
      </template>

      <template v-else-if="nodeType === 'prompt'">
        <label>
          <span>Agent</span>
          <select class="field-input" data-testid="prompt-agent-select" :value="promptAgent" @change="handlePromptAgentChange">
            <option value="" disabled>Select agent…</option>
            <option v-for="name in configStore.agentNames" :key="name" :value="name">
              {{ formatAgentOptionLabel(name) }}
            </option>
          </select>
        </label>
        <AgentProbePreview
          v-if="promptAgent"
          compact
          class="field-wide"
          :agent-name="promptAgent"
          :probe="getProbe(promptAgent)"
          :loading="isProbeLoading(promptAgent)"
        />
        <div v-if="promptAgent" class="field-wide agent-config-panel">
          <template v-if="readyPromptProbe()">
            <div v-if="readyPromptProbe()?.modeOptions.length" class="agent-mode-badges">
              <button
                v-for="mode in readyPromptProbe()?.modeOptions"
                :key="mode.id"
                type="button"
                class="mode-badge"
                :class="{ selected: getAgentModeValue(promptAgentConfig, readyPromptProbe()) === mode.id }"
                :data-testid="`prompt-mode-badge-${mode.id}`"
                @click="updatePromptMode(mode.id)"
              >
                {{ mode.name }}
              </button>
            </div>
            <div class="agent-config-grid">
              <label v-if="readyPromptProbe()?.modelOptions.length">
                <span>Model</span>
                <select
                  class="field-input"
                  data-testid="prompt-model-select"
                  :value="getAgentModelValue(promptAgentConfig, readyPromptProbe())"
                  @change="updatePromptModel"
                >
                  <option
                    v-for="model in readyPromptProbe()?.modelOptions"
                    :key="model.modelId"
                    :value="model.modelId"
                  >
                    {{ model.name }}
                  </option>
                </select>
              </label>
              <label v-if="readyPromptProbe()?.thoughtLevelConfigOption">
                <span>{{ readyPromptProbe()?.thoughtLevelConfigOption?.name }}</span>
                <select
                  class="field-input"
                  data-testid="prompt-thought-level-select"
                  :value="getAgentThoughtLevelValue(promptAgentConfig, readyPromptProbe())"
                  @change="updatePromptThoughtLevel"
                >
                  <template
                    v-for="entry in readyPromptProbe()?.thoughtLevelConfigOption?.options ?? []"
                    :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"
                  >
                    <optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name">
                      <option
                        v-for="groupOption in entry.options"
                        :key="groupOption.value"
                        :value="groupOption.value"
                      >
                        {{ groupOption.name }}
                      </option>
                    </optgroup>
                    <option v-else :value="entry.value">{{ entry.name }}</option>
                  </template>
                </select>
              </label>
              <label
                v-for="option in getEditableAgentConfigOptions(readyPromptProbe())"
                :key="option.id"
              >
                <span>{{ option.name }}</span>
                <select
                  class="field-input"
                  :data-testid="`prompt-config-${option.id}`"
                  :value="getAgentConfigOptionValue(promptAgentConfig, option)"
                  @change="updatePromptConfigOption(option, $event)"
                >
                  <template
                    v-for="entry in option.options"
                    :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"
                  >
                    <optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name">
                      <option
                        v-for="groupOption in entry.options"
                        :key="groupOption.value"
                        :value="groupOption.value"
                      >
                        {{ groupOption.name }}
                      </option>
                    </optgroup>
                    <option v-else :value="entry.value">{{ entry.name }}</option>
                  </template>
                </select>
              </label>
            </div>
          </template>
        </div>
        <label class="field-wide">
          <span>Prompt</span>
          <textarea
            v-model.trim="promptTemplate"
            class="field-textarea"
            rows="3"
            placeholder="Required prompt template"
          />
        </label>
      </template>

      <template v-else-if="nodeType === 'sequentialThinking'">
        <label>
          <span>Topic</span>
          <input v-model.trim="thinkingTopic" class="field-input" type="text" placeholder="Topic" />
        </label>
        <label>
          <span>Agent</span>
          <select class="field-input" data-testid="thinking-agent-select" :value="thinkingAgent" @change="handleThinkingAgentChange">
            <option value="" disabled>Select agent…</option>
            <option v-for="name in configStore.agentNames" :key="name" :value="name">
              {{ formatAgentOptionLabel(name) }}
            </option>
          </select>
        </label>
        <AgentProbePreview
          v-if="thinkingAgent"
          compact
          class="field-wide"
          :agent-name="thinkingAgent"
          :probe="getProbe(thinkingAgent)"
          :loading="isProbeLoading(thinkingAgent)"
        />
        <div v-if="thinkingAgent" class="field-wide agent-config-panel">
          <template v-if="readyThinkingProbe()">
            <div v-if="readyThinkingProbe()?.modeOptions.length" class="agent-mode-badges">
              <button
                v-for="mode in readyThinkingProbe()?.modeOptions"
                :key="mode.id"
                type="button"
                class="mode-badge"
                :class="{ selected: getAgentModeValue(thinkingAgentConfig, readyThinkingProbe()) === mode.id }"
                :data-testid="`thinking-mode-badge-${mode.id}`"
                @click="updateThinkingMode(mode.id)"
              >
                {{ mode.name }}
              </button>
            </div>
            <div class="agent-config-grid">
              <label v-if="readyThinkingProbe()?.modelOptions.length">
                <span>Model</span>
                <select
                  class="field-input"
                  data-testid="thinking-model-select"
                  :value="getAgentModelValue(thinkingAgentConfig, readyThinkingProbe())"
                  @change="updateThinkingModel"
                >
                  <option
                    v-for="model in readyThinkingProbe()?.modelOptions"
                    :key="model.modelId"
                    :value="model.modelId"
                  >
                    {{ model.name }}
                  </option>
                </select>
              </label>
              <label v-if="readyThinkingProbe()?.thoughtLevelConfigOption">
                <span>{{ readyThinkingProbe()?.thoughtLevelConfigOption?.name }}</span>
                <select
                  class="field-input"
                  data-testid="thinking-thought-level-select"
                  :value="getAgentThoughtLevelValue(thinkingAgentConfig, readyThinkingProbe())"
                  @change="updateThinkingThoughtLevel"
                >
                  <template
                    v-for="entry in readyThinkingProbe()?.thoughtLevelConfigOption?.options ?? []"
                    :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"
                  >
                    <optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name">
                      <option
                        v-for="groupOption in entry.options"
                        :key="groupOption.value"
                        :value="groupOption.value"
                      >
                        {{ groupOption.name }}
                      </option>
                    </optgroup>
                    <option v-else :value="entry.value">{{ entry.name }}</option>
                  </template>
                </select>
              </label>
              <label
                v-for="option in getEditableAgentConfigOptions(readyThinkingProbe())"
                :key="option.id"
              >
                <span>{{ option.name }}</span>
                <select
                  class="field-input"
                  :data-testid="`thinking-config-${option.id}`"
                  :value="getAgentConfigOptionValue(thinkingAgentConfig, option)"
                  @change="updateThinkingConfigOption(option, $event)"
                >
                  <template
                    v-for="entry in option.options"
                    :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"
                  >
                    <optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name">
                      <option
                        v-for="groupOption in entry.options"
                        :key="groupOption.value"
                        :value="groupOption.value"
                      >
                        {{ groupOption.name }}
                      </option>
                    </optgroup>
                    <option v-else :value="entry.value">{{ entry.name }}</option>
                  </template>
                </select>
              </label>
            </div>
          </template>
        </div>
        <label>
          <span>Total thoughts</span>
          <input
            v-model="thinkingTotalThoughts"
            class="field-input"
            type="number"
            min="1"
            inputmode="numeric"
            placeholder="5"
          />
        </label>
        <label>
          <span>Analysis type</span>
          <input
            v-model.trim="thinkingAnalysisType"
            class="field-input"
            type="text"
            placeholder="Optional analysis type"
          />
        </label>
      </template>

      <template v-else>
        <ToolPaletteFields
          :active="nodeType === 'tool'"
          :tool-name="toolName"
          :agent-name="toolAgent"
          :agent-config="toolAgentConfig"
          :tool-args="toolArgs"
          :can-submit="canAdd"
          @update:tool-name="toolName = $event"
          @update:agent-name="toolAgent = $event"
          @update:agent-config="toolAgentConfig = $event"
          @update:tool-args="toolArgs = $event"
          @validation="toolArgsValid = $event.valid"
          @submit-request="handleSubmit"
        />
      </template>
    </div>
  </form>
</template>

<style scoped>
.composer-toolbar {
  display: grid;
  gap: 1rem;
  align-content: start;
  height: 100%;
  padding: 1rem;
  border-right: 1px solid var(--border-default);
  background: color-mix(in srgb, var(--bg-main) 96%, transparent);
  overflow-y: auto;
}

.toolbar-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: flex-end;
}

.toolbar-type-field {
  min-width: 12rem;
}

.toolbar-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.toolbar-section {
  display: grid;
  gap: 0.8rem;
  grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
}

label {
  display: grid;
  gap: 0.35rem;
  min-width: 0;
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.field-wide {
  grid-column: 1 / -1;
}

.agent-capabilities,
.agent-mode-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.agent-capability,
.mode-badge {
  display: inline-flex;
  align-items: center;
  padding: 0.15rem 0.45rem;
  border: 1px solid var(--border-default);
  border-radius: 999px;
  font-size: var(--text-xs);
}

.agent-config-panel {
  display: grid;
  gap: 0.65rem;
  padding: 0.75rem;
  border: 1px solid var(--border-default);
  border-radius: 0.75rem;
  background: color-mix(in srgb, var(--bg-panel) 70%, transparent);
}

.agent-config-grid {
  display: grid;
  gap: 0.65rem;
  grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
}

.mode-badge {
  background: var(--bg-main);
  color: var(--text-secondary);
  cursor: pointer;
}

.mode-badge.selected {
  border-color: var(--border-focus);
  color: var(--text-primary);
  background: color-mix(in srgb, var(--border-focus) 45%, transparent);
}
</style>
