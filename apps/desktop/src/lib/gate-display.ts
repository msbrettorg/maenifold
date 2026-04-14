export type GateStageFamily = 'pending' | 'swe' | 'red-team' | 'blue-team' | 'done' | 'failed' | 'neutral';

const LABEL_OVERRIDES: Record<string, string> = {
  swe: 'SWE',
  'red-team': 'Red-team',
  'blue-team': 'Blue-team',
};

export function formatStageLabel(value: string) {
  const trimmed = value.trim();
  if (!trimmed) return 'Unknown';

  const normalized = trimmed.toLowerCase();
  if (LABEL_OVERRIDES[normalized]) return LABEL_OVERRIDES[normalized];

  const spaced = trimmed.replace(/[-_]+/g, ' ');
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

export function gateStageFamily(value: string): GateStageFamily {
  const normalized = value.trim().toLowerCase();

  if (!normalized) return 'neutral';
  if (['done', 'completed'].includes(normalized)) return 'done';
  if (['blue-team', 'verify'].includes(normalized)) return 'blue-team';
  if (['red-team', 'review'].includes(normalized)) return 'red-team';
  if (['swe', 'execute'].includes(normalized)) return 'swe';
  if (['pending', 'todo', 'discovery', 'spec', 'research', 'plan'].includes(normalized)) return 'pending';
  if (['failed', 'error', 'abandoned', 'cancelled', 'stuck'].includes(normalized)) return 'failed';

  return 'neutral';
}

export function getWorkspaceLabel(root: string) {
  const segments = root.split(/[\\/]+/).filter(Boolean);
  return segments.at(-1) || root;
}

export function getTaskPathHint(path: string, segmentCount = 3) {
  const segments = path.split(/[\\/]+/).filter(Boolean);
  if (!segments.length) return path;
  return segments.slice(-Math.min(segmentCount, segments.length)).join('/');
}
