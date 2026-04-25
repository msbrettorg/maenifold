<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useSessionBrowserStore } from '@/stores/session-browser';
import type { AgentSessionSummary } from '@/stores/agent';

const props = defineProps<{ open: boolean }>();

const emit = defineEmits<{
  (e: 'cancel'): void;
  (e: 'add', value: { agent: string; sessionId: string }): void;
}>();

const sessionBrowserStore = useSessionBrowserStore();
const { sessionsByAgent, isLoading, error } = storeToRefs(sessionBrowserStore);
const { refreshSessions } = sessionBrowserStore;

const selectedKey = ref('');

const groupedSessions = computed(() =>
  Object.entries(sessionsByAgent.value)
    .map(([agent, sessions]) => ({
      agent,
      sessions: [...sessions].sort((left, right) => sortByNewest(right) - sortByNewest(left)),
    }))
    .sort((left, right) => left.agent.localeCompare(right.agent)),
);

const selectedSession = computed(() => {
  for (const group of groupedSessions.value) {
    const session = group.sessions.find((entry) => sessionKey(group.agent, entry.sessionId) === selectedKey.value);
    if (session) {
      return { agent: group.agent, session };
    }
  }
  return null;
});

watch(
  () => props.open,
  (open) => {
    if (!open) {
      selectedKey.value = '';
      return;
    }
    selectedKey.value = '';
    void refreshSessions();
  },
  { immediate: true },
);

function sessionKey(agent: string, sessionId: string): string {
  return `${agent}::${sessionId}`;
}

function sortByNewest(session: AgentSessionSummary): number {
  const value = session.createdAt ?? session.updatedAt ?? session.lastActivity;
  return value ? Date.parse(value) || 0 : 0;
}

function shortSessionId(sessionId: string): string {
  return sessionId.slice(0, 12);
}

function formatStatus(status: string): string {
  return status.trim().replace(/[_-]+/g, ' ').replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function isResumableSession(session: AgentSessionSummary): boolean {
  return session.status.trim().toLowerCase() === 'resumable';
}

function handleAdd(): void {
  if (!selectedSession.value) return;
  emit('add', {
    agent: selectedSession.value.agent,
    sessionId: selectedSession.value.session.sessionId,
  });
}
</script>

<template>
  <Dialog :open="open" @update:open="(nextOpen) => !nextOpen && emit('cancel')">
    <DialogContent class="gap-4 p-0 sm:max-w-[36rem]" :show-close-button="false">
      <DialogHeader class="px-6 pt-6">
        <DialogTitle>Add session node</DialogTitle>
        <p class="m-0 text-xs text-text-secondary">
          Pick an ACP session to pre-fill a prompt node with its session reference.
        </p>
      </DialogHeader>

      <div class="px-6">
        <div
          v-if="error"
          class="rounded-2xl bg-destructive/10 px-3 py-2 text-xs text-destructive"
          role="alert"
        >
          {{ error }}
        </div>

        <div
          v-else-if="!isLoading && groupedSessions.length === 0"
          class="rounded-2xl bg-muted/40 px-4 py-8 text-center text-sm text-muted-foreground"
        >
          No sessions available
        </div>

        <ScrollArea v-else class="h-[20rem] pr-3">
          <div class="flex flex-col gap-3">
            <section v-for="group in groupedSessions" :key="group.agent" class="space-y-2">
              <header class="text-xs font-semibold uppercase tracking-wide text-text-secondary">
                {{ group.agent }}
              </header>
              <Button
                v-for="session in group.sessions"
                :key="session.sessionId"
                variant="ghost"
                type="button"
                class="flex h-auto w-full min-w-0 flex-col items-start gap-2 overflow-hidden rounded-2xl border px-3 py-3 text-left"
                :class="selectedKey === sessionKey(group.agent, session.sessionId)
                  ? 'border-primary bg-accent'
                  : 'border-border bg-background hover:bg-muted/50'"
                @pointerdown.stop
                @click.stop="selectedKey = sessionKey(group.agent, session.sessionId)"
              >
                <div class="flex w-full min-w-0 flex-wrap items-center gap-2">
                  <span class="min-w-0 break-all text-sm font-medium text-foreground">{{ group.agent }}</span>
                  <code class="font-mono text-xs text-muted-foreground">{{ shortSessionId(session.sessionId) }}</code>
                  <Badge variant="outline">{{ formatStatus(session.status) }}</Badge>
                  <Badge v-if="isResumableSession(session)" variant="outline" class="bg-success/15 text-success">
                    Resumable
                  </Badge>
                </div>
                <p v-if="session.title" class="w-full min-w-0 break-words text-xs text-muted-foreground">
                  {{ session.title }}
                </p>
              </Button>
            </section>
          </div>
        </ScrollArea>
      </div>

      <DialogFooter class="gap-2 px-6 pb-6">
        <Button type="button" variant="ghost" @click="emit('cancel')">Cancel</Button>
        <Button type="button" :disabled="!selectedSession" @click="handleAdd">Add</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
