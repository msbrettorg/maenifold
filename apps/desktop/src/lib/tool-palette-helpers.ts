import { Check, KeyRound, AlertTriangle, X } from 'lucide-vue-next';
import type { Component } from 'vue';
import type { ServerHealthStatus } from './types';
import type { ToolPaletteEntry } from './tool-palette-catalog';

export const AGENT_DEFAULT_OPTION = '__default__';

export const TOOL_PALETTE_CLASSES = {
  selectField:
    'w-full bg-input text-foreground border border-border rounded-md px-3 py-2 text-caption outline-none focus:border-ring focus:ring-2 focus:ring-ring/20',
  panel: 'grid gap-3 p-3 border border-border-subtle rounded-md bg-bg-elevated/70',
  tagBase:
    'inline-flex items-center px-1.5 py-0.5 border border-border-subtle rounded bg-text-primary/5 text-text-secondary text-xs',
  optionBase:
    'grid gap-1.5 p-3 border border-border-subtle rounded-xl bg-bg-app text-text-primary text-left cursor-pointer hover:bg-text-primary/5 focus-visible:outline-2 focus-visible:outline-brand focus-visible:outline-offset-2 h-auto whitespace-normal',
  flexRow: 'flex flex-wrap gap-1.5 items-center',
  splitRow: 'flex justify-between items-start gap-3',
} as const;

export const HEALTH_META: Record<ServerHealthStatus, { icon: Component; label: string }> = {
  ready: { icon: Check, label: 'Ready' },
  auth_required: { icon: KeyRound, label: 'Setup Required' },
  stale: { icon: AlertTriangle, label: 'Stale' },
  unavailable: { icon: X, label: 'Unavailable' },
};

export type ToolPaletteOption = ToolPaletteEntry;

export interface ToolPaletteToolOptionProps {
  tool: ToolPaletteOption;
  optionId: string;
  optionClasses: string[];
  selected: boolean;
  isPinned: boolean;
  isRecent: boolean;
  healthIcon: Component;
  healthLabel: string;
  healthBadgeToneClass: string;
  healthTagToneClass: string;
  isOption?: boolean;
  nameParts?: { text: string; match: boolean }[];
  descriptionParts?: { text: string; match: boolean }[];
}

export function tagToneClass(health: ServerHealthStatus): string {
  if (health === 'unavailable') return 'text-status-abandoned';
  if (health === 'auth_required') return 'text-status-active border-status-active/40';
  return '';
}

export function healthBadgeToneClass(health: ServerHealthStatus): string {
  if (health === 'ready') return 'text-text-primary';
  if (health === 'auth_required') return 'text-status-active';
  if (health === 'stale') return 'opacity-50';
  if (health === 'unavailable') return 'text-status-abandoned opacity-50';
  return '';
}

export function optionStateClass(
  health: ServerHealthStatus,
  isSelected: boolean,
  isServerState: boolean,
): string[] {
  const classes: string[] = [];
  if (health === 'stale' || health === 'unavailable') classes.push('opacity-60');
  if (isServerState) classes.push('border-dashed');
  if (isSelected) classes.push('border-brand ring-1 ring-brand');
  return classes;
}

export function syntheticChangeEvent(value: unknown): Event {
  const raw = String(value ?? '');
  const normalized = raw === AGENT_DEFAULT_OPTION ? '' : raw;
  return { target: { value: normalized } } as unknown as Event;
}
