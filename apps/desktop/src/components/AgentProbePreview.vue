<script setup lang="ts">
import { computed } from 'vue';
import type { AgentProbeConfigOption, AgentProbeResult } from '../lib/agent-probe-types';
import type {
  ModelInfo,
  SessionConfigSelectGroup,
  SessionConfigSelectOption,
  SessionConfigSelectOptions,
  SessionMode,
} from '../lib/types';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types';

const EMPTY_VALUE = '__preview-empty__';

interface PreviewField {
  key: string;
  label: string;
  description?: string;
  currentValue?: string;
  options: SessionConfigSelectOptions;
  emptyLabel: string;
}

const props = withDefaults(
  defineProps<{
    agentName: string;
    probe?: AgentProbeResult;
    loading?: boolean;
    compact?: boolean;
  }>(),
  {
    loading: false,
    compact: false,
  },
);

function isGroup(
  option: SessionConfigSelectOption | SessionConfigSelectGroup,
): option is SessionConfigSelectGroup {
  return 'group' in option;
}

function flattenOptions(options: SessionConfigSelectOptions): SessionConfigSelectOption[] {
  return options.flatMap((entry) => (isGroup(entry) ? entry.options : [entry]));
}

function toModeOptions(options: SessionMode[]): SessionConfigSelectOption[] {
  return options.map((option) => ({
    value: option.id,
    name: option.name,
    description: option.description,
    _meta: option._meta,
  }));
}

function toModelOptions(options: ModelInfo[]): SessionConfigSelectOption[] {
  return options.map((option) => ({
    value: option.modelId,
    name: option.name,
    description: option.description,
    _meta: option._meta,
  }));
}

function toPreviewField(
  option: AgentProbeConfigOption,
  fallbackLabel: string,
): PreviewField {
  return {
    key: option.id,
    label: option.name,
    description: option.description,
    currentValue: option.currentValue,
    options: option.options,
    emptyLabel: fallbackLabel,
  };
}

const ready = computed(() => props.probe?.state === 'ready');

const previewFields = computed<PreviewField[]>(() => {
  const probe = props.probe;
  if (!probe || probe.state !== 'ready') {
    return [];
  }

  const consumedIds = new Set<string>();
  const fields: PreviewField[] = [];

  if (probe.modeConfigOption) {
    consumedIds.add(probe.modeConfigOption.id);
    fields.push(toPreviewField(probe.modeConfigOption, 'Available after session start'));
  } else if (probe.modeOptions.length > 0) {
    fields.push({
      key: 'mode',
      label: 'Mode',
      options: toModeOptions(probe.modeOptions),
      emptyLabel: 'Available after session start',
    });
  }

  if (probe.modelConfigOption) {
    consumedIds.add(probe.modelConfigOption.id);
    fields.push(toPreviewField(probe.modelConfigOption, 'Available after session start'));
  } else if (probe.modelOptions.length > 0) {
    fields.push({
      key: 'model',
      label: 'Model',
      options: toModelOptions(probe.modelOptions),
      emptyLabel: 'Available after session start',
    });
  }

  if (probe.thoughtLevelConfigOption) {
    consumedIds.add(probe.thoughtLevelConfigOption.id);
    fields.push(toPreviewField(probe.thoughtLevelConfigOption, 'Available after session start'));
  }

  for (const option of probe.configOptionList) {
    if (consumedIds.has(option.id)) {
      continue;
    }

    fields.push(toPreviewField(option, 'Available after session start'));
  }

  return fields;
});

const previewConfig = computed<PipelineNodeAgentConfig>(() => {
  const probe = props.probe;
  if (!probe || probe.state !== 'ready') {
    return {};
  }

  const configOptions = Object.fromEntries(
    probe.configOptionList
      .filter((option) =>
        option.id !== probe.modeConfigOption?.id
        && option.id !== probe.modelConfigOption?.id
        && option.id !== probe.thoughtLevelConfigOption?.id
        && option.currentValue !== undefined,
      )
      .map((option) => [option.id, option.rawCurrentValue ?? option.currentValue]),
  );

  return {
    ...(probe.modeConfigOption?.currentValue ? { mode: probe.modeConfigOption.currentValue } : {}),
    ...(probe.modelConfigOption?.currentValue ? { model: probe.modelConfigOption.currentValue } : {}),
    ...(probe.thoughtLevelConfigOption?.currentValue
      ? { thoughtLevel: probe.thoughtLevelConfigOption.currentValue }
      : {}),
    ...(Object.keys(configOptions).length > 0 ? { configOptions } : {}),
  };
});

const currentDefaults = computed(() =>
  [
    previewConfig.value.mode ? `mode: ${previewConfig.value.mode}` : null,
    previewConfig.value.model ? `model: ${previewConfig.value.model}` : null,
    previewConfig.value.thoughtLevel ? `thoughtLevel: ${previewConfig.value.thoughtLevel}` : null,
  ].filter((entry): entry is string => entry !== null),
);

const statusTone = computed(() => {
  if (props.loading && !props.probe) return 'pending';
  if (!props.probe) return 'idle';
  if (props.probe.state === 'ready') return 'ready';
  return props.probe.state === 'offline' ? 'offline' : 'error';
});

const statusLabel = computed(() => {
  if (props.loading && !props.probe) return 'Probing';
  if (!props.probe) return 'Pending';
  if (props.probe.state === 'ready') return 'Ready';
  return props.probe.state === 'offline' ? 'Offline' : 'Probe failed';
});

const statusMessage = computed(() => {
  if (props.loading && !props.probe) {
    return `Discovering dynamic options for ${props.agentName}.`;
  }

  if (!props.probe) {
    return `Probe results for ${props.agentName} are not available yet.`;
  }

  if (props.probe.state === 'ready') {
    return previewFields.value.length > 0
      ? `${previewFields.value.length} dynamic control${previewFields.value.length === 1 ? '' : 's'} advertised`
      : 'No dynamic session controls advertised';
  }

  return props.probe.error ?? `Agent is ${props.probe.state}.`;
});

function optionCount(field: PreviewField): number {
  return flattenOptions(field.options).length;
}

function currentLabel(field: PreviewField): string {
  if (!field.currentValue) {
    return field.emptyLabel;
  }

  const match = flattenOptions(field.options).find((option) => option.value === field.currentValue);
  return match?.name ?? field.currentValue;
}
</script>

<template>
  <section class="probe-preview" :class="[statusTone, { compact }]">
    <div class="preview-header">
      <span class="status-pill">{{ statusLabel }}</span>
      <span class="status-message">{{ statusMessage }}</span>
    </div>

    <div v-if="ready && currentDefaults.length" class="default-summary">
      {{ currentDefaults.join(' · ') }}
    </div>

    <div v-if="ready && previewFields.length" class="field-grid">
      <div v-for="field in previewFields" :key="field.key" class="field-card">
        <div class="field-heading">
          <span>{{ field.label }}</span>
          <span class="option-count">{{ optionCount(field) }} options</span>
        </div>

        <select class="preview-select" :value="field.currentValue ?? EMPTY_VALUE" disabled>
          <option v-if="!field.currentValue" :value="EMPTY_VALUE">{{ field.emptyLabel }}</option>
          <template v-for="entry in field.options" :key="isGroup(entry) ? entry.group : entry.value">
            <optgroup v-if="isGroup(entry)" :label="entry.name">
              <option
                v-for="groupOption in entry.options"
                :key="groupOption.value"
                :value="groupOption.value"
              >
                {{ groupOption.name }}
              </option>
            </optgroup>
            <option v-else :value="entry.value">
              {{ entry.name }}
            </option>
          </template>
        </select>

        <div class="field-current">{{ currentLabel(field) }}</div>
        <p v-if="field.description" class="field-description">{{ field.description }}</p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.probe-preview {
  display: grid;
  gap: 0.65rem;
  padding: 0.75rem;
  border: 1px solid color-mix(in srgb, var(--border-color) 80%, transparent);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-main) 88%, transparent);
}

.probe-preview.compact {
  padding: 0.65rem;
}

.preview-header {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  align-items: center;
}

.status-pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 5.5rem;
  padding: 0.15rem 0.5rem;
  border-radius: 999px;
  font-size: var(--text-xs);
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.status-message,
.default-summary,
.field-current,
.field-description,
.option-count {
  font-size: var(--text-sm);
}

.status-message,
.field-description,
.option-count {
  color: var(--text-secondary);
}

.default-summary {
  color: var(--text-primary);
  word-break: break-word;
}

.field-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
  gap: 0.65rem;
}

.field-card {
  display: grid;
  gap: 0.35rem;
  min-width: 0;
  padding: 0.65rem;
  border: 1px solid color-mix(in srgb, var(--border-color) 75%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-sidebar) 72%, transparent);
}

.field-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  font-size: var(--text-sm);
  font-weight: 600;
}

.preview-select {
  width: 100%;
  padding: 0.45rem 0.55rem;
  border: 1px solid color-mix(in srgb, var(--border-color) 70%, transparent);
  border-radius: 6px;
  background: color-mix(in srgb, var(--bg-input) 92%, transparent);
  color: var(--text-primary);
  opacity: 0.75;
}

.field-current {
  font-weight: 500;
  color: var(--text-primary);
  word-break: break-word;
}

.probe-preview.ready .status-pill {
  background: color-mix(in srgb, var(--bg-user) 45%, transparent);
  color: var(--text-accent);
}

.probe-preview.pending .status-pill,
.probe-preview.idle .status-pill {
  background: color-mix(in srgb, var(--bg-sidebar) 82%, transparent);
  color: var(--text-secondary);
}

.probe-preview.offline {
  opacity: 0.85;
}

.probe-preview.offline .status-pill {
  background: color-mix(in srgb, var(--bg-warning) 65%, transparent);
  color: var(--text-primary);
}

.probe-preview.error .status-pill {
  background: color-mix(in srgb, var(--bg-danger) 18%, transparent);
  color: var(--bg-danger);
}

@media (max-width: 640px) {
  .field-grid {
    grid-template-columns: 1fr;
  }

  .field-heading {
    align-items: flex-start;
    flex-direction: column;
    gap: 0.25rem;
  }
}
</style>
