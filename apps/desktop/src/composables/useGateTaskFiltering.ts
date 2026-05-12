import { computed, ref, watch, type Ref } from 'vue';
import type { GateSnapshot, GateTask } from '../lib/gate';
import { getTaskPipelineLabel } from '../lib/gate-display';

export type GateReadinessFilter = 'all' | 'ready' | 'blocked';
export type GateSortColumn = 'id' | 'name' | 'process' | 'progress' | 'lastUpdated';
export type GateSortDirection = 'asc' | 'desc';

type GateTaskReadiness = GateTask & {
  ready?: boolean | null;
  blockedBy?: string[] | null;
  blocked_by?: string[] | null;
  missingDependencies?: string[] | null;
  missing_dependencies?: string[] | null;
  inCycle?: boolean | null;
  in_cycle?: boolean | null;
};

const defaultSortColumn: GateSortColumn = 'id';
const defaultSortDirection: GateSortDirection = 'asc';

function initialSortDirection(column: GateSortColumn): GateSortDirection {
  return column === 'lastUpdated' ? 'desc' : 'asc';
}

function oppositeDirection(direction: GateSortDirection): GateSortDirection {
  return direction === 'asc' ? 'desc' : 'asc';
}

function uniqueValues(values: string[]) {
  return [...new Set(values.filter(Boolean))].sort((left, right) => left.localeCompare(right));
}

export function processValue(task: GateTask) {
  return getTaskPipelineLabel(task);
}

export function progressRank(task: GateTask) {
  if (task.location === 'done') return task.stages?.length || 1;
  if (!task.stages?.length) return task.state === 'pending' ? 0 : 1;
  const index = task.stages.indexOf(task.state);
  return index < 0 ? 0 : index;
}

function lastUpdatedTime(task: GateTask) {
  if (!task.lastUpdatedAt) return 0;
  const time = Date.parse(task.lastUpdatedAt);
  return Number.isNaN(time) ? 0 : time;
}

function hasValues(values: string[] | null | undefined) {
  return Array.isArray(values) && values.length > 0;
}

function hasUnmetDependency(task: GateTaskReadiness) {
  return hasValues(task.blockedBy)
    || hasValues(task.blocked_by)
    || hasValues(task.missingDependencies)
    || hasValues(task.missing_dependencies)
    || task.inCycle === true
    || task.in_cycle === true;
}

function hasTaskError(task: GateTask) {
  return task.state === 'error' || Boolean(task.issue?.trim());
}

export function isReadyTask(task: GateTaskReadiness) {
  return task.state === 'pending'
    && !hasTaskError(task)
    && (task.ready === true || (task.ready !== false && !hasUnmetDependency(task)));
}

export function isBlockedTask(task: GateTaskReadiness) {
  return task.state === 'error'
    || (task.state === 'pending' && (hasTaskError(task) || hasUnmetDependency(task) || task.ready === false));
}

function matchesReadiness(task: GateTaskReadiness, filter: GateReadinessFilter) {
  if (filter === 'ready') return isReadyTask(task);
  if (filter === 'blocked') return isBlockedTask(task);
  return true;
}

export function useGateTaskFiltering(snapshot: Ref<GateSnapshot | null>) {
  const searchText = ref('');
  const selectedProcess = ref('all');
  const selectedState = ref('all');
  const selectedLocation = ref('todo');
  const readinessFilter = ref<GateReadinessFilter>('all');
  const issuesOnly = ref(false);
  const sortColumn = ref<GateSortColumn>(defaultSortColumn);
  const sortDirection = ref<GateSortDirection>(defaultSortDirection);

  const processOptions = computed(() => uniqueValues((snapshot.value?.tasks ?? []).map(processValue)));
  const stateOptions = computed(() => uniqueValues((snapshot.value?.tasks ?? []).map((task) => task.state)));
  const locationOptions = computed(() => uniqueValues((snapshot.value?.tasks ?? []).map((task) => task.location)));

  const visibleTasks = computed(() => {
    const search = searchText.value.trim().toLowerCase();

    return [...(snapshot.value?.tasks ?? [])]
      .filter((task) => {
        const haystack = [
          task.taskId,
          task.title,
          task.path,
          task.issue,
          processValue(task),
          task.state,
          task.location,
        ].join('\n').toLowerCase();

        return (!search || haystack.includes(search))
          && (selectedProcess.value === 'all' || processValue(task) === selectedProcess.value)
          && (selectedState.value === 'all' || task.state === selectedState.value)
          && (selectedLocation.value === 'all' || task.location === selectedLocation.value)
          && matchesReadiness(task, readinessFilter.value)
          && (!issuesOnly.value || Boolean(task.issue));
      })
      .sort((left, right) => {
        const direction = sortDirection.value === 'asc' ? 1 : -1;
        const compare = {
          id: (left.taskId ?? '').localeCompare(right.taskId ?? '', undefined, { numeric: true }),
          name: (left.title ?? '').localeCompare(right.title ?? ''),
          process: processValue(left).localeCompare(processValue(right)),
          progress: progressRank(left) - progressRank(right),
          lastUpdated: lastUpdatedTime(left) - lastUpdatedTime(right),
        }[sortColumn.value];

        return (compare ?? 0) * direction || (left.taskId ?? '').localeCompare(right.taskId ?? '', undefined, { numeric: true });
      });
  });

  function toggleSort(column: GateSortColumn) {
    const firstDirection = initialSortDirection(column);
    if (sortColumn.value !== column) {
      sortColumn.value = column;
      sortDirection.value = firstDirection;
      return;
    }
    if (sortDirection.value === firstDirection) {
      sortDirection.value = oppositeDirection(firstDirection);
      return;
    }
    sortColumn.value = defaultSortColumn;
    sortDirection.value = defaultSortDirection;
  }

  watch(processOptions, (options) => {
    if (selectedProcess.value !== 'all' && !options.includes(selectedProcess.value)) {
      selectedProcess.value = 'all';
    }
  });
  watch(stateOptions, (options) => {
    if (selectedState.value !== 'all' && !options.includes(selectedState.value)) {
      selectedState.value = 'all';
    }
  });
  watch(locationOptions, (options) => {
    if (selectedLocation.value !== 'all' && !options.includes(selectedLocation.value)) {
      selectedLocation.value = 'all';
    }
  });

  return {
    searchText,
    selectedProcess,
    selectedState,
    selectedLocation,
    readinessFilter,
    issuesOnly,
    sortColumn,
    sortDirection,
    processOptions,
    stateOptions,
    locationOptions,
    visibleTasks,
    toggleSort,
  };
}
