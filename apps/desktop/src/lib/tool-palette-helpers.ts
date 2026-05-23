import { Check, KeyRound, AlertTriangle, X } from 'lucide-vue-next';
import type { Component } from 'vue';
import type { ServerHealthStatus } from './types';
import type { ToolPaletteEntry } from './tool-palette-catalog';

const TOOL_SYNONYMS: Record<string, string[]> = {
  account: ['get_account_overview', 'get_account_team', 'get_pipeline_summary', 'get_my_deals'],
  email: ['ask_work_iq'],
  meeting: ['ask_work_iq'],
  calendar: ['ask_work_iq'],
  docs: ['microsoft_docs_search', 'microsoft_docs_fetch', 'microsoft_code_sample_search'],
  documentation: ['microsoft_docs_search', 'microsoft_docs_fetch'],
  memory: ['WriteMemory', 'ReadMemory', 'SearchMemories', 'EditMemory', 'BuildContext'],
  knowledge: ['BuildContext', 'FindSimilarConcepts', 'SearchMemories', 'Visualize'],
  task: ['status', 'pass', 'fail', 'append'],
  pipeline: ['Pipeline', 'Workflow'],
  search: ['SearchMemories', 'microsoft_docs_search', 'search_opportunities'],
  deal: ['get_my_deals', 'search_opportunities', 'get_opportunity_details', 'suggest_top_opportunities'],
};

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

export function normalizeSearchValue(value: string): string {
  return value.trim().toLowerCase().replace(/[._-]+/g, ' ').replace(/\s+/g, ' ');
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

export function scoreField(field: string, query: string): number {
  const normalizedField = normalizeSearchValue(field);
  const normalizedQuery = normalizeSearchValue(query);
  if (!normalizedField || !normalizedQuery) return 0;
  if (normalizedField === normalizedQuery) return 100;
  if (normalizedField.startsWith(normalizedQuery)) return 80;
  if (new RegExp(`(?:^|\\s)${escapeRegExp(normalizedQuery)}(?:\\s|$)`).test(normalizedField)) return 60;
  return normalizedField.includes(normalizedQuery) ? 40 : 0;
}

export function matchesSynonymTarget(field: string, target: string): boolean {
  const normalizedField = normalizeSearchValue(field);
  const normalizedTarget = normalizeSearchValue(target);
  return normalizedField === normalizedTarget || normalizedField.endsWith(` ${normalizedTarget}`);
}

export function getSynonymTargets(query: string): string[] {
  const normalizedQuery = normalizeSearchValue(query);
  if (!normalizedQuery) return [];

  const tokens = normalizedQuery
    .split(' ')
    .map((token) => token.trim())
    .filter((token) => token.length > 1);
  const targets = new Set<string>(TOOL_SYNONYMS[normalizedQuery] ?? []);

  for (const token of tokens) {
    for (const target of TOOL_SYNONYMS[token] ?? []) {
      targets.add(target);
    }
  }

  return [...targets];
}
