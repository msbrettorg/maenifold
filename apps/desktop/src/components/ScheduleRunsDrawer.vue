<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { ArrowDown, ArrowUp, ArrowUpDown, Loader2, RotateCcw, Trash2 } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle, AlertDialogTrigger } from '@/components/ui/alert-dialog';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { useMcpClient } from '@/composables/useMcpClient';
import { useMcpResources } from '@/composables/useMcpResources';

interface RawRun { fileName?: string; content?: string; timestamp?: string; status?: string; duration_seconds?: number; durationSeconds?: number; stdout?: string; stderr?: string; error?: string; mcp_action?: string | null; mcp_parameters?: unknown; mcp_result?: unknown; }
interface ScheduleRunRow { id: string; timestamp: string; outcome: 'success' | 'failed' | 'cancelled' | 'running' | 'unknown'; duration: string; stdout: string; stderr: string; mcpAction?: string; mcpParameters?: unknown; mcpResult?: unknown; }
type SortColumn = 'timestamp' | 'outcome' | 'duration';

const props = defineProps<{ scheduleId: string; open: boolean; onUpdateOpen?: (value: boolean) => void; }>();
const emit = defineEmits<{ 'update:open': [open: boolean]; }>();

const resources = useMcpResources();
const { callTool, connect, connected } = useMcpClient();
const runs = ref<ScheduleRunRow[]>([]);
const expandedRunId = ref('');
const loading = ref(false);
const clearing = ref(false);
const retryingRunId = ref('');
const error = ref('');
const clearError = ref('');
const sortColumn = ref<SortColumn | null>(null);
const sortDirection = ref<'asc' | 'desc' | null>(null);
let unsubscribe: (() => void) | null = null;
let generation = 0;

const encodedScheduleId = computed(() => encodeURIComponent(props.scheduleId));
const resourceUri = computed(() => `schedule://logs/${encodedScheduleId.value}`);

function updateOpen(value: boolean): void { props.onUpdateOpen?.(value); emit('update:open', value); }

function timestampFromFileName(fileName?: string): string {
  const match = fileName?.match(/^(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})Z/);
  if (!match) return fileName ?? 'Unknown';
  const [, year, month, day, hour, minute, second] = match;
  return `${year}-${month}-${day}T${hour}:${minute}:${second}Z`;
}

function formatTimestamp(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(date);
}

function normalizeRun(raw: RawRun, index: number): ScheduleRunRow {
  const content = raw.content ?? '';
  const status = raw.status?.toLowerCase();
  const legacyFailed = /exit code|timed out|error/i.test(content);
  const outcome: ScheduleRunRow['outcome'] =
    status === 'failed' || status === 'timeout'
      ? 'failed'
      : status === 'success'
        ? 'success'
        : status === 'cancelled'
          ? 'cancelled'
          : status === 'in-progress' || status === 'running'
            ? 'running'
            : status === 'skipped' || status === undefined
              ? legacyFailed ? 'failed' : 'unknown'
              : 'unknown';
  const seconds = raw.duration_seconds ?? raw.durationSeconds;
  return {
    id: raw.fileName ?? `${raw.timestamp ?? 'run'}-${index}`,
    timestamp: raw.timestamp ?? timestampFromFileName(raw.fileName),
    outcome,
    duration: typeof seconds === 'number' ? `${seconds}s` : '—',
    stdout: raw.stdout ?? content,
    stderr: raw.stderr ?? raw.error ?? '',
    ...(raw.mcp_action != null ? { mcpAction: raw.mcp_action } : {}),
    ...(raw.mcp_parameters != null ? { mcpParameters: raw.mcp_parameters } : {}),
    ...('mcp_result' in raw ? { mcpResult: raw.mcp_result } : {}),
  };
}
function handleSort(column: SortColumn): void {
  if (sortColumn.value !== column) { sortColumn.value = column; sortDirection.value = 'asc'; return; }
  if (sortDirection.value === 'asc') { sortDirection.value = 'desc'; return; }
  sortColumn.value = null; sortDirection.value = null;
}

function sortAriaOrder(column: SortColumn): 'ascending' | 'descending' | 'none' {
  return sortColumn.value === column ? sortDirection.value === 'asc' ? 'ascending' : 'descending' : 'none';
}

function sortIcon(column: SortColumn) {
  return sortColumn.value !== column ? ArrowUpDown : sortDirection.value === 'asc' ? ArrowUp : ArrowDown;
}

const sortedRuns = computed(() => {
  if (!sortColumn.value || !sortDirection.value) return runs.value;
  return [...runs.value].sort((a, b) => {
    const value = (run: ScheduleRunRow) => sortColumn.value === 'duration'
      ? (run.duration === '—' ? Infinity : Number.parseFloat(run.duration.replace(/s$/, '')))
      : run[sortColumn.value ?? 'timestamp'];
    const left = value(a);
    const right = value(b);
    const result = typeof left === 'number' && typeof right === 'number' ? left - right : String(left).localeCompare(String(right));
    return sortDirection.value === 'asc' ? result : -result;
  });
});

async function loadRuns(): Promise<void> {
  if (!props.open || !props.scheduleId) return;
  loading.value = true;
  error.value = '';
  try {
    const resource = await resources.readResource(resourceUri.value);
    const parsed = resource.text ? JSON.parse(resource.text) as RawRun[] : [];
    runs.value = Array.isArray(parsed) ? parsed.map(normalizeRun) : [];
  } catch (loadError) {
    error.value = loadError instanceof Error ? loadError.message : String(loadError);
  } finally {
    loading.value = false;
  }
}

async function resubscribe(): Promise<void> {
  unsubscribe?.();
  unsubscribe = null;
  const myGen = ++generation;
  const capturedUri = resourceUri.value;
  if (!props.open || !props.scheduleId) return;
  await loadRuns();
  if (myGen !== generation || !props.open || resourceUri.value !== capturedUri) return;
  try {
    unsubscribe = await resources.subscribeResource(capturedUri, () => {
      void loadRuns();
    });
  } catch {
    unsubscribe = null;
  }
}

async function retryRun(run: ScheduleRunRow): Promise<void> {
  if (retryingRunId.value) return;
  retryingRunId.value = run.id;
  error.value = '';
  try {
    if (!connected.value) await connect();
    await callTool('Schedule', { action: 'run_now', id: props.scheduleId });
    await loadRuns();
  } catch (retryError) {
    error.value = retryError instanceof Error ? retryError.message : String(retryError);
  } finally {
    retryingRunId.value = '';
  }
}

async function confirmClearHistory(): Promise<void> {
  if (clearing.value) return;
  clearing.value = true;
  clearError.value = '';
  try {
    if (!connected.value) await connect();
    await callTool('Schedule', { action: 'clear_history', id: props.scheduleId });
    await loadRuns();
  } catch (clearHistoryError) {
    clearError.value = clearHistoryError instanceof Error ? clearHistoryError.message : String(clearHistoryError);
  } finally {
    clearing.value = false;
  }
}

watch(() => [props.open, props.scheduleId], () => { void resubscribe(); }, { immediate: true });
onBeforeUnmount(() => { generation += 1; unsubscribe?.(); });
</script>

<template>
  <Sheet :open="open" @update:open="updateOpen">
    <SheetContent class="flex w-full flex-col gap-4 sm:max-w-2xl" side="right">
      <SheetHeader>
        <SheetTitle>Schedule runs</SheetTitle>
        <SheetDescription>Recent runs and logs for {{ scheduleId }}.</SheetDescription>
        <AlertDialog v-if="runs.length > 0">
          <AlertDialogTrigger as-child>
            <Button type="button" variant="destructive" size="sm" class="w-fit" :disabled="clearing">
              <Loader2 v-if="clearing" class="h-3.5 w-3.5 animate-spin" />
              <Trash2 v-else class="h-3.5 w-3.5" />
              Clear history
            </Button>
          </AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Clear run history?</AlertDialogTitle>
              <AlertDialogDescription>
                Delete all run logs for {{ scheduleId }}? This cannot be undone.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction variant="destructive" :disabled="clearing" @click="confirmClearHistory">
                Clear history
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </SheetHeader>

      <div v-if="clearError" class="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
        {{ clearError }}
      </div>

      <div v-if="error" class="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
        {{ error }}
      </div>

      <div v-if="loading" class="flex items-center gap-2 rounded-md border bg-muted/40 px-4 py-6 text-sm text-muted-foreground">
        <Loader2 class="h-4 w-4 animate-spin" />
        Loading runs…
      </div>

      <div v-else-if="runs.length === 0" class="rounded-md border bg-muted/30 px-4 py-8 text-center text-sm text-muted-foreground">
        No runs recorded yet.
      </div>

      <ScrollArea v-else class="min-h-0 flex-1 rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead :aria-sort="sortAriaOrder('timestamp')"><Button variant="ghost" size="sm" @click="handleSort('timestamp')">Timestamp <component :is="sortIcon('timestamp')" class="h-3.5 w-3.5" /></Button></TableHead>
              <TableHead :aria-sort="sortAriaOrder('outcome')"><Button variant="ghost" size="sm" @click="handleSort('outcome')">Outcome <component :is="sortIcon('outcome')" class="h-3.5 w-3.5" /></Button></TableHead>
              <TableHead :aria-sort="sortAriaOrder('duration')"><Button variant="ghost" size="sm" @click="handleSort('duration')">Duration <component :is="sortIcon('duration')" class="h-3.5 w-3.5" /></Button></TableHead>
              <TableHead class="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <template v-for="run in sortedRuns" :key="run.id">
              <TableRow class="cursor-pointer" @click="expandedRunId = expandedRunId === run.id ? '' : run.id">
                <TableCell>{{ formatTimestamp(run.timestamp) }}</TableCell>
                <TableCell>
                  <Badge
                    :variant="run.outcome === 'failed' ? 'destructive' : run.outcome === 'unknown' ? 'outline' : run.outcome === 'success' ? 'default' : 'secondary'"
                    :class="run.outcome === 'running' ? 'animate-pulse' : ''"
                  >
                    {{ run.outcome }}
                  </Badge>
                </TableCell>
                <TableCell>{{ run.duration }}</TableCell>
                <TableCell class="text-right">
                  <Button
                    v-if="run.outcome === 'failed' || run.outcome === 'unknown'"
                    type="button"
                    variant="outline"
                    size="sm"
                    :disabled="retryingRunId === run.id"
                    @click.stop="retryRun(run)"
                  >
                    <Loader2 v-if="retryingRunId === run.id" class="h-3.5 w-3.5 animate-spin" />
                    <RotateCcw v-else class="h-3.5 w-3.5" />
                    Retry now
                  </Button>
                </TableCell>
              </TableRow>
              <TableRow v-if="expandedRunId === run.id">
                <TableCell colspan="4" class="bg-muted/30">
                  <div class="grid gap-3 text-xs">
                    <div v-if="run.mcpAction" class="grid gap-2">
                      <div><span class="font-medium text-text-primary">MCP action: </span><Badge variant="outline">{{ run.mcpAction }}</Badge></div>
                      <div v-if="run.mcpParameters !== undefined">
                        <div class="mb-1 font-medium text-text-primary">Parameters</div>
                        <pre class="max-h-32 overflow-auto rounded-md bg-background p-3 text-muted-foreground">{{ JSON.stringify(run.mcpParameters, null, 2) }}</pre>
                      </div>
                      <div v-if="run.mcpResult !== undefined">
                        <div class="mb-1 font-medium text-text-primary">Result</div>
                        <pre class="max-h-48 overflow-auto rounded-md bg-background p-3 text-muted-foreground">{{ run.mcpResult === null ? 'No structured result' : JSON.stringify(run.mcpResult, null, 2) }}</pre>
                      </div>
                    </div>
                    <div><div class="mb-1 font-medium text-text-primary">stdout</div>
                      <pre class="max-h-48 overflow-auto rounded-md bg-background p-3 text-muted-foreground">{{ run.stdout || 'No stdout captured.' }}</pre>
                    </div>
                    <div v-if="run.stderr"><div class="mb-1 font-medium text-text-primary">stderr</div>
                      <pre class="max-h-48 overflow-auto rounded-md bg-background p-3 text-destructive">{{ run.stderr }}</pre>
                    </div>
                  </div>
                </TableCell>
              </TableRow>
            </template>
          </TableBody>
        </Table>
      </ScrollArea>
    </SheetContent>
  </Sheet>
</template>
