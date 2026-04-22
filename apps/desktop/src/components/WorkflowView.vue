<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue';
import { storeToRefs } from 'pinia';
import { Button } from '@/components/ui/button';
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

onMounted(() => {
  void workflowStore.mount();
});

onBeforeUnmount(() => {
  workflowStore.unmount();
});
</script>

<template>
  <section class="flex min-h-0 flex-1 gap-3 overflow-hidden">
    <aside class="flex w-80 min-w-80 flex-col gap-2 rounded-md border border-border bg-card p-2">
      <header class="flex items-center justify-between gap-2">
        <h2 class="text-sm font-semibold text-foreground">Workflow sessions</h2>
        <Button size="sm" variant="outline" @click="workflowStore.refreshSessions">Refresh</Button>
      </header>

      <p v-if="error" class="text-xs text-destructive">{{ error }}</p>
      <p v-else-if="loading" class="text-xs text-muted-foreground">Loading sessions…</p>
      <p v-else-if="activeSessions.length === 0" class="text-xs text-muted-foreground">No sessions available.</p>

      <div class="min-h-0 flex-1 overflow-auto">
        <Button
          v-for="session in activeSessions"
          :key="session.id"
          type="button"
          class="mb-1 flex w-full flex-col rounded-sm border border-border px-2 py-1 text-left hover:bg-muted"
          :class="selectedSessionId === session.id ? 'bg-muted' : ''"
          @click="workflowStore.selectSession(session.id)"
        >
          <span class="text-xs font-medium text-foreground">{{ session.workflow_id ?? session.id }}</span>
          <span class="text-[11px] text-muted-foreground">
            {{ session.current_step ?? 0 }}/{{ session.total_steps ?? 0 }} · {{ session.status }}
          </span>
        </Button>
      </div>
    </aside>

    <article class="flex min-h-0 flex-1 flex-col gap-2 rounded-md border border-border bg-card p-3">
      <h3 class="text-sm font-semibold text-foreground">Session detail</h3>
      <p v-if="!selectedSession" class="text-xs text-muted-foreground">Select a session to view details.</p>
      <template v-else>
        <div class="grid grid-cols-2 gap-2 text-xs text-muted-foreground">
          <span>ID: {{ selectedSession.id }}</span>
          <span>Status: {{ selectedSession.status }}</span>
          <span>Workflow: {{ selectedSession.workflow_id ?? 'n/a' }}</span>
          <span>Updated: {{ selectedSession.updated_at ?? lastUpdated ?? 'n/a' }}</span>
        </div>
        <div class="min-h-0 flex-1 overflow-auto rounded-sm border border-border bg-muted p-2">
          <pre class="whitespace-pre-wrap break-words text-xs text-foreground">{{
            JSON.stringify(selectedSession.history ?? [], null, 2)
          }}</pre>
        </div>
      </template>
    </article>
  </section>
</template>
