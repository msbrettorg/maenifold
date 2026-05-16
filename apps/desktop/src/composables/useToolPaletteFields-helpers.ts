// Type defs + pure helper functions for useToolPaletteFields.

import type { Ref } from 'vue';
import { writeLastUsedAgent } from '../lib/preferred-agent';
import { sanitizeResourceErrorMessage, warnRawResourceErrorInDev } from '../lib/error-sanitizer';
import {
  setAgentConfigOptionValue,
  setAgentModelValue,
  setAgentModeValue,
  setAgentThoughtLevelValue,
  type getEditableAgentConfigOptions,
} from './pipeline-node-form-helpers';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types-node';
import type { ToolPaletteEntry } from '../lib/tool-palette-catalog';

export interface ToolPaletteProps {
  active?: boolean;
  toolName: string;
  agentName?: string;
  agentConfig?: PipelineNodeAgentConfig;
  toolArgs: Record<string, unknown>;
  canSubmit?: boolean;
}

export type ToolPaletteEmit = {
  (event: 'update:toolName', value: string): void;
  (event: 'update:agentName', value: string): void;
  (event: 'update:agentConfig', value: PipelineNodeAgentConfig): void;
  (event: 'update:toolArgs', value: Record<string, unknown>): void;
  (event: 'validation', value: { valid: boolean }): void;
  (event: 'submitRequest'): void;
};

export type LiveToolEntry = ToolPaletteEntry & {
  kind: 'tool';
  toolName: string;
  inputSchema: Record<string, unknown>;
};

export function getErrorMessage(error: unknown): string {
  const rawError = error instanceof Error ? error.message : String(error);
  warnRawResourceErrorInDev(rawError);
  return sanitizeResourceErrorMessage(rawError);
}

export function isToolEntry(entry: ToolPaletteEntry): entry is LiveToolEntry {
  return entry.kind === 'tool' && entry.toolName !== null && entry.inputSchema !== null;
}

export function optionId(name: string): string {
  return `tool-palette-option-${name.replace(/[^a-z0-9_-]+/gi, '-')}`;
}

export function highlightText(text: string, query: string) {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) return [{ text, match: false }];
  const normalizedText = text.toLowerCase();
  const parts: Array<{ text: string; match: boolean }> = [];
  let cursor = 0;
  while (cursor < text.length) {
    const matchIndex = normalizedText.indexOf(normalizedQuery, cursor);
    if (matchIndex === -1) return [...parts, { text: text.slice(cursor), match: false }];
    if (matchIndex > cursor) parts.push({ text: text.slice(cursor, matchIndex), match: false });
    parts.push({ text: text.slice(matchIndex, matchIndex + normalizedQuery.length), match: true });
    cursor = matchIndex + normalizedQuery.length;
  }
  return parts.length > 0 ? parts : [{ text, match: false }];
}

export function createAgentUpdaters(props: ToolPaletteProps, emit: ToolPaletteEmit) {
  return {
    updateAgentName(event: Event): void {
      const nextAgent = (event.target as HTMLSelectElement).value;
      emit('update:agentName', nextAgent);
      emit('update:agentConfig', {});
      writeLastUsedAgent(nextAgent);
    },
    updateAgentMode(modeId: string): void {
      emit('update:agentConfig', setAgentModeValue(props.agentConfig, modeId) ?? {});
    },
    updateAgentModel(event: Event): void {
      emit('update:agentConfig', setAgentModelValue(props.agentConfig, (event.target as HTMLSelectElement).value) ?? {});
    },
    updateAgentThoughtLevel(event: Event): void {
      emit('update:agentConfig', setAgentThoughtLevelValue(props.agentConfig, (event.target as HTMLSelectElement).value) ?? {});
    },
    updateAgentConfigOption(option: ReturnType<typeof getEditableAgentConfigOptions>[number], event: Event): void {
      emit('update:agentConfig', setAgentConfigOptionValue(props.agentConfig, option, (event.target as HTMLSelectElement).value) ?? {});
    },
  };
}

export function createPaletteKeydownHandler(opts: {
  searchQuery: { value: string };
  visibleTools: Readonly<Ref<ToolPaletteEntry[]>>;
  activeOptionName: { value: string | null };
  toolName: () => string;
  clearSearch: () => void;
  focusTool: (name: string | null) => void;
  handleToolSelect: (tool: ToolPaletteEntry) => void;
  submitSelectedTool: () => void;
}) {
  return function handlePaletteKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && opts.searchQuery.value) {
      event.preventDefault();
      opts.clearSearch();
      return;
    }
    if (opts.visibleTools.value.length === 0 || !['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)) return;
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const currentIndex = opts.visibleTools.value.findIndex((tool) => tool.namespacedName === opts.activeOptionName.value);
      const delta = event.key === 'ArrowDown' ? 1 : -1;
      const nextIndex = currentIndex === -1
        ? (delta > 0 ? 0 : opts.visibleTools.value.length - 1)
        : (currentIndex + delta + opts.visibleTools.value.length) % opts.visibleTools.value.length;
      opts.focusTool(opts.visibleTools.value[nextIndex]!.namespacedName);
      return;
    }
    event.preventDefault();
    const activeTool = opts.visibleTools.value.find((tool) => tool.namespacedName === opts.activeOptionName.value) ?? opts.visibleTools.value[0]!;
    if (activeTool.namespacedName !== opts.toolName() || activeTool.health === 'auth_required') {
      opts.handleToolSelect(activeTool);
      return;
    }
    opts.submitSelectedTool();
  };
}
