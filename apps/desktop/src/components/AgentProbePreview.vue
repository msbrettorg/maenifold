<script setup lang="ts">
import { computed } from 'vue';
import type { AgentProbeConfigOption, AgentProbeResult } from '../lib/agent-probe-types';
import type { ModelInfo, SessionConfigSelectOptions, SessionMode } from '../lib/types';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import AgentProbePreviewCapabilities from './AgentProbePreviewCapabilities.vue';
import AgentProbePreviewStatus from './AgentProbePreviewStatus.vue';

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

const capabilitiesStore = useAgentCapabilitiesStore();

function toModeOptions(options: SessionMode[]) {
  return options.map((option) => ({
    value: option.id,
    name: option.name,
    description: option.description,
    _meta: option._meta,
  }));
}

function toModelOptions(options: ModelInfo[]) {
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
const refreshing = computed(() => props.probe?.refreshing === true);
const degradedMessage = computed(() => props.probe?.degradedMessage ?? null);
const warningMessage = computed(() =>
  degradedMessage.value ? null : capabilitiesStore.warningMessage,
);

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
    if (!consumedIds.has(option.id)) {
      fields.push(toPreviewField(option, 'Available after session start'));
    }
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
    return warningMessage.value
      ? `Live capability discovery is unavailable for ${props.agentName}. Use Refresh to retry loading dynamic options.`
      : `Capability metadata for ${props.agentName} is not available yet.`;
  }

  if (props.probe.state === 'ready') {
    return previewFields.value.length > 0
      ? `${previewFields.value.length} dynamic control${previewFields.value.length === 1 ? '' : 's'} advertised`
      : 'No dynamic session controls advertised';
  }

  if (props.probe.state === 'offline') {
    return `Live capability discovery is unavailable for ${props.agentName}. Cached options stay visible when present, and Refresh retries MCP discovery.`;
  }

  return 'Agent returned an unexpected capability payload. Retry or inspect the agent before relying on dynamic overrides.';
});

const technicalDetail = computed(() => {
  if (!props.probe || props.probe.state === 'ready') {
    return null;
  }
  return props.probe.error ?? null;
});

const refreshDisabled = computed(
  () => props.loading || capabilitiesStore.probeStatus === 'probing',
);

async function handleRefresh() {
  await capabilitiesStore.refreshAll();
}
</script>

<template>
  <section class="probe-preview" :class="{ compact }">
    <AgentProbePreviewStatus
      :compact="compact"
      :current-defaults="currentDefaults"
      :degraded-message="degradedMessage"
      :ready="ready"
      :refresh-disabled="refreshDisabled"
      :refreshing="refreshing"
      :status-label="statusLabel"
      :status-message="statusMessage"
      :status-tone="statusTone"
      :technical-detail="technicalDetail"
      :warning-message="warningMessage"
      @refresh="handleRefresh"
    />
    <AgentProbePreviewCapabilities
      v-if="ready && previewFields.length"
      :compact="compact"
      :fields="previewFields"
    />
  </section>
</template>

<style scoped>
.probe-preview {
  display: grid;
  gap: 0.65rem;
  padding: 0.75rem;
  border: 1px solid color-mix(in srgb, var(--border) 80%, transparent);
  border-radius: 10px;
  background: color-mix(in srgb, var(--background) 88%, transparent);
}

.probe-preview.compact {
  padding: 0.65rem;
}
</style>
