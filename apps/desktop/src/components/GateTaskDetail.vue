<script setup lang="ts">
import { computed } from 'vue';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { ScrollArea } from '@/components/ui/scroll-area';
import GateTaskDetailHistory from './GateTaskDetailHistory.vue';
import GateTaskDetailMeta from './GateTaskDetailMeta.vue';
import GateTaskDetailStages from './GateTaskDetailStages.vue';
import GateStageActions from './GateStageActions.vue';
import KnowledgeNoteRenderer from './KnowledgeNoteRenderer.vue';
import type { GateTask, GateTaskHistoryEntry } from '../lib/gate';
import { getTaskPipelineLabel, isTaskPipelineDigestStale } from '../lib/gate-display';

const props = defineProps<{
  task: GateTask | null;
  body: string;
  history: GateTaskHistoryEntry[];
  bodyLoading: boolean;
  historyLoading: boolean;
  bodyError: string;
  historyError: string;
  root: string;
}>();

defineEmits<{ close: []; 'action-complete': [] }>();

const processLabel = computed(() => {
  if (!props.task) return '';
  const pipeline = getTaskPipelineLabel(props.task);
  return props.task.location === 'done' ? `${pipeline} done` : pipeline;
});

const pipelineStale = computed(() => Boolean(props.task && isTaskPipelineDigestStale(props.task)));

const detailTitleId = computed(() =>
  props.task ? `gate-task-detail-title-${props.task.taskId}` : 'gate-task-detail-title-empty'
);
</script>

<template>
  <Card
    as="aside"
    class="flex flex-col gap-4 flex-[3] min-w-0 min-h-0 overflow-hidden p-4"
    role="article"
    :aria-labelledby="detailTitleId"
  >
    <CardHeader class="p-0">
      <div class="flex justify-between items-start gap-3">
        <div class="flex flex-col min-w-0 gap-2">
          <p class="text-xs text-muted-foreground uppercase tracking-wide">Task detail</p>
          <h3 :id="detailTitleId" class="text-base font-semibold text-foreground truncate">
            {{ task ? `${task.taskId} ${task.title}` : 'Select a task' }}
          </h3>
          <GateTaskDetailMeta
            v-if="task"
            :task="task"
            :process-label="processLabel"
            :pipeline-stale="pipelineStale"
          />
        </div>
        <Button v-if="task" variant="ghost" size="sm" @click="$emit('close')">Clear</Button>
      </div>
    </CardHeader>

    <Separator v-if="task" />

    <CardContent v-if="task" class="p-0 flex flex-col gap-4 min-h-0 flex-1">
      <ScrollArea class="max-h-[min(52vh,34rem)] pr-1">
      <div class="flex flex-col gap-4 min-h-0">
        <section class="flex flex-col gap-3">
          <p class="text-xs text-muted-foreground uppercase tracking-wide">Spec</p>

          <p
            v-if="bodyLoading"
            class="p-3 border border-border rounded-md bg-muted/40 text-sm text-muted-foreground"
          >
            Loading task detail…
          </p>
          <p
            v-else-if="bodyError"
            class="p-3 border border-destructive/40 rounded-md bg-destructive/10 text-sm text-destructive"
          >
            {{ bodyError }}
          </p>
          <p
            v-else-if="!body.trim()"
            class="p-3 border border-border rounded-md bg-muted/40 text-sm text-muted-foreground"
          >
            No task body content found.
          </p>
          <KnowledgeNoteRenderer
            v-else
            :content="body"
            class="p-4 border border-border rounded-md bg-muted/30"
          />
        </section>

        <GateStageActions
          v-if="task.location === 'todo'"
          :task="task"
          :root="root"
          @completed="$emit('action-complete')"
        />

        <GateTaskDetailStages :task="task" />

        <p
          v-if="task.issue"
          class="p-3 border border-destructive/40 rounded-md bg-destructive/10 text-sm text-destructive"
        >
          {{ task.issue }}
        </p>
      </div>
      </ScrollArea>

      <Separator />

      <GateTaskDetailHistory
        :history="history"
        :history-loading="historyLoading"
        :history-error="historyError"
      />
    </CardContent>

    <CardContent v-else class="p-0">
      <p class="p-3 border border-border rounded-md bg-muted/40 text-sm text-muted-foreground">
        Select a task from the table to inspect its gate history, path, and gate metadata.
      </p>
    </CardContent>
  </Card>
</template>
