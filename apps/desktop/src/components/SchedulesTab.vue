<script setup lang="ts">
import { computed, ref } from 'vue';
import { ChevronDown, ChevronUp, ChevronsUpDown, History, Loader2, Pencil, Play, Plus, RefreshCw, Trash2, Wrench } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import ScheduleFormDialog from '@/components/ScheduleFormDialog.vue';
import ScheduleRowActions from '@/components/ScheduleRowActions.vue';
import ScheduleRunsDrawer from '@/components/ScheduleRunsDrawer.vue';
import SchedulesEmptyState from '@/components/SchedulesEmptyState.vue';
import { useSchedulesSettings } from '@/composables/useSchedulesSettings';

const {
  agentNames,
  jobs,
  isLoading,
  isSubmitting,
  isActionInProgress,
  installStatusFor,
  error,
  statusMessage,
  dialogOpen,
  dialogMode,
  dialogTitle,
  dialogDescription,
  canSubmit,
  previewIsValid,
  form,
  schedulePlatform,
  pauseState,
  isPausedAll,
  scheduleLabel,
  statusFor,
  installBadgeFor,
  permissionLabel,
  permissionVariant,
  allowedToolsLabel,
  formatLastRun,
  formatPausedAt,
  loadSchedules,
  setPauseAll,
  openCreateDialog,
  openEditDialog,
  saveSchedule,
  runSchedule,
  repairSchedule,
  deleteSchedule,
} = useSchedulesSettings();

const runsDrawerOpen = ref(false);
const runsScheduleId = ref('');
type SortColumn = 'name' | 'schedule' | 'agent' | 'permissions' | 'status' | 'installState' | 'lastRun';
type SortDirection = 'asc' | 'desc';
type ScheduleRow = (typeof jobs.value)[number];
type SortValue = number | string | null | undefined;

const sortColumn = ref<SortColumn | null>(null);
const sortDirection = ref<SortDirection>('asc');

function toggleSort(column: SortColumn): void {
  if (sortColumn.value !== column) {
    sortColumn.value = column;
    sortDirection.value = 'asc';
    return;
  }
  if (sortDirection.value === 'asc') {
    sortDirection.value = 'desc';
    return;
  }
  sortColumn.value = null;
  sortDirection.value = 'asc';
}

function sortAriaOrder(column: SortColumn): 'ascending' | 'descending' | undefined {
  if (sortColumn.value !== column) return undefined;
  return sortDirection.value === 'asc' ? 'ascending' : 'descending';
}

function sortIcon(column: SortColumn) {
  if (sortColumn.value !== column) return ChevronsUpDown;
  return sortDirection.value === 'asc' ? ChevronUp : ChevronDown;
}

function sortValue(job: ScheduleRow, column: SortColumn): SortValue {
  if (column === 'name') return job.id;
  if (column === 'schedule') return job.schedule;
  if (column === 'agent') return job.agent;
  if (column === 'permissions') return permissionLabel(job);
  if (column === 'status') return statusFor(job).label;
  if (column === 'installState') return installBadgeFor(job).label;
  if (column === 'lastRun') return job.last_run?.timestamp ? Date.parse(job.last_run.timestamp) : null;
  return null;
}

function compareSortValues(a: SortValue, b: SortValue): number {
  const aMissing = a === null || a === undefined || a === '';
  const bMissing = b === null || b === undefined || b === '';
  if (aMissing && bMissing) return 0;
  if (aMissing) return 1;
  if (bMissing) return -1;
  const comparison = typeof a === 'number' && typeof b === 'number'
    ? a - b
    : String(a).localeCompare(String(b));
  return sortDirection.value === 'asc' ? comparison : -comparison;
}

const sortedJobs = computed(() => {
  const column = sortColumn.value;
  if (column === null) return jobs.value;
  return [...jobs.value].sort((a, b) => compareSortValues(sortValue(a, column), sortValue(b, column)));
});

function openRunsDrawer(id: string): void {
  runsScheduleId.value = id;
  runsDrawerOpen.value = true;
}
</script>

<template>
  <section class="flex h-full flex-col gap-4 overflow-auto p-6" aria-label="Schedules">
    <div class="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
      <div class="flex flex-col gap-1">
        <h2 class="text-lg font-semibold text-text-primary">Schedules</h2>
        <p class="text-sm text-text-secondary">Recurring agent tasks installed via macOS launchd or Linux systemd user timers.</p>
      </div>
      <div class="flex gap-2">
        <Button type="button" variant="outline" size="sm" :disabled="isLoading || isSubmitting" @click="loadSchedules">
          <RefreshCw class="h-4 w-4" />
          Refresh
        </Button>
        <Button type="button" size="sm" :disabled="isLoading" @click="openCreateDialog">
          <Plus class="h-4 w-4" />
          New schedule
        </Button>
      </div>
    </div>

    <div v-if="error" class="flex items-center justify-between gap-3 rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
      <span>{{ error }}</span>
      <Button type="button" variant="outline" size="sm" @click="loadSchedules">Retry</Button>
    </div>
    <p v-else-if="statusMessage" class="text-sm text-primary" role="status">{{ statusMessage }}</p>

    <div class="flex items-center gap-3 rounded-md border px-4 py-3">
      <Switch
        id="pause-all-schedules"
        :model-value="isPausedAll"
        :disabled="isLoading || isSubmitting || jobs.length === 0"
        @update:model-value="setPauseAll"
      />
      <div class="flex flex-col gap-1">
        <label for="pause-all-schedules" class="text-sm font-medium text-text-primary">Pause all schedules</label>
        <p class="text-sm text-muted-foreground">
          <template v-if="isPausedAll">
            {{ pauseState.schedulesPaused ?? 0 }} schedules paused since {{ formatPausedAt(pauseState.pausedAt) }}. Toggle off to resume prior state.
          </template>
          <template v-else-if="jobs.length > 0">All schedules running normally.</template>
          <template v-else>No schedules to pause.</template>
        </p>
      </div>
    </div>

    <div class="rounded-md border px-4 py-3 text-sm" :class="schedulePlatform.supportsEnabledSchedules ? 'border-primary/30 bg-primary/5 text-text-primary' : 'border-destructive/30 bg-destructive/10 text-destructive'">
      <div class="font-medium">{{ schedulePlatform.label }}</div>
      <p class="mt-1">
        {{ schedulePlatform.description }}
        <span class="text-muted-foreground">Supported backends: macOS launchd, Linux systemd user timers. Windows is unsupported.</span>
      </p>
    </div>

    <div v-if="isLoading" class="flex items-center gap-2 rounded-md border bg-muted/40 px-4 py-6 text-sm text-muted-foreground">
      <Loader2 class="h-4 w-4 animate-spin" />
      Loading schedules…
    </div>

    <SchedulesEmptyState v-else-if="jobs.length === 0" @create="openCreateDialog" />

    <div v-else class="rounded-md border">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead :aria-sort="sortAriaOrder('name')">
              <Button variant="ghost" size="sm" class="-ml-3 h-8" @click="toggleSort('name')"><span>Name</span><component :is="sortIcon('name')" class="h-3.5 w-3.5" :class="sortColumn !== 'name' ? 'opacity-50' : ''" /></Button>
            </TableHead>
            <TableHead :aria-sort="sortAriaOrder('schedule')">
              <Button variant="ghost" size="sm" class="-ml-3 h-8" @click="toggleSort('schedule')"><span>Schedule</span><component :is="sortIcon('schedule')" class="h-3.5 w-3.5" :class="sortColumn !== 'schedule' ? 'opacity-50' : ''" /></Button>
            </TableHead>
            <TableHead :aria-sort="sortAriaOrder('agent')">
              <Button variant="ghost" size="sm" class="-ml-3 h-8" @click="toggleSort('agent')"><span>Agent</span><component :is="sortIcon('agent')" class="h-3.5 w-3.5" :class="sortColumn !== 'agent' ? 'opacity-50' : ''" /></Button>
            </TableHead>
            <TableHead :aria-sort="sortAriaOrder('permissions')">
              <Button variant="ghost" size="sm" class="-ml-3 h-8" @click="toggleSort('permissions')"><span>Permissions</span><component :is="sortIcon('permissions')" class="h-3.5 w-3.5" :class="sortColumn !== 'permissions' ? 'opacity-50' : ''" /></Button>
            </TableHead>
            <TableHead :aria-sort="sortAriaOrder('status')">
              <Button variant="ghost" size="sm" class="-ml-3 h-8" @click="toggleSort('status')"><span>Status</span><component :is="sortIcon('status')" class="h-3.5 w-3.5" :class="sortColumn !== 'status' ? 'opacity-50' : ''" /></Button>
            </TableHead>
            <TableHead :aria-sort="sortAriaOrder('installState')">
              <Button variant="ghost" size="sm" class="-ml-3 h-8" @click="toggleSort('installState')"><span>Install state</span><component :is="sortIcon('installState')" class="h-3.5 w-3.5" :class="sortColumn !== 'installState' ? 'opacity-50' : ''" /></Button>
            </TableHead>
            <TableHead :aria-sort="sortAriaOrder('lastRun')">
              <Button variant="ghost" size="sm" class="-ml-3 h-8" @click="toggleSort('lastRun')"><span>Last run</span><component :is="sortIcon('lastRun')" class="h-3.5 w-3.5" :class="sortColumn !== 'lastRun' ? 'opacity-50' : ''" /></Button>
            </TableHead>
            <TableHead class="text-right">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow v-for="job in sortedJobs" :key="job.id">
            <TableCell class="font-medium text-text-primary">{{ job.id }}</TableCell>
            <TableCell class="max-w-[18rem] text-sm text-muted-foreground">{{ scheduleLabel(job) }}</TableCell>
            <TableCell>{{ job.agent }}</TableCell>
            <TableCell>
              <div class="flex flex-col gap-1">
                <Badge :variant="permissionVariant(job)">{{ permissionLabel(job) }}</Badge>
                <span class="text-xs text-muted-foreground">{{ allowedToolsLabel(job) }}</span>
              </div>
            </TableCell>
            <TableCell><Badge :variant="statusFor(job).variant">{{ statusFor(job).label }}</Badge></TableCell>
            <TableCell>
              <Badge :variant="installBadgeFor(job).variant" :class="installBadgeFor(job).class">
                {{ installBadgeFor(job).label }}
              </Badge>
            </TableCell>
            <TableCell class="max-w-[18rem] text-sm text-muted-foreground">
              <div>{{ formatLastRun(job.last_run?.timestamp) }}</div>
              <p v-if="job.last_run?.error" class="mt-1 text-xs text-destructive">{{ job.last_run.error }}</p>
            </TableCell>
            <TableCell>
              <div class="flex flex-wrap justify-end gap-2">
                <Button type="button" variant="outline" size="sm" :disabled="isActionInProgress(job.id) || isSubmitting" @click="openEditDialog(job)">
                  <Pencil class="h-3.5 w-3.5" />
                  Edit
                </Button>
                <Button type="button" variant="outline" size="sm" :disabled="isActionInProgress(job.id) || isSubmitting" @click="runSchedule(job)">
                  <Loader2 v-if="isActionInProgress(job.id)" class="h-3.5 w-3.5 animate-spin" />
                  <Play v-else class="h-3.5 w-3.5" />
                  Run now
                </Button>
                <Button v-if="installStatusFor(job)?.discrepancy === true" type="button" variant="outline" size="sm" :disabled="isActionInProgress(job.id) || isSubmitting" @click="repairSchedule(job)">
                  <Loader2 v-if="isActionInProgress(job.id)" class="h-3.5 w-3.5 animate-spin" />
                  <Wrench v-else class="h-3.5 w-3.5" />
                  Repair
                </Button>
                <Button type="button" variant="outline" size="sm" :disabled="isSubmitting" @click="openRunsDrawer(job.id)">
                  <History class="h-3.5 w-3.5" />
                  View runs
                </Button>
                <ScheduleRowActions :schedule-id="job.id" @backfilled="loadSchedules" />
                <Button type="button" variant="destructive" size="sm" :disabled="isActionInProgress(job.id) || isSubmitting" @click="deleteSchedule(job)">
                  <Trash2 class="h-3.5 w-3.5" />
                  Delete
                </Button>
              </div>
            </TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </div>

    <ScheduleFormDialog
      v-model:open="dialogOpen"
      :form="form"
      :mode="dialogMode"
      :title="dialogTitle"
      :description="dialogDescription"
      :agent-names="agentNames"
      :is-submitting="isSubmitting"
      :can-submit="canSubmit"
      :schedule-platform="schedulePlatform"
      :is-paused-all="isPausedAll"
      @submit="saveSchedule"
      @update-preview-valid="(valid) => { previewIsValid = valid; }"
    />

    <ScheduleRunsDrawer
      :open="runsDrawerOpen"
      :schedule-id="runsScheduleId"
      :on-update-open="(open) => { runsDrawerOpen = open; }"
    />
  </section>
</template>
