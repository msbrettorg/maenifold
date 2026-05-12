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
  if (['swe', 'execute', 'remediate'].includes(normalized)) return 'swe';
  if (['pending', 'todo', 'discovery', 'spec', 'research', 'plan'].includes(normalized)) return 'pending';
  if (['failed', 'error', 'abandoned', 'cancelled', 'stuck'].includes(normalized)) return 'failed';
  // Confession stages map to their parent role
  if (normalized === 'swe-confess') return 'swe';
  if (normalized === 'red-team-confess') return 'red-team';
  if (normalized === 'blue-team-confess') return 'blue-team';
  if (normalized === 'pm-confess') return 'pending';
  if (normalized === 'red-team-audit') return 'red-team';
  // MDD stages
  if (['opus-plan', 'opus-acceptance'].includes(normalized)) return 'pending';
  if (normalized.startsWith('goldeneye-')) return 'red-team';
  if (normalized.startsWith('gpt-5.4-')) return 'swe';

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

interface PipelineBindingTaskFields {
  pipeline?: string | null;
  issue?: string | null;
  state?: string | null;
}

const PIPELINE_LABEL_FALLBACK = '—';

// Extracts the bound pipeline name from a backend issue string. The backend emits
// errors like:
//   "task pipeline binding 'sdd' does not match requested pipeline 'foo'"
// when a task's pipeline definition has drifted (digest mismatch) and clears the
// `pipeline` field on the task row. We recover the original binding from the
// issue text so the UI can keep showing it instead of falling back to "unbound".
function parseBoundPipelineFromIssue(issue: string | null | undefined): string | null {
  if (!issue) return null;
  const match = issue.match(/binding ['"]([^'"]+)['"]/i);
  if (!match) return null;
  const bound = match[1].trim();
  if (!bound || bound.toLowerCase() === 'unbound') return null;
  return bound;
}

export function getTaskPipelineLabel(task: PipelineBindingTaskFields): string {
  const direct = task.pipeline?.trim();
  if (direct) return direct;
  const recovered = parseBoundPipelineFromIssue(task.issue);
  if (recovered) return recovered;
  return PIPELINE_LABEL_FALLBACK;
}

// True when the task's pipeline binding cannot be resolved against the current
// definition on disk (e.g. the pipeline definition's digest changed). The
// backend signals this by clearing the pipeline field while populating an
// issue string and forcing the task into an "error" state.
export function isTaskPipelineDigestStale(task: PipelineBindingTaskFields): boolean {
  const directlyBound = Boolean(task.pipeline?.trim());
  if (directlyBound) return false;
  const hasIssue = Boolean(task.issue?.trim());
  return hasIssue;
}

const STALE_THRESHOLD_MS = 24 * 60 * 60 * 1000;

interface TaskStalenessFields {
  lastUpdatedAt?: string | null;
  state?: string | null;
}

// True when a non-terminal task has had no gate activity in the last 24 hours.
// Tasks in a terminal state (done/completed) are never considered stale.
// If lastUpdatedAt is absent or unparseable we conservatively return false.
export function isTaskStale(task: TaskStalenessFields): boolean {
  if (!task.lastUpdatedAt) return false;
  if (gateStageFamily(task.state ?? '') === 'done') return false;
  const ts = new Date(task.lastUpdatedAt).getTime();
  if (Number.isNaN(ts)) return false;
  return Date.now() - ts > STALE_THRESHOLD_MS;
}
