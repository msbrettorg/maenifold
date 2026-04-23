<script setup lang="ts">
import { computed } from 'vue';
import type { AgentProbeConfigOption, AgentProbeResult } from '../lib/agent-probe-types';
import type { ModelInfo, SessionConfigSelectOptions, SessionMode } from '../lib/types';
import { useAgentCapabilitiesStore } from '../stores/agent-capabilities';
import { useAgentSessionOverridesStore } from '../stores/agent-session-overrides';
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
const sessionOverridesStore = useAgentSessionOverridesStore();

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

function getFieldValue(key: string, currentValue?: string) {
  return sessionOverridesStore.getSelectedOverride(props.agentName, key) ?? currentValue;
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
    fields.push({
      ...toPreviewField(probe.modeConfigOption, 'Available after session start'),
      currentValue: getFieldValue(probe.modeConfigOption.id, probe.modeConfigOption.currentValue),
    });
  } else if (probe.modeOptions.length > 0) {
    fields.push({
      key: 'mode',
      label: 'Mode',
      currentValue: getFieldValue('mode'),
      options: toModeOptions(probe.modeOptions),
      emptyLabel: 'Available after session start',
    });
  }

  if (probe.modelConfigOption) {
    consumedIds.add(probe.modelConfigOption.id);
    fields.push({
      ...toPreviewField(probe.modelConfigOption, 'Available after session start'),
      currentValue: getFieldValue(probe.modelConfigOption.id, probe.modelConfigOption.currentValue),
    });
  } else if (probe.modelOptions.length > 0) {
    fields.push({
      key: 'model',
      label: 'Model',
      currentValue: getFieldValue('model'),
      options: toModelOptions(probe.modelOptions),
      emptyLabel: 'Available after session start',
    });
  }

  if (probe.thoughtLevelConfigOption) {
    consumedIds.add(probe.thoughtLevelConfigOption.id);
    fields.push({
      ...toPreviewField(probe.thoughtLevelConfigOption, 'Available after session start'),
      currentValue: getFieldValue(
        probe.thoughtLevelConfigOption.id,
        probe.thoughtLevelConfigOption.currentValue,
      ),
    });
  }

  for (const option of probe.configOptionList) {
    if (!consumedIds.has(option.id)) {
      fields.push({
        ...toPreviewField(option, 'Available after session start'),
        currentValue: getFieldValue(option.id, option.currentValue),
      });
    }
  }

  return fields;
});

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
    return '';
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

function handleUpdateField(key: string, value: string) {
  sessionOverridesStore.setSelectedOverride(props.agentName, key, value);
}
</script>

<template>
  <section
    class="probe-preview grid"
    :class="[compact ? 'gap-[0.75rem]' : 'gap-[0.65rem]', { compact }]"
  >
    <AgentProbePreviewStatus
      :compact="compact"
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
      @update-field="handleUpdateField"
    />
  </section>
</template>
