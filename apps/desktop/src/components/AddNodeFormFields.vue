<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
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

function syntheticEvent(value: string | number | string[]): Event {
  return { target: { value: String(value) } } as unknown as Event;
}
</script>

<template>
  <div class="toolbar-section">
    <template v-if="nodeType === 'workflow'">
      <label>
        <span>Workflow ID</span>
        <Input
          :model-value="workflowId"
          :list="workflowListId"
          type="text"
          placeholder="Workflow ID"
          @update:model-value="emit('update:workflowId', String($event).trim())"
        />
      </label>
      <datalist :id="workflowListId">
        <option v-for="workflow in workflows" :key="workflow" :value="workflow" />
      </datalist>
    </template>

    <label v-else-if="nodeType === 'sequentialThinking'">
      <span>Topic</span>
      <Input
        :model-value="thinkingTopic"
        type="text"
        placeholder="Topic"
        @update:model-value="emit('update:thinkingTopic', String($event).trim())"
      />
    </label>

    <template v-if="hasAgentControls">
      <label>
        <span>Agent</span>
        <Select
          :model-value="currentAgentName"
          @update:model-value="(value) => updateCurrentAgentName(syntheticEvent(value as string))"
        >
          <SelectTrigger
            class="w-full"
            :data-testid="`${currentTestPrefix}-agent-select`"
          >
            <SelectValue placeholder="Select agent…" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="name in configStore.agentNames" :key="name" :value="name">
              {{ formatAgentOptionLabel(name) }}
            </SelectItem>
          </SelectContent>
        </Select>
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
            <Button
              v-for="mode in currentReadyProbe.modeOptions"
              :key="mode.id"
              type="button"
              variant="ghost"
              size="xs"
              class="mode-badge"
              :class="{ selected: getAgentModeValue(currentAgentConfig, currentReadyProbe) === mode.id }"
              :data-testid="`${currentTestPrefix}-mode-badge-${mode.id}`"
              @click="updateCurrentMode(mode.id)"
            >
              {{ mode.name }}
            </Button>
          </div>
          <div class="agent-config-grid">
            <label v-if="currentReadyProbe.modelOptions.length">
              <span>Model</span>
              <Select
                :model-value="getAgentModelValue(currentAgentConfig, currentReadyProbe)"
                @update:model-value="(value) => updateCurrentModel(syntheticEvent(value as string))"
              >
                <SelectTrigger
                  class="w-full"
                  :data-testid="`${currentTestPrefix}-model-select`"
                >
                  <SelectValue placeholder="Select model" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem v-for="model in currentReadyProbe.modelOptions" :key="model.modelId" :value="model.modelId">
                    {{ model.name }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </label>
            <label v-if="currentReadyProbe.thoughtLevelConfigOption">
              <span>{{ currentReadyProbe.thoughtLevelConfigOption.name }}</span>
              <Select
                :model-value="getAgentThoughtLevelValue(currentAgentConfig, currentReadyProbe)"
                @update:model-value="(value) => updateCurrentThoughtLevel(syntheticEvent(value as string))"
              >
                <SelectTrigger
                  class="w-full"
                  :data-testid="`${currentTestPrefix}-thought-level-select`"
                >
                  <SelectValue placeholder="Select" />
                </SelectTrigger>
                <SelectContent>
                  <template
                    v-for="entry in currentReadyProbe.thoughtLevelConfigOption.options"
                    :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"
                  >
                    <SelectGroup v-if="isSessionConfigSelectGroup(entry)">
                      <SelectLabel>{{ entry.name }}</SelectLabel>
                      <SelectItem v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">
                        {{ groupOption.name }}
                      </SelectItem>
                    </SelectGroup>
                    <SelectItem v-else :value="entry.value">{{ entry.name }}</SelectItem>
                  </template>
                </SelectContent>
              </Select>
            </label>
            <label v-for="option in currentEditableOptions" :key="option.id">
              <span>{{ option.name }}</span>
              <Select
                :model-value="getAgentConfigOptionValue(currentAgentConfig, option)"
                @update:model-value="(value) => updateCurrentConfigOption(option, syntheticEvent(value as string))"
              >
                <SelectTrigger
                  class="w-full"
                  :data-testid="`${currentTestPrefix}-config-${option.id}`"
                >
                  <SelectValue placeholder="Select" />
                </SelectTrigger>
                <SelectContent>
                  <template
                    v-for="entry in option.options"
                    :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"
                  >
                    <SelectGroup v-if="isSessionConfigSelectGroup(entry)">
                      <SelectLabel>{{ entry.name }}</SelectLabel>
                      <SelectItem v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">
                        {{ groupOption.name }}
                      </SelectItem>
                    </SelectGroup>
                    <SelectItem v-else :value="entry.value">{{ entry.name }}</SelectItem>
                  </template>
                </SelectContent>
              </Select>
            </label>
          </div>
        </template>
      </div>
    </template>

    <label v-if="nodeType === 'workflow'" class="field-wide">
      <span>Prompt</span>
      <Textarea
        :model-value="workflowPrompt"
        rows="3"
        placeholder="Optional workflow prompt"
        @update:model-value="emit('update:workflowPrompt', String($event).trim())"
      />
    </label>

    <label v-else-if="nodeType === 'prompt'" class="field-wide">
      <span>Prompt</span>
      <Textarea
        :model-value="promptTemplate"
        rows="3"
        placeholder="Required prompt template"
        @update:model-value="emit('update:promptTemplate', String($event).trim())"
      />
    </label>

    <template v-else-if="nodeType === 'sequentialThinking'">
      <label>
        <span>Total thoughts</span>
        <Input
          :model-value="thinkingTotalThoughts"
          type="number"
          min="1"
          inputmode="numeric"
          placeholder="5"
          @update:model-value="emit('update:thinkingTotalThoughts', String($event))"
        />
      </label>
      <label>
        <span>Analysis type</span>
        <Input
          :model-value="thinkingAnalysisType"
          type="text"
          placeholder="Optional analysis type"
          @update:model-value="emit('update:thinkingAnalysisType', String($event).trim())"
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
