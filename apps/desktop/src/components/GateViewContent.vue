<script setup lang="ts">
import { useTemplateRef } from 'vue';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import GateTaskDetail from './GateTaskDetail.vue';
import GateTaskTable from './GateTaskTable.vue';
import type { GateTask, GateTaskHistoryEntry } from '../lib/gate';

type SortColumn = 'id' | 'name' | 'process' | 'progress' | 'lastUpdated' | 'priority' | 'default';
type SortDirection = 'asc' | 'desc';

defineProps<{
  tasks: GateTask[];
  selectedTaskKey: string;
  sortColumn: SortColumn;
  sortDirection: SortDirection;
  activeTask: GateTask | null;
  body: string;
  history: GateTaskHistoryEntry[];
  bodyLoading: boolean;
  historyLoading: boolean;
  bodyError: string;
  historyError: string;
  root: string;
}>();

defineEmits<{
  sort: [column: SortColumn];
  select: [task: GateTask];
  close: [];
  'action-complete': [];
}>();

type DetailExposed = { openAppend: () => void; openPass: () => void; openFail: () => void };
const detailPanel = useTemplateRef<DetailExposed>('detailPanel');
defineExpose({
  openAppend: () => detailPanel.value?.openAppend(),
  openPass: () => detailPanel.value?.openPass(),
  openFail: () => detailPanel.value?.openFail(),
});
</script>

<template>
  <div class="flex flex-1 min-h-0">
    <ResizablePanelGroup
      direction="horizontal"
      auto-save-id="gate-layout"
      class="h-full w-full"
    >
      <ResizablePanel
        id="gate-task-table"
        :default-size="45"
        :min-size="15"
        :order="1"
      >
        <div class="flex h-full w-full min-h-0 min-w-0">
          <GateTaskTable
            :tasks="tasks"
            :selected-task-key="selectedTaskKey"
            :sort-column="sortColumn"
            :sort-direction="sortDirection"
            @sort="$emit('sort', $event)"
            @select="$emit('select', $event)"
          />
        </div>
      </ResizablePanel>
      <template v-if="selectedTaskKey">
        <ResizableHandle
          class="w-px shrink-0 cursor-col-resize bg-border after:w-4 after:bg-transparent after:transition-colors hover:after:bg-accent/70 focus-visible:after:bg-accent/70 data-[state=drag]:after:bg-accent"
        />
        <ResizablePanel
          id="gate-task-detail"
          :default-size="55"
          :min-size="30"
          :order="2"
        >
          <div class="flex h-full w-full min-h-0 min-w-0 pl-4" data-testid="gate-detail-panel">
            <GateTaskDetail
              ref="detailPanel"
              :task="activeTask"
              :body="body"
              :history="history"
              :body-loading="bodyLoading"
              :history-loading="historyLoading"
              :body-error="bodyError"
              :history-error="historyError"
              :root="root"
              @close="$emit('close')"
              @action-complete="$emit('action-complete')"
            />
          </div>
        </ResizablePanel>
      </template>
    </ResizablePanelGroup>
  </div>
</template>
