<script setup lang="ts">
import {
  useAddNodeFormFieldBindings,
  type AddNodeFormFieldsEmit,
  type AddNodeFormFieldsProps,
} from '../composables/useAddNodeFormFieldBindings';
import AgentProbePreview from './AgentProbePreview.vue';
import ToolPaletteFields from './ToolPaletteFields.vue';

const props = defineProps<AddNodeFormFieldsProps>();
const emit = defineEmits<AddNodeFormFieldsEmit>();

const {
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
} = useAddNodeFormFieldBindings(props, emit);
</script>

<template>
  <div class="toolbar-section">
    <template v-if="nodeType === 'workflow'">
      <label>
        <span>Workflow ID</span>
        <input
          class="field-input"
          :value="workflowId"
          :list="workflowListId"
          type="text"
          placeholder="Workflow ID"
          @input="emit('update:workflowId', ($event.target as HTMLInputElement).value.trim())"
        />
      </label>
      <datalist :id="workflowListId">
        <option v-for="workflow in workflows" :key="workflow" :value="workflow" />
      </datalist>
    </template>

    <label v-else-if="nodeType === 'sequentialThinking'">
      <span>Topic</span>
      <input
        class="field-input"
        :value="thinkingTopic"
        type="text"
        placeholder="Topic"
        @input="emit('update:thinkingTopic', ($event.target as HTMLInputElement).value.trim())"
      />
    </label>

    <template v-if="hasAgentControls">
      <label>
        <span>Agent</span>
        <select
          class="field-input"
          :data-testid="`${currentTestPrefix}-agent-select`"
          :value="currentAgentName"
          @change="updateCurrentAgentName"
        >
          <option value="">Select agent…</option>
          <option v-for="name in configStore.agentNames" :key="name" :value="name">
            {{ formatAgentOptionLabel(name) }}
          </option>
        </select>
      </label>

      <div v-if="nodeType === 'workflow' && workflowAgent" class="field-wide">
        <div v-if="getCapabilityBadges(workflowAgent).length" class="agent-capabilities">
          <span v-for="badge in getCapabilityBadges(workflowAgent)" :key="badge" class="agent-capability">
            {{ badge }}
          </span>
        </div>
        <AgentProbePreview compact :agent-name="workflowAgent" :probe="workflowProbe" />
      </div>

      <AgentProbePreview
        v-else-if="currentAgentName"
        compact
        class="field-wide"
        :agent-name="currentAgentName"
        :probe="currentProbe"
        :loading="isProbeLoading(currentAgentName)"
      />

      <div v-if="currentAgentName" class="field-wide agent-config-panel">
        <template v-if="currentReadyProbe">
          <div v-if="currentReadyProbe.modeOptions.length" class="agent-mode-badges">
            <button
              v-for="mode in currentReadyProbe.modeOptions"
              :key="mode.id"
              type="button"
              class="mode-badge"
              :class="{ selected: getAgentModeValue(currentAgentConfig, currentReadyProbe) === mode.id }"
              :data-testid="`${currentTestPrefix}-mode-badge-${mode.id}`"
              @click="updateCurrentMode(mode.id)"
            >
              {{ mode.name }}
            </button>
          </div>
          <div class="agent-config-grid">
            <label v-if="currentReadyProbe.modelOptions.length">
              <span>Model</span>
              <select
                class="field-input"
                :data-testid="`${currentTestPrefix}-model-select`"
                :value="getAgentModelValue(currentAgentConfig, currentReadyProbe)"
                @change="updateCurrentModel"
              >
                <option v-for="model in currentReadyProbe.modelOptions" :key="model.modelId" :value="model.modelId">
                  {{ model.name }}
                </option>
              </select>
            </label>
            <label v-if="currentReadyProbe.thoughtLevelConfigOption">
              <span>{{ currentReadyProbe.thoughtLevelConfigOption.name }}</span>
              <select
                class="field-input"
                :data-testid="`${currentTestPrefix}-thought-level-select`"
                :value="getAgentThoughtLevelValue(currentAgentConfig, currentReadyProbe)"
                @change="updateCurrentThoughtLevel"
              >
                <template
                  v-for="entry in currentReadyProbe.thoughtLevelConfigOption.options"
                  :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"
                >
                  <optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name">
                    <option v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">
                      {{ groupOption.name }}
                    </option>
                  </optgroup>
                  <option v-else :value="entry.value">{{ entry.name }}</option>
                </template>
              </select>
            </label>
            <label v-for="option in currentEditableOptions" :key="option.id">
              <span>{{ option.name }}</span>
              <select
                class="field-input"
                :data-testid="`${currentTestPrefix}-config-${option.id}`"
                :value="getAgentConfigOptionValue(currentAgentConfig, option)"
                @change="updateCurrentConfigOption(option, $event)"
              >
                <template
                  v-for="entry in option.options"
                  :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"
                >
                  <optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name">
                    <option v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">
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
    </template>

    <label v-if="nodeType === 'workflow'" class="field-wide">
      <span>Prompt</span>
      <textarea
        class="field-textarea"
        rows="3"
        placeholder="Optional workflow prompt"
        :value="workflowPrompt"
        @input="emit('update:workflowPrompt', ($event.target as HTMLTextAreaElement).value.trim())"
      />
    </label>

    <label v-else-if="nodeType === 'prompt'" class="field-wide">
      <span>Prompt</span>
      <textarea
        class="field-textarea"
        rows="3"
        placeholder="Required prompt template"
        :value="promptTemplate"
        @input="emit('update:promptTemplate', ($event.target as HTMLTextAreaElement).value.trim())"
      />
    </label>

    <template v-else-if="nodeType === 'sequentialThinking'">
      <label>
        <span>Total thoughts</span>
        <input
          class="field-input"
          :value="thinkingTotalThoughts"
          type="number"
          min="1"
          inputmode="numeric"
          placeholder="5"
          @input="emit('update:thinkingTotalThoughts', ($event.target as HTMLInputElement).value)"
        />
      </label>
      <label>
        <span>Analysis type</span>
        <input
          class="field-input"
          :value="thinkingAnalysisType"
          type="text"
          placeholder="Optional analysis type"
          @input="emit('update:thinkingAnalysisType', ($event.target as HTMLInputElement).value.trim())"
        />
      </label>
    </template>

    <ToolPaletteFields
      v-else
      :active="nodeType === 'tool'"
      :tool-name="toolName"
      :agent-name="toolAgent"
      :agent-config="toolAgentConfig"
      :tool-args="toolArgs"
      :can-submit="canSubmit"
      :cwd="cwd"
      @update:tool-name="emit('update:toolName', $event)"
      @update:agent-name="emit('update:toolAgent', $event)"
      @update:agent-config="emit('update:toolAgentConfig', $event)"
      @update:tool-args="emit('update:toolArgs', $event)"
      @update:cwd="emit('update:cwd', $event)"
      @validation="emit('validation', $event)"
      @submit-request="emit('submitRequest')"
    />
  </div>
</template>
