import { computed, type ComputedRef } from 'vue';
import type { ScheduleForm } from '@/composables/useSchedulesSettings';

export type McpParametersParseResult = { ok: true; value: Record<string, unknown> } | { ok: false };

export function parseAllowedTools(value: string): string[] {
  return value
    .split(/[\n,]/)
    .map((tool) => tool.trim())
    .filter(Boolean);
}

export function parseMcpParameters(rawValue: string): McpParametersParseResult {
  const raw = rawValue.trim();
  if (!raw) return { ok: true, value: {} };
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (typeof parsed === 'object' && !Array.isArray(parsed) && parsed !== null) {
      return { ok: true, value: parsed as Record<string, unknown> };
    }
  } catch {
    return { ok: false };
  }
  return { ok: false };
}

export function useSchedulesValidation(form: ScheduleForm): {
  mcpActionSet: ComputedRef<boolean>;
  mcpParametersValid: ComputedRef<boolean>;
  parseCurrentMcpParameters: () => McpParametersParseResult;
} {
  const parseCurrentMcpParameters = (): McpParametersParseResult => parseMcpParameters(form.mcpParameters);

  return {
    mcpActionSet: computed(() => !!form.mcpAction.trim()),
    mcpParametersValid: computed(() => parseCurrentMcpParameters().ok),
    parseCurrentMcpParameters,
  };
}
