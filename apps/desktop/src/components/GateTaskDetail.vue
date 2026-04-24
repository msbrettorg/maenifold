<script setup lang="ts">
import { computed } from 'vue';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { ScrollArea } from '@/components/ui/scroll-area';
import GateTaskDetailHistory from './GateTaskDetailHistory.vue';
import GateTaskDetailMeta from './GateTaskDetailMeta.vue';
import GateTaskDetailStages from './GateTaskDetailStages.vue';
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
}>();

defineEmits<{ close: [] }>();

const processLabel = computed(() => {
  if (!props.task) return '';
  const pipeline = getTaskPipelineLabel(props.task);
  return props.task.location === 'done' ? `${pipeline} done` : pipeline;
});

const pipelineStale = computed(() => Boolean(props.task && isTaskPipelineDigestStale(props.task)));

const detailTitleId = computed(() =>
  props.task ? `gate-task-detail-title-${props.task.taskId}` : 'gate-task-detail-title-empty'
);

function renderMarkdown(content: string | null) {
  if (!content) return '';
  const parsed = marked.parse(content, { async: false });
  return DOMPurify.sanitize(typeof parsed === 'string' ? parsed : '');
}
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
          <div
            v-else
            class="text-sm text-foreground p-4 border border-border rounded-md bg-muted/30 [overflow-wrap:anywhere] [&_*]:mt-0 [&_p+p]:mt-3 [&_ul]:mt-3 [&_ol]:mt-3 [&_pre]:mt-3 [&_table]:mt-3 [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:list-decimal [&_ol]:pl-5 [&_h1]:text-lg [&_h1]:font-semibold [&_h1]:mt-4 [&_h2]:text-base [&_h2]:font-semibold [&_h2]:mt-4 [&_h3]:text-sm [&_h3]:font-semibold [&_h3]:mt-3 [&_a]:text-primary [&_a]:underline [&_code]:rounded [&_code]:border [&_code]:border-border [&_code]:bg-muted [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-xs [&_pre]:overflow-x-auto [&_pre]:rounded-md [&_pre]:bg-muted [&_pre]:p-3 [&_pre_code]:bg-transparent [&_pre_code]:border-0 [&_pre_code]:p-0 [&_table]:w-full [&_table]:border-collapse [&_th]:border [&_th]:border-border [&_th]:px-3 [&_th]:py-2 [&_th]:text-left [&_td]:border [&_td]:border-border [&_td]:px-3 [&_td]:py-2 [&_blockquote]:border-l-2 [&_blockquote]:border-border [&_blockquote]:pl-3 [&_blockquote]:italic [&_blockquote]:text-muted-foreground"
            v-html="renderMarkdown(body)"
          />
        </section>

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
