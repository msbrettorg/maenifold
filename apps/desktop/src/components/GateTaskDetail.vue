<script setup lang="ts">
import { computed, ref } from 'vue';
import { Pencil } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Separator } from '@/components/ui/separator';
import { ScrollArea } from '@/components/ui/scroll-area';
import GateTaskDetailHistory from './GateTaskDetailHistory.vue';
import GateTaskDetailMeta from './GateTaskDetailMeta.vue';
import GateTaskDetailStages from './GateTaskDetailStages.vue';
import GateStageActions from './GateStageActions.vue';
import MarkdownView from './MarkdownView.vue';
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

const titleHelpOpen = ref(false);

function openTitleHelp() {
  if (props.task) titleHelpOpen.value = true;
}

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
            <Button
              v-if="task"
              type="button"
              variant="ghost"
              size="sm"
              class="-ml-2 h-auto max-w-full justify-start px-2 py-1 text-base font-semibold text-foreground hover:underline"
              :aria-label="`Edit raw markdown title for ${task.taskId}`"
              data-testid="gate-task-title-edit"
              @click="openTitleHelp"
            >
              <span class="truncate">{{ `${task.taskId} ${task.title}` }}</span>
              <Pencil class="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
            </Button>
            <span v-else>Select a task</span>
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
          <MarkdownView
            v-else
            :content="body"
            wikilink-action="search-knowledge"
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

  <Dialog :open="titleHelpOpen" @update:open="titleHelpOpen = $event">
    <DialogContent v-if="titleHelpOpen && task" class="border-border-card bg-bg-surface text-text-primary sm:max-w-lg">
      <DialogHeader>
        <DialogTitle>Edit raw markdown</DialogTitle>
        <DialogDescription>
          Title editing requires backend gate_UpdateTitle (filed T-249.169). For now, edit the markdown file directly.
        </DialogDescription>
      </DialogHeader>
      <p class="rounded-md border border-border bg-muted/30 p-3 font-mono text-xs text-muted-foreground break-all">
        {{ task.path }}
      </p>
      <DialogFooter>
        <Button type="button" variant="ghost" @click="titleHelpOpen = false">Close</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
