<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue';
import { storeToRefs } from 'pinia';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useWorkflowStore } from '../stores/workflow';

const workflowStore = useWorkflowStore();
const {
  activeSessions,
  selectedSession,
  selectedSessionId,
  loading,
  error,
  lastUpdated,
} = storeToRefs(workflowStore);

const props = defineProps<{
  filterWorkflowId: string | null;
}>();

const filteredSessions = computed(() => {
  if (!props.filterWorkflowId) return activeSessions.value;
  return activeSessions.value.filter((session) => session.workflow_id === props.filterWorkflowId);
});

const visibleSelectedSession = computed(() => {
  if (!props.filterWorkflowId) return selectedSession.value;
  return selectedSession.value?.workflow_id === props.filterWorkflowId ? selectedSession.value : null;
});

onMounted(() => {
  void workflowStore.mount();
});

onBeforeUnmount(() => {
  workflowStore.unmount();
});
</script>

<template>
  <section data-testid="workflow-view" class="flex min-h-0 gap-3 overflow-hidden p-3">
    <Card class="flex w-80 min-w-80 flex-col gap-2 rounded-md py-0 shadow-none ring-0 border border-border bg-card">
      <CardContent class="flex min-h-0 flex-1 flex-col gap-2 p-2">
        <header class="flex items-center justify-between gap-2">
          <h2 class="text-sm font-semibold text-foreground">Workflow sessions</h2>
          <Button size="sm" variant="outline" @click="workflowStore.refreshSessions">Refresh</Button>
        </header>

        <p v-if="error" class="text-xs text-destructive">{{ error }}</p>
        <p v-else-if="loading" class="text-xs text-muted-foreground">Loading sessions…</p>
        <p v-else-if="filteredSessions.length === 0" class="text-xs text-muted-foreground">No sessions available.</p>

        <ScrollArea class="min-h-0 flex-1">
          <Button
            v-for="session in filteredSessions"
            :key="session.id"
            type="button"
            data-testid="workflow-session-row"
            class="mb-1 flex w-full flex-col rounded-sm border border-border px-2 py-1 text-left hover:bg-muted"
            :class="selectedSessionId === session.id ? 'bg-muted' : ''"
            @click="workflowStore.selectSession(session.id)"
          >
            <span class="text-xs font-medium text-foreground">{{ session.workflow_id ?? session.id }}</span>
            <span class="text-[11px] text-muted-foreground">
              {{ session.current_step ?? 0 }}/{{ session.total_steps ?? 0 }} · {{ session.status }}
            </span>
          </Button>
        </ScrollArea>
      </CardContent>
    </Card>

    <Card class="flex min-h-0 flex-1 flex-col gap-2 rounded-md py-0 shadow-none ring-0 border border-border bg-card">
      <CardContent class="flex min-h-0 flex-1 flex-col gap-2 p-3">
        <h3 class="text-sm font-semibold text-foreground">Session detail</h3>
        <p v-if="!visibleSelectedSession" class="text-xs text-muted-foreground">Select a session to view details.</p>
        <template v-else>
          <div class="grid grid-cols-2 gap-2 text-xs text-muted-foreground">
            <span>ID: {{ visibleSelectedSession.id }}</span>
            <span>Status: {{ visibleSelectedSession.status }}</span>
            <span>Workflow: {{ visibleSelectedSession.workflow_id ?? 'n/a' }}</span>
            <span>Updated: {{ visibleSelectedSession.updated_at ?? lastUpdated ?? 'n/a' }}</span>
          </div>
          <ScrollArea class="min-h-0 flex-1 rounded-sm border border-border bg-muted">
            <pre class="whitespace-pre-wrap break-words text-xs text-foreground p-2">{{
              JSON.stringify(visibleSelectedSession.history ?? [], null, 2)
            }}</pre>
          </ScrollArea>
        </template>
      </CardContent>
    </Card>
  </section>
</template>
