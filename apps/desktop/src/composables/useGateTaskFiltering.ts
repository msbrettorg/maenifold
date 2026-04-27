import { computed, ref, watch, type Ref } from 'vue';
import type { GateSnapshot, GateTask } from '../lib/gate';
import { getTaskPipelineLabel } from '../lib/gate-display';

export type GateSortColumn = 'id' | 'name' | 'process' | 'progress';
export type GateSortDirection = 'asc' | 'desc';

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

export function useGateTaskFiltering(snapshot: Ref<GateSnapshot | null>) {
  const searchText = ref('');
  const selectedProcess = ref('all');
  const selectedState = ref('all');
  const selectedLocation = ref('todo');
  const issuesOnly = ref(false);
  const sortColumn = ref<GateSortColumn>('id');
  const sortDirection = ref<GateSortDirection>('asc');

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
          && (!issuesOnly.value || Boolean(task.issue));
      })
      .sort((left, right) => {
        const direction = sortDirection.value === 'asc' ? 1 : -1;
        const compare = {
          id: (left.taskId ?? '').localeCompare(right.taskId ?? '', undefined, { numeric: true }),
          name: (left.title ?? '').localeCompare(right.title ?? ''),
          process: processValue(left).localeCompare(processValue(right)),
          progress: progressRank(left) - progressRank(right),
        }[sortColumn.value];

        return (compare ?? 0) * direction || (left.taskId ?? '').localeCompare(right.taskId ?? '', undefined, { numeric: true });
      });
  });

  function toggleSort(column: GateSortColumn) {
    if (sortColumn.value === column) {
      sortDirection.value = sortDirection.value === 'asc' ? 'desc' : 'asc';
      return;
    }
    sortColumn.value = column;
    sortDirection.value = 'asc';
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
