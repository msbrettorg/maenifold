<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { storeToRefs } from 'pinia';
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
  Trash2,
  X,
} from '@lucide/vue';
import { useSchedulesStore } from '../stores/schedules';
import type {
  McpActionOption,
  RunRecord,
  RunStatus,
  ScheduledTask,
} from '../stores/schedules';
import { useAgentsAdminStore } from '../stores/agentsAdmin';

type RowState = 'enabled' | 'paused' | 'failed';
type RunMode = 'agent' | 'mcp_action';

const schedulesStore = useSchedulesStore();
const {
  tasks,
  loading,
  error,
  lastRefreshed,
  runningIds,
  historyByTaskId,
  mcpActionOptions,
  allowedToolOptions,
  optionsLoaded,
} = storeToRefs(schedulesStore);

const agentsAdminStore = useAgentsAdminStore();
const { agents: agentsByName } = storeToRefs(agentsAdminStore);
const agentNames = computed<string[]>(() =>
  Object.keys(agentsByName.value).sort((a, b) => a.localeCompare(b)),
);
const hasAgents = computed<boolean>(() => agentNames.value.length > 0);

onMounted(() => {
  void schedulesStore.loadTasks();
  void schedulesStore.loadOptions();
  void agentsAdminStore.fetchConfig().catch(() => undefined);
});

const mcpActionByValue = computed<Map<string, McpActionOption>>(
  () => new Map(mcpActionOptions.value.map((option) => [option.value, option])),
);

function describeMcpAction(value: string | null): string {
  if (!value) return 'No MCP action';
  const option = mcpActionByValue.value.get(value);
  return option ? option.label : value;
}

function formatFreshness(value: Date | null): string {
  if (!value) return 'never';
  const seconds = Math.max(0, Math.round((Date.now() - value.getTime()) / 1000));
  if (seconds < 5) return 'just now';
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  return `${hours}h ago`;
}

const freshnessLabel = computed<string>(() => formatFreshness(lastRefreshed.value));
const isStale = computed<boolean>(
  () => !lastRefreshed.value || Date.now() - lastRefreshed.value.getTime() > 120_000,
);

function refresh(): void {
  void schedulesStore.loadTasks();
}

function rowState(task: ScheduledTask): RowState {
  if (!task.enabled) return 'paused';
  if (task.last_run?.status === 'failed') return 'failed';
  return 'enabled';
}

function isRunning(taskId: string): boolean {
  return runningIds.value.has(taskId);
}

const stateLabels: Record<RowState, string> = {
  enabled: 'Enabled',
  paused: 'Paused',
  failed: 'Last run failed',
};

function runNow(task: ScheduledTask): void {
  void schedulesStore.runTask(task.id);
}

function togglePause(task: ScheduledTask): void {
  void schedulesStore.togglePaused(task.id);
}

const historyOpenId = ref<string | null>(null);

const historyTask = computed<ScheduledTask | null>(
  () => tasks.value.find((task) => task.id === historyOpenId.value) ?? null,
);

const historyRuns = computed<RunRecord[]>(() =>
  historyOpenId.value ? historyByTaskId.value[historyOpenId.value] ?? [] : [],
);

function openHistory(task: ScheduledTask): void {
  historyOpenId.value = task.id;
  void schedulesStore.loadHistory(task.id);
}

function closeHistory(): void {
  historyOpenId.value = null;
}

interface EditorForm {
  id: string | null;
  name: string;
  schedule: string;
  agent: string;
  prompt: string;
  workdir: string;
  auto_grant_permissions: boolean;
  allowed_tools: string[];
  mcp_action: string;
  timeout_seconds: number;
  enabled: boolean;
  run_mode: RunMode;
}

const editorOpen = ref(false);
const editorForm = ref<EditorForm>(emptyForm());
const editorError = ref<string | null>(null);
const editorSaving = ref<boolean>(false);

function pickDefaultAgent(): string {
  if (agentNames.value.length > 0) return agentNames.value[0];
  return '';
}

function emptyForm(): EditorForm {
  return {
    id: null,
    name: '',
    schedule: '',
    agent: pickDefaultAgent(),
    prompt: '',
    workdir: '',
    auto_grant_permissions: false,
    allowed_tools: [],
    mcp_action: '',
    timeout_seconds: 600,
    enabled: true,
    run_mode: 'agent',
  };
}

const editorIsEditing = computed<boolean>(() => editorForm.value.id !== null);

function openCreate(): void {
  editorForm.value = emptyForm();
  editorError.value = null;
  editorOpen.value = true;
}

function openEdit(task: ScheduledTask): void {
  editorForm.value = {
    id: task.id,
    name: task.name,
    schedule: task.schedule,
    agent: task.agent || pickDefaultAgent(),
    prompt: task.prompt,
    workdir: task.workdir ?? '',
    auto_grant_permissions: task.auto_grant_permissions,
    allowed_tools: [...task.allowed_tools],
    mcp_action: task.mcp_action ?? '',
    timeout_seconds: task.timeout_seconds,
    enabled: task.enabled,
    run_mode: task.mcp_action ? 'mcp_action' : 'agent',
  };
  editorError.value = null;
  editorOpen.value = true;
}

function closeEditor(): void {
  editorError.value = null;
  editorOpen.value = false;
}

function createScheduleId(name: string): string {
  // Backend requires alphanumeric + hyphens only (see schedule_helpers.rs
  // validate_id). Collapse any other characters into hyphens.
  const slug = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 48);
  return slug.length > 0 ? `sched-${slug}` : `sched-${Math.random().toString(36).slice(2, 8)}`;
}

function toggleAllowedTool(name: string, checked: boolean): void {
  const set = new Set(editorForm.value.allowed_tools);
  if (checked) {
    set.add(name);
  } else {
    set.delete(name);
  }
  editorForm.value.allowed_tools = Array.from(set);
}

const editorAgentMissing = computed<boolean>(
  () => editorForm.value.run_mode === 'agent' && !hasAgents.value,
);

const editorCanSave = computed<boolean>(() => {
  const form = editorForm.value;
  if (form.name.trim().length === 0) return false;
  if (form.schedule.trim().length === 0) return false;
  if (form.run_mode === 'agent') {
    if (!form.agent) return false;
    if (form.prompt.trim().length === 0) return false;
  } else {
    if (!form.mcp_action) return false;
  }
  return true;
});

async function saveEditor(): Promise<void> {
  const form = editorForm.value;
  if (!editorCanSave.value) return;
  if (editorSaving.value) return;

  const name = form.name.trim();
  const schedule = form.schedule.trim();
  const isMcp = form.run_mode === 'mcp_action';

  const payload = {
    name,
    schedule,
    agent: form.agent,
    prompt: isMcp ? '' : form.prompt.trim(),
    workdir: form.workdir.trim() || null,
    auto_grant_permissions: form.auto_grant_permissions,
    allowed_tools: isMcp ? [] : [...form.allowed_tools],
    mcp_action: isMcp ? form.mcp_action : null,
    timeout_seconds: Number(form.timeout_seconds) || 600,
    enabled: form.enabled,
  };

  editorSaving.value = true;
  editorError.value = null;
  try {
    if (form.id) {
      await schedulesStore.updateTask({ id: form.id, ...payload });
    } else {
      await schedulesStore.createTask({ id: createScheduleId(name), ...payload });
    }
    editorOpen.value = false;
  } catch (err) {
    editorError.value = err instanceof Error ? err.message : String(err);
  } finally {
    editorSaving.value = false;
  }
}

const deleteTarget = ref<ScheduledTask | null>(null);

function askDelete(task: ScheduledTask): void {
  deleteTarget.value = task;
}

function cancelDelete(): void {
  deleteTarget.value = null;
}

async function confirmDelete(): Promise<void> {
  const target = deleteTarget.value;
  if (!target) return;

  await schedulesStore.deleteTask(target.id);
  if (historyOpenId.value === target.id) {
    historyOpenId.value = null;
  }
  deleteTarget.value = null;
}

const enabledCount = computed<number>(() => tasks.value.filter((task) => task.enabled).length);

function formatTimestamp(iso: string | null): string {
  if (!iso) return 'Unknown time';
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleString(undefined, {
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
            :disabled="loading"
            @click="refresh"
          >
            <RefreshCw :size="15" :class="{ spinning: loading }" />
            <span>{{ loading ? 'Refreshing…' : 'Refresh' }}</span>
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

        <div v-if="error && tasks.length === 0" class="schedules-state schedules-state-error">
          {{ error }}
        </div>

        <div v-else-if="loading && tasks.length === 0" class="schedules-state">
          Loading schedules...
        </div>

        <div v-else-if="tasks.length === 0" class="schedules-state">
          No schedules yet. Create one to get started.
        </div>

        <div v-else class="schedules-table-wrap">
          <div v-if="error" class="schedules-banner">
            <AlertCircle :size="14" />
            <span>{{ error }}</span>
          </div>
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
                        <span v-if="task.mcp_action" class="mcp-tag" :title="task.mcp_action">
                          {{ describeMcpAction(task.mcp_action) }}
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
                    <button
                      class="icon-btn"
                      type="button"
                      title="Run now"
                      aria-label="Run now"
                      :disabled="isRunning(task.id)"
                      @click="runNow(task)"
                    >
                      <RefreshCw v-if="isRunning(task.id)" :size="15" class="spinning" />
                      <Play v-else :size="15" />
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
              <span class="field-label">Timeout (seconds)</span>
              <input v-model.number="editorForm.timeout_seconds" class="input mono" type="number" min="1" />
            </label>
          </div>

          <div class="field">
            <span class="field-label">What runs</span>
            <div class="segmented" role="radiogroup" aria-label="Run mode">
              <button
                type="button"
                class="seg-btn"
                :class="{ active: editorForm.run_mode === 'agent' }"
                role="radio"
                :aria-checked="editorForm.run_mode === 'agent'"
                @click="editorForm.run_mode = 'agent'"
              >
                ACP agent prompt
              </button>
              <button
                type="button"
                class="seg-btn"
                :class="{ active: editorForm.run_mode === 'mcp_action' }"
                role="radio"
                :aria-checked="editorForm.run_mode === 'mcp_action'"
                @click="editorForm.run_mode = 'mcp_action'"
              >
                MCP action
              </button>
            </div>
            <p class="field-hint">
              An agent runs a prompt through the ACP runtime. An MCP action calls one
              allow-listed gateway command directly — no inference.
            </p>
          </div>

          <template v-if="editorForm.run_mode === 'agent'">
            <label class="field">
              <span class="field-label">Agent</span>
              <select
                v-model="editorForm.agent"
                class="input"
                :disabled="editorAgentMissing"
              >
                <option v-if="editorAgentMissing" value="">No agents configured</option>
                <option
                  v-for="name in agentNames"
                  :key="name"
                  :value="name"
                >
                  {{ name }}
                </option>
              </select>
              <p v-if="editorAgentMissing" class="field-hint">
                Add agents in <code>~/maenifold/maenifold.json</code> under
                <code>agents</code>, then refresh.
              </p>
            </label>

            <label class="field">
              <span class="field-label">Prompt</span>
              <textarea
                v-model="editorForm.prompt"
                class="input textarea"
                rows="3"
                placeholder="Re-embed changed documents and refresh the vector index."
              />
            </label>

            <label class="field">
              <span class="field-label">Workdir <em>(optional)</em></span>
              <input v-model="editorForm.workdir" class="input mono" type="text" placeholder="/path/to/repo" />
            </label>

            <div class="field">
              <span class="field-label">Allowed tools</span>
              <p class="field-hint">
                Tools the ACP agent may call during this run. Backed by the daemon's
                MCP wire surface.
              </p>
              <div class="checklist">
                <label
                  v-for="tool in allowedToolOptions"
                  :key="tool.name"
                  class="check-row"
                >
                  <input
                    type="checkbox"
                    :checked="editorForm.allowed_tools.includes(tool.name)"
                    @change="toggleAllowedTool(tool.name, ($event.target as HTMLInputElement).checked)"
                  />
                  <span class="check-text">
                    <code class="check-name">{{ tool.name }}</code>
                    <span class="check-desc">{{ tool.description }}</span>
                  </span>
                </label>
              </div>
            </div>
          </template>

          <template v-else>
            <label class="field">
              <span class="field-label">MCP action</span>
              <select v-model="editorForm.mcp_action" class="input">
                <option value="" disabled>Select an allow-listed action…</option>
                <option
                  v-for="opt in mcpActionOptions"
                  :key="opt.value"
                  :value="opt.value"
                >
                  {{ opt.label }} — {{ opt.command }}
                </option>
              </select>
              <p v-if="editorForm.mcp_action" class="field-hint">
                {{ mcpActionByValue.get(editorForm.mcp_action)?.description }}
              </p>
              <p v-else-if="!optionsLoaded" class="field-hint">
                Loading allow-listed actions from the daemon…
              </p>
              <p v-else class="field-hint">
                Only inference-free gateway commands are schedulable.
              </p>
            </label>

            <label class="field">
              <span class="field-label">Workdir <em>(optional)</em></span>
              <input v-model="editorForm.workdir" class="input mono" type="text" placeholder="/path/to/repo" />
            </label>
          </template>

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

        <p v-if="editorError" class="editor-error" role="alert">{{ editorError }}</p>

        <footer class="editor-foot">
          <button class="btn" type="button" @click="closeEditor" :disabled="editorSaving">Cancel</button>
          <button
            class="btn btn-primary"
            type="button"
            :disabled="!editorCanSave || editorSaving"
            @click="saveEditor"
          >
            <Check :size="15" />
            <span>{{ editorSaving ? 'Saving…' : editorIsEditing ? 'Save changes' : 'Create schedule' }}</span>
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

.schedules-state-error {
  color: color-mix(in srgb, var(--bg-danger) 78%, var(--text-primary));
}

.schedules-banner {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  margin: 1rem 1rem 0;
  padding: 0.75rem 0.85rem;
  border: 1px solid color-mix(in srgb, var(--bg-danger) 35%, var(--border-color));
  border-radius: 12px;
  color: color-mix(in srgb, var(--bg-danger) 78%, var(--text-primary));
  background: color-mix(in srgb, var(--bg-danger) 10%, var(--bg-main));
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

.field-hint {
  margin: 0.35rem 0 0;
  font-size: 0.78rem;
  color: var(--text-secondary);
  line-height: 1.45;
}

.field-hint code {
  font-family: var(--font-mono, ui-monospace, SFMono-Regular, monospace);
  font-size: 0.74rem;
  padding: 0.1rem 0.3rem;
  border-radius: 4px;
  background: color-mix(in srgb, var(--text-accent) 8%, transparent);
}

.segmented {
  display: inline-flex;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  overflow: hidden;
  background: color-mix(in srgb, var(--bg-sidebar) 50%, var(--bg-main));
}

.seg-btn {
  appearance: none;
  border: 0;
  background: transparent;
  color: var(--text-secondary);
  padding: 0.45rem 0.85rem;
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease;
}

.seg-btn:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-accent) 6%, transparent);
}

.seg-btn.active {
  background: color-mix(in srgb, var(--text-accent) 16%, transparent);
  color: var(--text-accent);
}

.seg-btn + .seg-btn {
  border-left: 1px solid var(--border-color);
}

.checklist {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  margin-top: 0.5rem;
  padding: 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-sidebar) 35%, var(--bg-main));
}

.check-row {
  display: flex;
  align-items: flex-start;
  gap: 0.55rem;
  padding: 0.35rem 0.45rem;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.12s ease;
}

.check-row:hover {
  background: color-mix(in srgb, var(--text-accent) 6%, transparent);
}

.check-row input[type="checkbox"] {
  margin-top: 0.18rem;
}

.check-text {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
}

.check-name {
  font-family: var(--font-mono, ui-monospace, SFMono-Regular, monospace);
  font-size: 0.82rem;
  color: var(--text-primary);
}

.check-desc {
  font-size: 0.76rem;
  color: var(--text-secondary);
  line-height: 1.4;
}

select.input {
  appearance: none;
  background-image:
    linear-gradient(45deg, transparent 50%, var(--text-secondary) 50%),
    linear-gradient(135deg, var(--text-secondary) 50%, transparent 50%);
  background-position:
    calc(100% - 18px) 50%,
    calc(100% - 13px) 50%;
  background-size: 5px 5px, 5px 5px;
  background-repeat: no-repeat;
  padding-right: 2.1rem;
}

select.input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
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

.editor-error {
  margin: 0;
  padding: 0.75rem 1.25rem;
  border-top: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--accent-danger, #d93b3b) 12%, transparent);
  color: var(--accent-danger, #d93b3b);
  font-size: 0.85rem;
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  word-break: break-word;
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
