<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { Clock, FileText, GitBranch, MessageSquare } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { useMemoryApi } from '../composables/useMemoryApi';
import { usePipelineMcp } from '../composables/usePipelineMcp';
import { useKnowledgeStore } from '../stores/knowledge';
import { usePipelineStore } from '../stores/pipeline';
import type { RecentActivityItem } from '../lib/workflow-types';
import type { PipelineListItem, PipelineView } from '../lib/pipeline-types-view';
import type { SavedSession } from '../lib/types';
import type { AppShellTab } from '../stores/app-shell';

type RecentKind = 'session' | 'pipeline' | 'note';
type PipelineViewTimestampNode = {
  startedAt?: string | number | null;
  completedAt?: string | number | null;
};

interface RecentRow {
  id: string;
  title: string;
  kind: RecentKind;
  timestamp: number;
  sourceIndex: number;
  payload: SavedSession | PipelineListItem | RecentActivityItem;
}

const props = withDefaults(defineProps<{
  recentSessions?: SavedSession[];
}>(), {
  recentSessions: () => [],
});

const emit = defineEmits<{
  'open-tab': [tab: AppShellTab];
  'resume-session': [session: SavedSession];
}>();

const memoryApi = useMemoryApi();
const pipelineMcp = usePipelineMcp();
const knowledgeStore = useKnowledgeStore();
const pipelineStore = usePipelineStore();
const { pipelines } = storeToRefs(pipelineStore);
const notes = ref<RecentActivityItem[]>([]);
const pipelineTimestamps = ref<Record<string, number>>({});
const UNKNOWN_TIMESTAMP = 0;

const rtf = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });

const rows = computed(() => {
  const sessionRows = props.recentSessions.map((session, sourceIndex) => ({
    id: `session:${session.id}`,
    title: session.title || session.sessionId,
    kind: 'session' as const,
    timestamp: sortableTimestamp(session.lastUpdated),
    sourceIndex,
    payload: session,
  }));

  const pipelineRows = pipelines.value.map((pipeline, sourceIndex) => ({
    id: `pipeline:${pipeline.pipelineId}`,
    title: pipeline.pipelineId,
    kind: 'pipeline' as const,
    timestamp: pipelineTimestamps.value[pipeline.pipelineId] ?? UNKNOWN_TIMESTAMP,
    sourceIndex,
    payload: pipeline,
  }));

  const noteRows = notes.value.map((note, sourceIndex) => ({
    id: `note:${note.uri}`,
    title: note.title || note.uri,
    kind: 'note' as const,
    timestamp: sortableTimestamp(note.modified),
    sourceIndex,
    payload: note,
  }));

  return [...sessionRows, ...pipelineRows, ...noteRows]
    .filter((row) => row.title.trim().length > 0)
    .sort((left, right) =>
      right.timestamp - left.timestamp
      || kindRank(left.kind) - kindRank(right.kind)
      || left.sourceIndex - right.sourceIndex
    )
    .slice(0, 10);
});

onMounted(async () => {
  await Promise.all([pipelineStore.refreshList(), refreshRecentNotes()]);
  await refreshPipelineTimestamps();
});

async function refreshRecentNotes(): Promise<void> {
  try {
    notes.value = (await memoryApi.recentActivity({ filter: 'memory', limit: 5 })).items;
  } catch {
    notes.value = [];
  }
}

function parseTimestamp(value: string | number | null | undefined): number {
  if (typeof value === 'number') return value;
  if (!value) return Number.NaN;
  const parsed = new Date(value).getTime();
  return Number.isFinite(parsed) ? parsed : Number.NaN;
}

function sortableTimestamp(value: string | number | null | undefined): number {
  const parsed = parseTimestamp(value);
  return Number.isFinite(parsed) ? parsed : UNKNOWN_TIMESTAMP;
}

async function refreshPipelineTimestamps(): Promise<void> {
  const entries = await Promise.all(pipelines.value.map(async (pipeline) => {
    try {
      const view = await pipelineMcp.viewPipeline(pipeline.pipelineId);
      return [pipeline.pipelineId, latestPipelineActivity(view)] as const;
    } catch {
      return [pipeline.pipelineId, UNKNOWN_TIMESTAMP] as const;
    }
  }));
  pipelineTimestamps.value = Object.fromEntries(entries);
}

function latestPipelineActivity(view: PipelineView): number {
  return view.nodes.reduce((latest, node) => {
    const record = node as PipelineViewTimestampNode;
    return Math.max(
      latest,
      sortableTimestamp(record.completedAt),
      sortableTimestamp(record.startedAt),
    );
  }, UNKNOWN_TIMESTAMP);
}

function kindRank(kind: RecentKind): number {
  if (kind === 'session') return 0;
  if (kind === 'pipeline') return 1;
  return 2;
}

function kindLabel(kind: RecentKind): string {
  if (kind === 'session') return 'session';
  if (kind === 'pipeline') return 'pipeline';
  return 'note';
}

function relativeTime(timestamp: number): string {
  if (timestamp === UNKNOWN_TIMESTAMP) return 'date unknown';
  const diffSeconds = Math.round((timestamp - Date.now()) / 1000);
  const absSeconds = Math.abs(diffSeconds);
  if (absSeconds < 60) return rtf.format(diffSeconds, 'second');
  const diffMinutes = Math.round(diffSeconds / 60);
  if (Math.abs(diffMinutes) < 60) return rtf.format(diffMinutes, 'minute');
  const diffHours = Math.round(diffMinutes / 60);
  if (Math.abs(diffHours) < 24) return rtf.format(diffHours, 'hour');
  const diffDays = Math.round(diffHours / 24);
  if (Math.abs(diffDays) < 30) return rtf.format(diffDays, 'day');
  const diffMonths = Math.round(diffDays / 30);
  if (Math.abs(diffMonths) < 12) return rtf.format(diffMonths, 'month');
  return rtf.format(Math.round(diffMonths / 12), 'year');
}

async function openRow(row: RecentRow): Promise<void> {
  if (row.kind === 'session') {
    emit('open-tab', 'agents');
    emit('resume-session', row.payload as SavedSession);
    return;
  }
  if (row.kind === 'pipeline') {
    const pipeline = row.payload as PipelineListItem;
    if (await pipelineStore.loadPipeline(pipeline.pipelineId)) {
      emit('open-tab', 'pipelines');
    }
    return;
  }
  const note = row.payload as RecentActivityItem;
  knowledgeStore.setActiveSubtab('memory');
  knowledgeStore.openNote(note.uri);
  emit('open-tab', 'knowledge');
}
</script>

<template>
  <Card class="mt-4 w-full max-w-[800px] rounded-xl border border-border-card bg-bg-elevated py-0 shadow-fluent-8">
    <CardHeader class="flex flex-row items-center justify-between gap-3 px-4 pb-2 pt-4">
      <CardTitle class="flex items-center gap-2 text-sm font-semibold text-text-primary">
        <Clock class="size-4 text-text-tertiary" :stroke-width="1.5" />
        Recent activity
      </CardTitle>
    </CardHeader>
    <CardContent class="px-3 pb-3">
      <p v-if="rows.length === 0" class="px-3 py-5 text-sm text-text-secondary">
        Nothing here yet — open an Agents session, run a pipeline, or write a note.
      </p>
      <ul v-else class="m-0 flex list-none flex-col gap-1 p-0" role="list" aria-label="Recent activity">
        <li v-for="row in rows" :key="row.id" role="listitem">
          <Button
            type="button"
            variant="ghost"
            class="flex h-auto w-full items-center justify-start gap-3 rounded-[8px] px-3 py-2 text-left font-normal hover:bg-bg-surface-hover"
            @click="openRow(row)"
          >
            <MessageSquare v-if="row.kind === 'session'" class="size-4 shrink-0 text-brand" :stroke-width="1.5" />
            <GitBranch v-else-if="row.kind === 'pipeline'" class="size-4 shrink-0 text-success" :stroke-width="1.5" />
            <FileText v-else class="size-4 shrink-0 text-thinking" :stroke-width="1.5" />
            <span class="min-w-0 flex-1">
              <span class="block truncate text-sm font-medium text-text-primary">{{ row.title }}</span>
              <span class="text-xs text-text-tertiary">{{ kindLabel(row.kind) }} · {{ relativeTime(row.timestamp) }}</span>
            </span>
          </Button>
        </li>
      </ul>
    </CardContent>
  </Card>
</template>
