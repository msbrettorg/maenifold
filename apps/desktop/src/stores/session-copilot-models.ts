import type { Ref } from 'vue';
import type { SessionConfigOption, SessionConfigSelectOption, SessionState } from '../lib/types';

export type CopilotReasoningEffortMap = Map<string, string[]>;
export type SessionCopilotReasoningEffortMap = Ref<Map<string, CopilotReasoningEffortMap>>;

interface CopilotModelsResponse {
  models?: Array<{
    id?: string;
    capabilities?: {
      supports?: {
        reasoning_effort?: unknown;
      };
    };
  }>;
}

const REASONING_LABELS: Record<string, string> = {
  none: 'None',
  low: 'Low',
  medium: 'Medium',
  high: 'High',
  xhigh: 'Extra High',
};

export function isCopilotSession(session: SessionState | null | undefined): boolean {
  return session?.agentName.trim().toLowerCase() === 'copilot';
}

export function buildCopilotReasoningEffortMap(
  response: CopilotModelsResponse,
): CopilotReasoningEffortMap {
  const map = new Map<string, string[]>();
  for (const model of response.models ?? []) {
    if (typeof model.id !== 'string') continue;
    const reasoningEffort = model.capabilities?.supports?.reasoning_effort;
    if (!Array.isArray(reasoningEffort)) continue;
    const values = reasoningEffort.filter((value): value is string => typeof value === 'string');
    if (values.length > 0) {
      map.set(model.id, values);
    }
  }
  return map;
}

export function overlayReasoningEffortOptions(
  configOptions: SessionConfigOption[],
  reasoningEfforts: CopilotReasoningEffortMap | null | undefined,
): SessionConfigOption[] {
  if (!reasoningEfforts || reasoningEfforts.size === 0) return configOptions;
  const modelId = configOptions.find((option) => option.id === 'model')?.currentValue;
  const nextValues = modelId ? reasoningEfforts.get(modelId) : undefined;
  if (!nextValues || nextValues.length === 0) return configOptions;
  return configOptions.map((option) => {
    if (option.id !== 'reasoning_effort') return option;
    const currentOptions = Array.isArray(option.options) ? option.options : [];
    const currentMap = new Map<string, SessionConfigSelectOption>();
    for (const candidate of currentOptions) {
      if ('value' in candidate && typeof candidate.value === 'string') {
        currentMap.set(candidate.value, candidate);
      }
    }
    return {
      ...option,
      options: nextValues.map((value) => currentMap.get(value) ?? {
        value,
        name: REASONING_LABELS[value] ?? value,
      }),
    };
  });
}
