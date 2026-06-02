<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  AlertCircle,
  CalendarClock,
  Check,
  ChevronRight,
  Pause,
  Pencil,
  Play,
  Plus,
  RefreshCw,
  RotateCcw,
  History,
  Square,
  Trash2,
  X,
} from '@lucide/vue';

/**
 * Opus 4.8 — Schedules tab redesign (MOCKUP).
 *
 * Self-contained: no Pinia, no live store, no MCP, no network. All handlers are
 * local state mutations / console.log. Sample data is hardcoded below.
 *
 * Design intent:
 *  - Explicit Refresh button + a live "freshness" affordance (relative age that
 *    drifts toward stale, with a spinning indicator while refreshing).
 *  - One master table with an inline per-row action cluster
 *    (run-now / pause-resume / cancel / history / edit / delete).
 *  - A right-hand slide-over panel for run history of the selected schedule.
 *  - A centered modal for create/edit, mirroring the SettingsAgents editor idiom.
 *  - A compact inline confirm modal for destructive delete.
 */

type RunStatus = 'success' | 'failed' | 'timeout' | 'skipped';

interface RunRecord {
  status: RunStatus;
  timestamp: string;
  duration_seconds: number;
  error: string | null;
  mcp_action: string | null;
  mcp_parameters: unknown | null;
  mcp_result: unknown | null;
}

interface ScheduledTask {
  id: string;
  name: string;
  schedule: string;
  schedule_human: string | null;
  agent: string;
  prompt: string;
  workdir: string | null;
  auto_grant_permissions: boolean;
  allowed_tools: string[];
  mcp_action: string | null;
  mcp_parameters: unknown | null;
  timeout_seconds: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
  last_run: RunRecord | null;
}

// Derived row state shown in the table. A schedule is "running" when its id is
// the one currently in-flight; otherwise it is enabled / paused / failed.
type RowState = 'running' | 'enabled' | 'paused' | 'failed';

// ---------------------------------------------------------------------------
// Sample data (hardcoded — mockup only).
// ---------------------------------------------------------------------------

const tasks = ref<ScheduledTask[]>([
  {
    id: 'sched_morning_digest',
    name: 'Morning memory digest',
    schedule: '0 7 * * *',
    schedule_human: 'Every day at 7:00 AM',
    agent: 'claude-opus-4.8',
    prompt: 'Summarize new memory:// notes captured in the last 24h and post a digest.',
    workdir: '/Users/brett/src/ma-collective/maenifold-rs',
    auto_grant_permissions: true,
    allowed_tools: ['GatewayExecute', 'ReadMcpResource', 'GetHelp'],
    mcp_action: null,
    mcp_parameters: null,
    timeout_seconds: 600,
    enabled: true,
    created_at: '2025-02-04T11:20:00Z',
    updated_at: '2025-03-01T08:02:00Z',
    last_run: {
      status: 'success',
      timestamp: '2025-03-12T07:00:09Z',
      duration_seconds: 42,
      error: null,
      mcp_action: null,
      mcp_parameters: null,
      mcp_result: null,
    },
  },
  {
    id: 'sched_gate_sync',
    name: 'Gate read-model sync',
    schedule: '*/30 * * * *',
    schedule_human: 'Every 30 minutes',
    agent: 'gpt-5.4',
    prompt: 'Rebuild the gate read-model so the board reflects on-disk task markers.',
    workdir: null,
    auto_grant_permissions: true,
    allowed_tools: ['GatewayExecute'],
    mcp_action: 'gate_Sync',
    mcp_parameters: { root: '/Users/brett/src/ma-collective/maenifold-rs', rebuild: true },
    timeout_seconds: 120,
    enabled: true,
    created_at: '2025-01-18T09:00:00Z',
    updated_at: '2025-03-10T14:31:00Z',
    last_run: {
      status: 'success',
      timestamp: '2025-03-12T09:30:02Z',
      duration_seconds: 6,
      error: null,
      mcp_action: 'gate_Sync',
      mcp_parameters: { root: '/Users/brett/src/ma-collective/maenifold-rs', rebuild: true },
      mcp_result: { tasks_indexed: 47, stages_rebuilt: 5 },
    },
  },
  {
    id: 'sched_nightly_index',
    name: 'Nightly knowledge index',
    schedule: '0 2 * * *',
    schedule_human: 'Every day at 2:00 AM',
    agent: 'mai-code-flash-1',
    prompt: 'Re-embed changed documents and refresh the vector index.',
    workdir: '/Users/brett/src/ma-collective/maenifold-rs',
    auto_grant_permissions: false,
    allowed_tools: ['GatewayIndex', 'GatewayBatch'],
    mcp_action: 'GatewayIndex',
    mcp_parameters: { scope: 'changed', since: '24h' },
    timeout_seconds: 1800,
    enabled: true,
    created_at: '2025-02-22T19:45:00Z',
    updated_at: '2025-03-12T09:41:00Z',
    last_run: {
      status: 'success',
      timestamp: '2025-03-12T02:00:11Z',
      duration_seconds: 311,
      error: null,
      mcp_action: 'GatewayIndex',
      mcp_parameters: { scope: 'changed', since: '24h' },
      mcp_result: { documents: 128, embedded: 41 },
    },
  },
  {
    id: 'sched_acr_report',
    name: 'Weekly ACR reconciliation',
    schedule: '0 6 * * 1',
    schedule_human: 'Mondays at 6:00 AM',
    agent: 'claude-opus-4.8',
    prompt: 'Pull ACR consumption and flag week-over-week anomalies over 15%.',
    workdir: null,
    auto_grant_permissions: false,
    allowed_tools: ['GatewayExecute', 'ReadMcpResource'],
    mcp_action: null,
    mcp_parameters: null,
    timeout_seconds: 900,
    enabled: false,
    created_at: '2025-01-30T12:10:00Z',
    updated_at: '2025-03-04T17:22:00Z',
    last_run: {
      status: 'skipped',
      timestamp: '2025-03-10T06:00:00Z',
      duration_seconds: 0,
      error: 'Schedule paused — run skipped.',
      mcp_action: null,
      mcp_parameters: null,
      mcp_result: null,
    },
  },
  {
    id: 'sched_link_audit',
    name: 'Broken link audit',
    schedule: '15 4 * * *',
    schedule_human: 'Every day at 4:15 AM',
    agent: 'gpt-5.4',
    prompt: 'Crawl docs/ for dead internal links and open a gate task per cluster.',
    workdir: '/Users/brett/src/ma-collective/maenifold-rs/docs',
    auto_grant_permissions: true,
    allowed_tools: ['GatewayExecute', 'GatewayBatch'],
    mcp_action: null,
    mcp_parameters: null,
    timeout_seconds: 600,
    enabled: true,
    created_at: '2025-02-11T08:00:00Z',
    updated_at: '2025-03-11T22:14:00Z',
    last_run: {
      status: 'failed',
      timestamp: '2025-03-12T04:15:33Z',
      duration_seconds: 73,
      error: 'GatewayExecute returned 502 from docs crawler after 3 internal retries.',
      mcp_action: null,
      mcp_parameters: null,
      mcp_result: null,
    },
  },
  {
    id: 'sched_standup_brief',
    name: 'Standup briefing',
    schedule: '45 8 * * 1-5',
    schedule_human: 'Weekdays at 8:45 AM',
    agent: 'mai-code-flash-1',
    prompt: 'Compile overnight run results and open PRs into a short standup brief.',
    workdir: null,
    auto_grant_permissions: true,
    allowed_tools: ['GatewayExecute', 'ReadMcpResource', 'GetHelp'],
    mcp_action: null,
    mcp_parameters: null,
    timeout_seconds: 300,
    enabled: true,
    created_at: '2025-02-28T07:30:00Z',
    updated_at: '2025-03-12T09:39:00Z',
    last_run: {
      status: 'success',
      timestamp: '2025-03-11T08:45:08Z',
      duration_seconds: 58,
      error: null,
      mcp_action: null,
      mcp_parameters: null,
      mcp_result: null,
    },
  },
]);

// Map of schedule id -> recent run history (newest first).
const history = ref<Record<string, RunRecord[]>>({
  sched_morning_digest: [
    { status: 'success', timestamp: '2025-03-12T07:00:09Z', duration_seconds: 42, error: null, mcp_action: null, mcp_parameters: null, mcp_result: null },
    { status: 'success', timestamp: '2025-03-11T07:00:07Z', duration_seconds: 39, error: null, mcp_action: null, mcp_parameters: null, mcp_result: null },
    { status: 'timeout', timestamp: '2025-03-10T07:00:00Z', duration_seconds: 600, error: 'Exceeded timeout_seconds=600 while waiting on memory:// search.', mcp_action: null, mcp_parameters: null, mcp_result: null },
    { status: 'success', timestamp: '2025-03-09T07:00:05Z', duration_seconds: 44, error: null, mcp_action: null, mcp_parameters: null, mcp_result: null },
  ],
  sched_gate_sync: [
    { status: 'success', timestamp: '2025-03-12T09:30:02Z', duration_seconds: 6, error: null, mcp_action: 'gate_Sync', mcp_parameters: { rebuild: true }, mcp_result: { tasks_indexed: 47 } },
    { status: 'success', timestamp: '2025-03-12T09:00:02Z', duration_seconds: 5, error: null, mcp_action: 'gate_Sync', mcp_parameters: { rebuild: true }, mcp_result: { tasks_indexed: 47 } },
    { status: 'skipped', timestamp: '2025-03-12T08:30:00Z', duration_seconds: 0, error: 'Previous run still in flight — overlap skipped.', mcp_action: 'gate_Sync', mcp_parameters: null, mcp_result: null },
  ],
  sched_nightly_index: [
    { status: 'success', timestamp: '2025-03-12T02:00:11Z', duration_seconds: 311, error: null, mcp_action: 'GatewayIndex', mcp_parameters: { scope: 'changed' }, mcp_result: { embedded: 41 } },
    { status: 'failed', timestamp: '2025-03-11T02:00:09Z', duration_seconds: 88, error: 'Vector store rejected batch: dimension mismatch (1536 != 3072).', mcp_action: 'GatewayIndex', mcp_parameters: { scope: 'changed' }, mcp_result: null },
  ],
  sched_acr_report: [
    { status: 'skipped', timestamp: '2025-03-10T06:00:00Z', duration_seconds: 0, error: 'Schedule paused — run skipped.', mcp_action: null, mcp_parameters: null, mcp_result: null },
    { status: 'success', timestamp: '2025-03-03T06:00:14Z', duration_seconds: 204, error: null, mcp_action: null, mcp_parameters: null, mcp_result: null },
  ],
  sched_link_audit: [
    { status: 'failed', timestamp: '2025-03-12T04:15:33Z', duration_seconds: 73, error: 'GatewayExecute returned 502 from docs crawler after 3 internal retries.', mcp_action: null, mcp_parameters: null, mcp_result: null },
    { status: 'success', timestamp: '2025-03-11T04:15:21Z', duration_seconds: 61, error: null, mcp_action: null, mcp_parameters: null, mcp_result: null },
    { status: 'success', timestamp: '2025-03-10T04:15:19Z', duration_seconds: 59, error: null, mcp_action: null, mcp_parameters: null, mcp_result: null },
  ],
  sched_standup_brief: [
    { status: 'success', timestamp: '2025-03-11T08:45:08Z', duration_seconds: 58, error: null, mcp_action: null, mcp_parameters: null, mcp_result: null },
    { status: 'success', timestamp: '2025-03-10T08:45:11Z', duration_seconds: 63, error: null, mcp_action: null, mcp_parameters: null, mcp_result: null },
  ],
});

// ---------------------------------------------------------------------------
// Freshness / refresh affordance.
// ---------------------------------------------------------------------------

const lastRefreshed = ref<Date>(new Date());
const refreshing = ref(false);
// Drives the relative-age recompute without a poll: a tick the user can bump.
const ageTick = ref(0);

const freshnessLabel = computed<string>(() => {
  void ageTick.value; // dependency so manual refreshes re-render the label
  const seconds = Math.max(0, Math.round((Date.now() - lastRefreshed.value.getTime()) / 1000));
  if (seconds < 5) return 'just now';
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  return `${hours}h ago`;
});

const isStale = computed<boolean>(() => {
  void ageTick.value;
  return Date.now() - lastRefreshed.value.getTime() > 120_000;
});

function refresh(): void {
  if (refreshing.value) return;
  refreshing.value = true;
  console.log('[mock] refresh schedules');
  // Simulate a fetch latency, then stamp freshness. No polling is started.
  window.setTimeout(() => {
    lastRefreshed.value = new Date();
    ageTick.value += 1;
    refreshing.value = false;
  }, 650);
}

// ---------------------------------------------------------------------------
// Row state + run controls.
// ---------------------------------------------------------------------------

// Which schedule (if any) currently has an in-flight run. Seeded so the mockup
// shows a "running" row out of the box.
const runningId = ref<string | null>('sched_nightly_index');

function rowState(task: ScheduledTask): RowState {
  if (runningId.value === task.id) return 'running';
  if (!task.enabled) return 'paused';
  if (task.last_run && task.last_run.status === 'failed') return 'failed';
  return 'enabled';
}

const stateLabels: Record<RowState, string> = {
  running: 'Running',
  enabled: 'Enabled',
  paused: 'Paused',
  failed: 'Last run failed',
};

function runNow(task: ScheduledTask): void {
  console.log('[mock] run now', task.id);
  runningId.value = task.id;
}

function cancelRun(task: ScheduledTask): void {
  console.log('[mock] cancel in-flight run', task.id);
  if (runningId.value === task.id) {
    runningId.value = null;
  }
}

function togglePause(task: ScheduledTask): void {
  console.log('[mock] toggle pause', task.id, '->', !task.enabled);
  task.enabled = !task.enabled;
  task.updated_at = new Date().toISOString();
}

// ---------------------------------------------------------------------------
// History slide-over.
// ---------------------------------------------------------------------------

const historyOpenId = ref<string | null>(null);

const historyTask = computed<ScheduledTask | null>(
  () => tasks.value.find((t) => t.id === historyOpenId.value) ?? null,
);

const historyRuns = computed<RunRecord[]>(() =>
  historyOpenId.value ? history.value[historyOpenId.value] ?? [] : [],
);

function openHistory(task: ScheduledTask): void {
  console.log('[mock] open history', task.id);
  historyOpenId.value = task.id;
}

function closeHistory(): void {
  historyOpenId.value = null;
}

// ---------------------------------------------------------------------------
// Create / edit modal.
// ---------------------------------------------------------------------------

interface EditorForm {
  id: string | null;
  name: string;
  schedule: string;
  schedule_human: string;
  agent: string;
  prompt: string;
  workdir: string;
  auto_grant_permissions: boolean;
  allowedToolsText: string;
  mcp_action: string;
  timeout_seconds: number;
  enabled: boolean;
}

const editorOpen = ref(false);
const editorForm = ref<EditorForm>(emptyForm());

function emptyForm(): EditorForm {
  return {
    id: null,
    name: '',
    schedule: '',
    schedule_human: '',
    agent: 'claude-opus-4.8',
    prompt: '',
    workdir: '',
    auto_grant_permissions: false,
    allowedToolsText: '',
    mcp_action: '',
    timeout_seconds: 600,
    enabled: true,
  };
}

const editorIsEditing = computed<boolean>(() => editorForm.value.id !== null);

function openCreate(): void {
  console.log('[mock] open create');
  editorForm.value = emptyForm();
  editorOpen.value = true;
}

function openEdit(task: ScheduledTask): void {
  console.log('[mock] open edit', task.id);
  editorForm.value = {
    id: task.id,
    name: task.name,
    schedule: task.schedule,
    schedule_human: task.schedule_human ?? '',
    agent: task.agent,
    prompt: task.prompt,
    workdir: task.workdir ?? '',
    auto_grant_permissions: task.auto_grant_permissions,
    allowedToolsText: task.allowed_tools.join(', '),
    mcp_action: task.mcp_action ?? '',
    timeout_seconds: task.timeout_seconds,
    enabled: task.enabled,
  };
  editorOpen.value = true;
}

function closeEditor(): void {
  editorOpen.value = false;
}

function parseTools(text: string): string[] {
  return text
    .split(',')
    .map((t) => t.trim())
    .filter((t) => t.length > 0);
}

function saveEditor(): void {
  const f = editorForm.value;
  const now = new Date().toISOString();
  const tools = parseTools(f.allowedToolsText);
  const mcpAction = f.mcp_action.trim().length > 0 ? f.mcp_action.trim() : null;

  if (f.id === null) {
    const created: ScheduledTask = {
      id: `sched_${Math.random().toString(36).slice(2, 8)}`,
      name: f.name.trim() || 'Untitled schedule',
      schedule: f.schedule.trim() || '0 0 * * *',
      schedule_human: f.schedule_human.trim() || null,
      agent: f.agent.trim() || 'claude-opus-4.8',
      prompt: f.prompt.trim(),
      workdir: f.workdir.trim() || null,
      auto_grant_permissions: f.auto_grant_permissions,
      allowed_tools: tools,
      mcp_action: mcpAction,
      mcp_parameters: null,
      timeout_seconds: Number(f.timeout_seconds) || 600,
      enabled: f.enabled,
      created_at: now,
      updated_at: now,
      last_run: null,
    };
    console.log('[mock] create schedule', created);
    tasks.value = [created, ...tasks.value];
    history.value[created.id] = [];
  } else {
    const target = tasks.value.find((t) => t.id === f.id);
    if (target) {
      target.name = f.name.trim() || target.name;
      target.schedule = f.schedule.trim() || target.schedule;
      target.schedule_human = f.schedule_human.trim() || null;
      target.agent = f.agent.trim() || target.agent;
      target.prompt = f.prompt.trim();
      target.workdir = f.workdir.trim() || null;
      target.auto_grant_permissions = f.auto_grant_permissions;
      target.allowed_tools = tools;
      target.mcp_action = mcpAction;
      target.timeout_seconds = Number(f.timeout_seconds) || target.timeout_seconds;
      target.enabled = f.enabled;
      target.updated_at = now;
      console.log('[mock] update schedule', target.id);
    }
  }
  editorOpen.value = false;
}

// ---------------------------------------------------------------------------
// Delete confirmation.
// ---------------------------------------------------------------------------

const deleteTarget = ref<ScheduledTask | null>(null);

function askDelete(task: ScheduledTask): void {
  deleteTarget.value = task;
}

function cancelDelete(): void {
  deleteTarget.value = null;
}

function confirmDelete(): void {
  const target = deleteTarget.value;
  if (!target) return;
  console.log('[mock] delete schedule', target.id);
  tasks.value = tasks.value.filter((t) => t.id !== target.id);
  delete history.value[target.id];
  if (historyOpenId.value === target.id) historyOpenId.value = null;
  if (runningId.value === target.id) runningId.value = null;
  deleteTarget.value = null;
}

// ---------------------------------------------------------------------------
// Formatting helpers.
// ---------------------------------------------------------------------------

const enabledCount = computed<number>(() => tasks.value.filter((t) => t.enabled).length);

function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function formatDuration(seconds: number): string {
  if (seconds <= 0) return '—';
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const rest = seconds % 60;
  return rest === 0 ? `${minutes}m` : `${minutes}m ${rest}s`;
}

const statusLabels: Record<RunStatus, string> = {
  success: 'Success',
  failed: 'Failed',
  timeout: 'Timeout',
  skipped: 'Skipped',
};
</script>

<template>
  <div class="schedules">
    <div class="schedules-inner">
      <!-- Hero -->
      <section class="schedules-hero">
        <div class="schedules-icon-wrap">
          <CalendarClock :size="34" :stroke-width="1.5" />
        </div>
        <div class="schedules-copy">
          <h1 class="schedules-title">Schedules</h1>
          <p class="schedules-description">
            Scheduled agent runs and MCP actions. Data is fetched on demand — use Refresh to pull
            the latest state from the daemon.
          </p>
        </div>
      </section>

      <!-- Toolbar: freshness + refresh + create -->
      <section class="toolbar">
        <div class="freshness" :class="{ stale: isStale }">
          <span class="freshness-dot" aria-hidden="true" />
          <span class="freshness-text">
            Updated <strong>{{ freshnessLabel }}</strong>
          </span>
          <span v-if="isStale" class="freshness-flag">stale</span>
        </div>

        <div class="toolbar-actions">
          <button
            class="btn"
            type="button"
            :disabled="refreshing"
            @click="refresh"
          >
            <RefreshCw :size="15" :class="{ spinning: refreshing }" />
            <span>{{ refreshing ? 'Refreshing…' : 'Refresh' }}</span>
          </button>
          <button class="btn btn-primary" type="button" @click="openCreate">
            <Plus :size="15" />
            <span>New schedule</span>
          </button>
        </div>
      </section>

      <!-- Schedules table -->
      <section class="schedules-card">
        <div class="schedules-card-header">
          <div>
            <h2 class="schedules-card-title">Scheduled jobs</h2>
            <p class="schedules-card-subtitle">
              {{ enabledCount }} of {{ tasks.length }} enabled
            </p>
          </div>
          <span class="schedules-count">{{ tasks.length }}</span>
        </div>

        <div v-if="tasks.length === 0" class="schedules-state">
          No schedules yet. Create one to get started.
        </div>

        <div v-else class="schedules-table-wrap">
          <table class="schedules-table">
            <thead>
              <tr>
                <th scope="col">Schedule</th>
                <th scope="col">Cadence</th>
                <th scope="col">Agent</th>
                <th scope="col">Last run</th>
                <th scope="col" class="col-actions">Controls</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="task in tasks"
                :key="task.id"
                :class="{ 'row-running': rowState(task) === 'running' }"
              >
                <!-- Name + state -->
                <td>
                  <div class="job-head">
                    <span class="state-dot" :class="`state-${rowState(task)}`" aria-hidden="true" />
                    <div>
                      <div class="job-primary">{{ task.name }}</div>
                      <div class="job-secondary">
                        <span class="state-label" :class="`state-text-${rowState(task)}`">
                          {{ stateLabels[rowState(task)] }}
                        </span>
                        <span v-if="task.mcp_action" class="mcp-tag">
                          {{ task.mcp_action }}
                        </span>
                      </div>
                    </div>
                  </div>
                </td>

                <!-- Cadence -->
                <td>
                  <div class="cadence-human">{{ task.schedule_human ?? '—' }}</div>
                  <code class="cadence-cron">{{ task.schedule }}</code>
                </td>

                <!-- Agent -->
                <td>
                  <span class="agent-chip">{{ task.agent }}</span>
                </td>

                <!-- Last run -->
                <td>
                  <template v-if="task.last_run">
                    <span class="run-pill" :class="`run-${task.last_run.status}`">
                      {{ statusLabels[task.last_run.status] }}
                    </span>
                    <div class="run-meta">
                      {{ formatTimestamp(task.last_run.timestamp) }}
                      · {{ formatDuration(task.last_run.duration_seconds) }}
                    </div>
                  </template>
                  <span v-else class="run-meta">Never run</span>
                </td>

                <!-- Controls -->
                <td class="col-actions">
                  <div class="row-actions">
                    <!-- run / cancel -->
                    <button
                      v-if="rowState(task) === 'running'"
                      class="icon-btn icon-btn-danger"
                      type="button"
                      title="Cancel in-flight run"
                      aria-label="Cancel in-flight run"
                      @click="cancelRun(task)"
                    >
                      <Square :size="15" />
                    </button>
                    <button
                      v-else
                      class="icon-btn"
                      type="button"
                      title="Run now"
                      aria-label="Run now"
                      @click="runNow(task)"
                    >
                      <Play :size="15" />
                    </button>

                    <!-- pause / resume -->
                    <button
                      class="icon-btn"
                      type="button"
                      :title="task.enabled ? 'Pause schedule' : 'Resume schedule'"
                      :aria-label="task.enabled ? 'Pause schedule' : 'Resume schedule'"
                      @click="togglePause(task)"
                    >
                      <Pause v-if="task.enabled" :size="15" />
                      <RotateCcw v-else :size="15" />
                    </button>

                    <!-- history -->
                    <button
                      class="icon-btn"
                      type="button"
                      title="View run history"
                      aria-label="View run history"
                      @click="openHistory(task)"
                    >
                      <History :size="15" />
                    </button>

                    <!-- edit -->
                    <button
                      class="icon-btn"
                      type="button"
                      title="Edit schedule"
                      aria-label="Edit schedule"
                      @click="openEdit(task)"
                    >
                      <Pencil :size="15" />
                    </button>

                    <!-- delete -->
                    <button
                      class="icon-btn icon-btn-danger"
                      type="button"
                      title="Delete schedule"
                      aria-label="Delete schedule"
                      @click="askDelete(task)"
                    >
                      <Trash2 :size="15" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </div>

    <!-- History slide-over -->
    <div
      v-if="historyTask"
      class="overlay overlay-right"
      @click.self="closeHistory"
    >
      <aside class="history-panel" role="dialog" aria-modal="true" aria-label="Run history">
        <header class="history-head">
          <div>
            <div class="history-eyebrow">
              <History :size="14" />
              <span>Run history</span>
            </div>
            <h3>{{ historyTask.name }}</h3>
            <code class="cadence-cron">{{ historyTask.schedule }}</code>
          </div>
          <button class="icon-btn" type="button" aria-label="Close history" @click="closeHistory">
            <X :size="16" />
          </button>
        </header>

        <div class="history-body">
          <p v-if="historyRuns.length === 0" class="history-empty">
            No runs recorded yet.
          </p>
          <ol v-else class="run-list">
            <li v-for="(run, index) in historyRuns" :key="index" class="run-row">
              <span class="run-pill" :class="`run-${run.status}`">
                {{ statusLabels[run.status] }}
              </span>
              <div class="run-detail">
                <div class="run-when">
                  {{ formatTimestamp(run.timestamp) }}
                  <span class="run-dur">· {{ formatDuration(run.duration_seconds) }}</span>
                </div>
                <div v-if="run.mcp_action" class="run-action">
                  <ChevronRight :size="13" />
                  <code>{{ run.mcp_action }}</code>
                </div>
                <p v-if="run.error" class="run-error">
                  <AlertCircle :size="13" />
                  <span>{{ run.error }}</span>
                </p>
              </div>
            </li>
          </ol>
        </div>
      </aside>
    </div>

    <!-- Create / edit modal -->
    <div v-if="editorOpen" class="overlay" @click.self="closeEditor">
      <section class="editor-dialog" role="dialog" aria-modal="true" aria-label="Schedule editor">
        <header class="editor-head">
          <div>
            <h3>{{ editorIsEditing ? 'Edit schedule' : 'New schedule' }}</h3>
            <p>
              {{ editorIsEditing
                ? 'Update cadence, agent, and run options for this schedule.'
                : 'Define a cadence and the agent prompt or MCP action to run.' }}
            </p>
          </div>
          <button class="icon-btn" type="button" aria-label="Close editor" @click="closeEditor">
            <X :size="16" />
          </button>
        </header>

        <div class="editor-body">
          <label class="field">
            <span class="field-label">Name</span>
            <input v-model="editorForm.name" class="input" type="text" placeholder="Nightly knowledge index" />
          </label>

          <div class="field-grid">
            <label class="field">
              <span class="field-label">Cron expression</span>
              <input v-model="editorForm.schedule" class="input mono" type="text" placeholder="0 2 * * *" />
            </label>
            <label class="field">
              <span class="field-label">Human cadence <em>(optional)</em></span>
              <input v-model="editorForm.schedule_human" class="input" type="text" placeholder="Every day at 2:00 AM" />
            </label>
          </div>

          <div class="field-grid">
            <label class="field">
              <span class="field-label">Agent</span>
              <input v-model="editorForm.agent" class="input" type="text" placeholder="claude-opus-4.8" />
            </label>
            <label class="field">
              <span class="field-label">Timeout (seconds)</span>
              <input v-model.number="editorForm.timeout_seconds" class="input mono" type="number" min="1" />
            </label>
          </div>

          <label class="field">
            <span class="field-label">Prompt</span>
            <textarea v-model="editorForm.prompt" class="input textarea" rows="3" placeholder="Re-embed changed documents and refresh the vector index." />
          </label>

          <div class="field-grid">
            <label class="field">
              <span class="field-label">Workdir <em>(optional)</em></span>
              <input v-model="editorForm.workdir" class="input mono" type="text" placeholder="/path/to/repo" />
            </label>
            <label class="field">
              <span class="field-label">MCP action <em>(optional)</em></span>
              <input v-model="editorForm.mcp_action" class="input mono" type="text" placeholder="gate_Sync" />
            </label>
          </div>

          <label class="field">
            <span class="field-label">Allowed tools <em>(comma separated)</em></span>
            <input v-model="editorForm.allowedToolsText" class="input mono" type="text" placeholder="GatewayExecute, ReadMcpResource" />
          </label>

          <div class="toggle-row">
            <label class="toggle">
              <input v-model="editorForm.enabled" type="checkbox" />
              <span>Enabled</span>
            </label>
            <label class="toggle">
              <input v-model="editorForm.auto_grant_permissions" type="checkbox" />
              <span>Auto-grant permissions</span>
            </label>
          </div>
        </div>

        <footer class="editor-foot">
          <button class="btn" type="button" @click="closeEditor">Cancel</button>
          <button class="btn btn-primary" type="button" @click="saveEditor">
            <Check :size="15" />
            <span>{{ editorIsEditing ? 'Save changes' : 'Create schedule' }}</span>
          </button>
        </footer>
      </section>
    </div>

    <!-- Delete confirmation -->
    <div v-if="deleteTarget" class="overlay" @click.self="cancelDelete">
      <section class="confirm-dialog" role="alertdialog" aria-modal="true" aria-label="Delete schedule">
        <div class="confirm-icon">
          <AlertCircle :size="22" />
        </div>
        <h3 class="confirm-title">Delete this schedule?</h3>
        <p class="confirm-copy">
          <strong>{{ deleteTarget.name }}</strong> and its run history will be removed. This
          cannot be undone.
        </p>
        <div class="confirm-actions">
          <button class="btn" type="button" @click="cancelDelete">Cancel</button>
          <button class="btn btn-danger" type="button" @click="confirmDelete">
            <Trash2 :size="15" />
            <span>Delete</span>
          </button>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.schedules {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 2rem;
  background:
    radial-gradient(70% 50% at 50% 0%, color-mix(in srgb, var(--text-accent) 9%, transparent) 0%, transparent 70%),
    var(--bg-main);
}

.schedules-inner {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: 100%;
  max-width: 980px;
  margin: 0 auto;
}

/* Hero ------------------------------------------------------------------ */
.schedules-hero {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1.25rem 1.35rem;
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: color-mix(in srgb, var(--bg-sidebar) 70%, var(--bg-main));
}

.schedules-icon-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 64px;
  height: 64px;
  border-radius: 18px;
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--text-accent) 25%, var(--border-color));
  box-shadow: 0 10px 30px color-mix(in srgb, var(--text-accent) 16%, transparent);
}

.schedules-copy {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.schedules-title {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 600;
  letter-spacing: -0.02em;
  color: var(--text-primary);
}

.schedules-description {
  margin: 0;
  color: var(--text-secondary);
  line-height: 1.5;
}

/* Toolbar --------------------------------------------------------------- */
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
}

.freshness {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.4rem 0.75rem;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 60%, var(--bg-main));
  color: var(--text-secondary);
  font-size: 0.82rem;
}

.freshness-dot {
  width: 0.5rem;
  height: 0.5rem;
  border-radius: 999px;
  background: var(--bg-success);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--bg-success) 22%, transparent);
}

.freshness.stale .freshness-dot {
  background: var(--text-muted);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--text-muted) 22%, transparent);
}

.freshness-text strong {
  color: var(--text-primary);
  font-weight: 600;
}

.freshness-flag {
  padding: 0.05rem 0.45rem;
  border-radius: 999px;
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: color-mix(in srgb, var(--bg-warning) 30%, var(--text-primary));
  background: color-mix(in srgb, var(--bg-warning) 35%, var(--bg-main));
  border: 1px solid color-mix(in srgb, var(--bg-warning) 50%, var(--border-color));
}

.toolbar-actions {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
}

/* Buttons --------------------------------------------------------------- */
.btn,
.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: var(--bg-sidebar);
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.82rem;
  cursor: pointer;
  transition: background 0.12s ease, border-color 0.12s ease, color 0.12s ease;
}

.btn {
  padding: 0.5rem 0.8rem;
}

.icon-btn {
  width: 2.1rem;
  height: 2.1rem;
  padding: 0;
  border-radius: 9px;
}

.btn:hover,
.icon-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.btn:disabled,
.icon-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.btn-primary {
  color: var(--bg-main);
  background: var(--text-accent);
  border-color: var(--text-accent);
}

.btn-primary:hover {
  filter: brightness(1.08);
  color: var(--bg-main);
}

.btn-danger,
.icon-btn-danger {
  color: color-mix(in srgb, var(--bg-danger) 82%, var(--text-primary));
  border-color: color-mix(in srgb, var(--bg-danger) 52%, var(--border-color));
}

.btn-danger:hover,
.icon-btn-danger:hover {
  background: color-mix(in srgb, var(--bg-danger) 20%, var(--bg-main));
  color: color-mix(in srgb, var(--bg-danger) 68%, var(--text-primary));
}

.spinning {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* Card + table ---------------------------------------------------------- */
.schedules-card {
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: var(--bg-main);
  overflow: hidden;
}

.schedules-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 1rem 1.15rem;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 55%, var(--bg-main));
}

.schedules-card-title {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text-primary);
}

.schedules-card-subtitle {
  margin: 0.2rem 0 0;
  font-size: 0.78rem;
  color: var(--text-muted);
}

.schedules-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 2rem;
  height: 2rem;
  padding: 0 0.6rem;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--bg-sidebar);
  color: var(--text-primary);
  font-size: 0.85rem;
  font-weight: 600;
}

.schedules-state {
  padding: 2.5rem 1rem;
  text-align: center;
  color: var(--text-secondary);
}

.schedules-table-wrap {
  overflow-x: auto;
}

.schedules-table {
  width: 100%;
  border-collapse: collapse;
}

.schedules-table th,
.schedules-table td {
  padding: 0.85rem 1.1rem;
  text-align: left;
  border-bottom: 1px solid var(--border-color);
  vertical-align: top;
}

.schedules-table th {
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--text-muted);
  background: color-mix(in srgb, var(--bg-sidebar) 35%, var(--bg-main));
}

.schedules-table td {
  font-size: 0.9rem;
  color: var(--text-primary);
}

.schedules-table tbody tr:hover {
  background: var(--bg-hover);
}

.schedules-table tbody tr:last-child td {
  border-bottom: none;
}

.row-running {
  background: color-mix(in srgb, var(--text-accent) 6%, transparent);
}

.col-actions {
  width: 1%;
  white-space: nowrap;
}

/* Name cell ------------------------------------------------------------- */
.job-head {
  display: flex;
  align-items: flex-start;
  gap: 0.6rem;
}

.state-dot {
  flex-shrink: 0;
  width: 0.6rem;
  height: 0.6rem;
  margin-top: 0.3rem;
  border-radius: 999px;
  background: var(--text-muted);
}

.state-enabled {
  background: var(--bg-success);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--bg-success) 20%, transparent);
}

.state-paused {
  background: var(--text-muted);
}

.state-failed {
  background: var(--bg-danger);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--bg-danger) 20%, transparent);
}

.state-running {
  background: var(--text-accent);
  box-shadow: 0 0 0 0 color-mix(in srgb, var(--text-accent) 45%, transparent);
  animation: pulse 1.4s ease-out infinite;
}

@keyframes pulse {
  0% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--text-accent) 45%, transparent);
  }
  100% {
    box-shadow: 0 0 0 7px color-mix(in srgb, var(--text-accent) 0%, transparent);
  }
}

.job-primary {
  font-weight: 600;
  color: var(--text-primary);
}

.job-secondary {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  margin-top: 0.25rem;
  flex-wrap: wrap;
}

.state-label {
  font-size: 0.74rem;
  color: var(--text-muted);
}

.state-text-running {
  color: var(--text-accent);
  font-weight: 600;
}

.state-text-failed {
  color: color-mix(in srgb, var(--bg-danger) 75%, var(--text-primary));
  font-weight: 600;
}

.mcp-tag {
  padding: 0.05rem 0.4rem;
  border-radius: 999px;
  font-size: 0.68rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--text-accent) 28%, var(--border-color));
}

/* Cadence cell ---------------------------------------------------------- */
.cadence-human {
  font-size: 0.86rem;
  color: var(--text-primary);
}

.cadence-cron {
  display: inline-block;
  margin-top: 0.2rem;
  padding: 0.05rem 0.35rem;
  border-radius: 6px;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.74rem;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--bg-sidebar) 60%, var(--bg-main));
  border: 1px solid var(--border-color);
}

/* Agent cell ------------------------------------------------------------ */
.agent-chip {
  display: inline-flex;
  align-items: center;
  padding: 0.2rem 0.55rem;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--bg-sidebar);
  color: var(--text-secondary);
  font-size: 0.76rem;
}

/* Run pill / meta ------------------------------------------------------- */
.run-pill {
  display: inline-flex;
  align-items: center;
  padding: 0.18rem 0.5rem;
  border-radius: 999px;
  font-size: 0.72rem;
  font-weight: 600;
  border: 1px solid var(--border-color);
  background: var(--bg-sidebar);
  color: var(--text-secondary);
}

.run-success {
  color: color-mix(in srgb, var(--bg-success) 72%, var(--text-primary));
  background: color-mix(in srgb, var(--bg-success) 12%, var(--bg-main));
  border-color: color-mix(in srgb, var(--bg-success) 45%, var(--border-color));
}

.run-failed {
  color: color-mix(in srgb, var(--bg-danger) 76%, var(--text-primary));
  background: color-mix(in srgb, var(--bg-danger) 12%, var(--bg-main));
  border-color: color-mix(in srgb, var(--bg-danger) 48%, var(--border-color));
}

.run-timeout {
  color: color-mix(in srgb, var(--bg-warning) 28%, var(--text-primary));
  background: color-mix(in srgb, var(--bg-warning) 35%, var(--bg-main));
  border-color: color-mix(in srgb, var(--bg-warning) 55%, var(--border-color));
}

.run-skipped {
  color: var(--text-muted);
  background: color-mix(in srgb, var(--bg-sidebar) 70%, var(--bg-main));
}

.run-meta {
  margin-top: 0.25rem;
  font-size: 0.74rem;
  color: var(--text-muted);
}

/* Row actions ----------------------------------------------------------- */
.row-actions {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
}

/* Overlays -------------------------------------------------------------- */
.overlay {
  position: fixed;
  inset: 0;
  z-index: 1100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.25rem;
  background: rgba(0, 0, 0, 0.55);
}

.overlay-right {
  justify-content: flex-end;
  padding: 0;
}

/* History panel --------------------------------------------------------- */
.history-panel {
  width: min(440px, 100%);
  height: 100%;
  display: flex;
  flex-direction: column;
  border-left: 1px solid var(--border-color);
  background: var(--bg-main);
  box-shadow: -24px 0 60px rgba(0, 0, 0, 0.3);
  animation: slide-in 0.18s ease;
}

@keyframes slide-in {
  from {
    transform: translateX(2rem);
    opacity: 0.6;
  }
}

.history-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  padding: 1.1rem 1.2rem;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 55%, var(--bg-main));
}

.history-eyebrow {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  color: var(--text-accent);
  font-size: 0.72rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.history-head h3 {
  margin: 0.35rem 0 0.3rem;
  font-size: 1.05rem;
  font-weight: 600;
  color: var(--text-primary);
}

.history-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 1rem 1.2rem 1.5rem;
}

.history-empty {
  margin: 2rem 0;
  text-align: center;
  color: var(--text-muted);
}

.run-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
}

.run-row {
  display: flex;
  align-items: flex-start;
  gap: 0.7rem;
  padding: 0.8rem 0.85rem;
  border-radius: 12px;
  border: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 40%, var(--bg-main));
}

.run-detail {
  flex: 1;
  min-width: 0;
}

.run-when {
  font-size: 0.84rem;
  color: var(--text-primary);
}

.run-dur {
  color: var(--text-muted);
}

.run-action {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  margin-top: 0.3rem;
  color: var(--text-secondary);
}

.run-action code {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.74rem;
}

.run-error {
  display: flex;
  align-items: flex-start;
  gap: 0.35rem;
  margin: 0.45rem 0 0;
  padding: 0.45rem 0.55rem;
  border-radius: 8px;
  font-size: 0.76rem;
  line-height: 1.45;
  color: color-mix(in srgb, var(--bg-danger) 78%, var(--text-primary));
  background: color-mix(in srgb, var(--bg-danger) 10%, var(--bg-main));
  border: 1px solid color-mix(in srgb, var(--bg-danger) 35%, var(--border-color));
}

.run-error svg {
  flex-shrink: 0;
  margin-top: 0.12rem;
}

/* Editor dialog --------------------------------------------------------- */
.editor-dialog {
  width: min(680px, 100%);
  max-height: calc(100vh - 2.5rem);
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-color);
  border-radius: 18px;
  background: var(--bg-main);
  overflow: hidden;
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.35);
}

.editor-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  padding: 1.15rem 1.25rem;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 55%, var(--bg-main));
}

.editor-head h3 {
  margin: 0;
  font-size: 1.08rem;
  font-weight: 600;
  color: var(--text-primary);
}

.editor-head p {
  margin: 0.25rem 0 0;
  color: var(--text-secondary);
  font-size: 0.84rem;
  line-height: 1.5;
}

.editor-body {
  display: grid;
  gap: 0.9rem;
  padding: 1.15rem 1.25rem;
  overflow: auto;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.field-label {
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.field-label em {
  font-weight: 400;
  font-style: normal;
  color: var(--text-muted);
}

.field-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.9rem;
}

.input {
  width: 100%;
  padding: 0.55rem 0.7rem;
  border-radius: 10px;
  border: 1px solid var(--border-color);
  background: var(--bg-main);
  color: var(--text-primary);
  font: inherit;
  font-size: 0.86rem;
}

.input:focus {
  outline: none;
  border-color: var(--text-accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--text-accent) 18%, transparent);
}

.input.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.82rem;
}

.textarea {
  resize: vertical;
  line-height: 1.5;
}

.toggle-row {
  display: flex;
  gap: 1.5rem;
  flex-wrap: wrap;
  padding-top: 0.2rem;
}

.toggle {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.86rem;
  color: var(--text-primary);
  cursor: pointer;
}

.editor-foot {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.6rem;
  padding: 1rem 1.25rem;
  border-top: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 45%, var(--bg-main));
}

/* Confirm dialog -------------------------------------------------------- */
.confirm-dialog {
  width: min(420px, 100%);
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 0.7rem;
  padding: 1.6rem 1.5rem 1.35rem;
  border: 1px solid var(--border-color);
  border-radius: 18px;
  background: var(--bg-main);
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.35);
}

.confirm-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 3rem;
  height: 3rem;
  border-radius: 999px;
  color: color-mix(in srgb, var(--bg-danger) 78%, var(--text-primary));
  background: color-mix(in srgb, var(--bg-danger) 12%, var(--bg-main));
  border: 1px solid color-mix(in srgb, var(--bg-danger) 35%, var(--border-color));
}

.confirm-title {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 600;
  color: var(--text-primary);
}

.confirm-copy {
  margin: 0;
  color: var(--text-secondary);
  font-size: 0.88rem;
  line-height: 1.5;
}

.confirm-actions {
  display: flex;
  gap: 0.6rem;
  margin-top: 0.5rem;
}

/* Responsive ------------------------------------------------------------ */
@media (max-width: 720px) {
  .schedules {
    padding: 1rem;
  }

  .schedules-hero {
    align-items: flex-start;
  }

  .toolbar {
    align-items: flex-start;
    flex-direction: column;
  }

  .field-grid {
    grid-template-columns: 1fr;
  }

  .history-panel {
    width: 100%;
  }
}
</style>
