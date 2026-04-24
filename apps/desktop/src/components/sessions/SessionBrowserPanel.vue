<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { storeToRefs } from 'pinia';
import { AlertCircle, Clock3, RefreshCw } from 'lucide-vue-next';
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { useSessionBrowserStore } from '@/stores/session-browser';
import type { AgentSessionSummary } from '@/stores/agent';

const sessionBrowserStore = useSessionBrowserStore();
const { sessionsByAgent, isLoading, error } = storeToRefs(sessionBrowserStore);
const { refreshSessions } = sessionBrowserStore;

interface AgentSessionGroup {
  agent: string;
  sessions: AgentSessionSummary[];
}

const groupedSessions = computed<AgentSessionGroup[]>(() =>
  Object.entries(sessionsByAgent.value)
    .map(([agent, sessions]) => ({
      agent,
      sessions: [...sessions].sort((left, right) => sortByNewest(right) - sortByNewest(left)),
    }))
    .sort((left, right) => left.agent.localeCompare(right.agent)),
);

const openGroups = computed(() => groupedSessions.value.map((group) => group.agent));

onMounted(() => {
  void refreshSessions();
});

function sortByNewest(session: AgentSessionSummary): number {
  const value = session.createdAt ?? session.updatedAt ?? session.lastActivity;
  return value ? Date.parse(value) || 0 : 0;
}

function shortSessionId(sessionId: string): string {
  return sessionId.slice(0, 8);
}

function isResumableSession(session: AgentSessionSummary): boolean {
  return session.status.trim().toLowerCase() === 'resumable';
}

function formatStatus(status: string): string {
  return status
    .trim()
    .replace(/[_-]+/g, ' ')
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function formatRelativeTime(value?: string): string {
  if (!value) {
    return 'Unknown';
  }

  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp)) {
    return 'Unknown';
  }

  const diffSeconds = Math.round((timestamp - Date.now()) / 1000);
  const absSeconds = Math.abs(diffSeconds);
  const formatter = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });

  if (absSeconds < 60) {
    return formatter.format(Math.round(diffSeconds), 'second');
  }

  const diffMinutes = Math.round(diffSeconds / 60);
  if (Math.abs(diffMinutes) < 60) {
    return formatter.format(diffMinutes, 'minute');
  }

  const diffHours = Math.round(diffMinutes / 60);
  if (Math.abs(diffHours) < 24) {
    return formatter.format(diffHours, 'hour');
  }

  const diffDays = Math.round(diffHours / 24);
  return formatter.format(diffDays, 'day');
}

function statusBadgeVariant(status: string): 'default' | 'secondary' | 'destructive' | 'outline' {
  const normalized = status.trim().toLowerCase();

  if (/error|fail|closed|abandoned/.test(normalized)) {
    return 'destructive';
  }

  if (/active|ready|resumable|connected/.test(normalized)) {
    return 'secondary';
  }

  return 'outline';
}

function statusBadgeClass(status: string): string {
  const normalized = status.trim().toLowerCase();

  if (/active|ready|resumable|connected/.test(normalized)) {
    return 'bg-success/15 text-success';
  }

  if (/loading|starting|connecting|initializing|pending/.test(normalized)) {
    return 'bg-warning/15 text-warning';
  }

  return '';
}
</script>

<template>
  <Card size="sm" class="flex min-h-0 flex-1 py-0 shadow-none">
    <CardContent class="flex min-h-0 flex-1 flex-col gap-4 p-4">
      <header class="flex items-start justify-between gap-3">
        <div class="space-y-1">
          <h2 class="text-sm font-semibold text-foreground">
            Session browser
          </h2>
          <p class="text-xs text-muted-foreground">
            Live ACP sessions grouped by agent.
          </p>
        </div>

        <Button
          variant="outline"
          size="sm"
          :disabled="isLoading"
          @click="refreshSessions"
        >
          <RefreshCw class="size-4" :class="isLoading ? 'animate-spin' : ''" />
          Refresh
        </Button>
      </header>

      <div
        v-if="error"
        class="flex items-start gap-2 rounded-3xl bg-destructive/10 px-3 py-2 text-xs text-destructive"
        role="alert"
      >
        <AlertCircle class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
        <span>{{ error }}</span>
      </div>

      <div v-if="isLoading && groupedSessions.length === 0" class="flex flex-col gap-3">
        <div v-for="index in 3" :key="index" class="rounded-3xl bg-muted/30 p-3">
          <div class="flex items-center justify-between gap-3">
            <Skeleton class="h-4 w-28 rounded-full" />
            <Skeleton class="h-4 w-12 rounded-full" />
          </div>
          <div class="mt-3 flex flex-col gap-2">
            <Skeleton class="h-14 w-full rounded-3xl" />
            <Skeleton class="h-14 w-full rounded-3xl" />
          </div>
        </div>
      </div>

      <div
        v-else-if="groupedSessions.length === 0"
        class="flex flex-1 items-center justify-center rounded-3xl bg-muted/30 px-4 py-10 text-center text-sm text-muted-foreground"
      >
        No sessions available
      </div>

      <ScrollArea v-else class="min-h-0 flex-1 pr-3">
        <Accordion type="multiple" :default-value="openGroups" class="w-full">
          <AccordionItem
            v-for="group in groupedSessions"
            :key="group.agent"
            :value="group.agent"
          >
            <AccordionTrigger class="px-3 py-3 hover:no-underline">
              <div class="flex min-w-0 items-center gap-3">
                <div class="min-w-0 text-left">
                  <div class="truncate text-sm font-semibold text-foreground">
                    {{ group.agent }}
                  </div>
                  <div class="text-xs text-muted-foreground">
                    {{ group.sessions.length }} session{{ group.sessions.length === 1 ? '' : 's' }}
                  </div>
                </div>
              </div>
            </AccordionTrigger>

            <AccordionContent class="px-0 pb-0">
              <div class="flex flex-col gap-2 pb-2">
                <div
                  v-for="session in group.sessions"
                  :key="session.sessionId"
                  class="rounded-3xl bg-muted/40 px-3 py-3"
                >
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0 space-y-2">
                      <div class="flex flex-wrap items-center gap-2">
                        <code class="font-mono text-xs font-semibold text-foreground">
                          {{ shortSessionId(session.sessionId) }}
                        </code>
                        <Badge
                          :variant="statusBadgeVariant(session.status)"
                          :class="statusBadgeClass(session.status)"
                        >
                          {{ formatStatus(session.status) }}
                        </Badge>
                        <Badge
                          v-if="isResumableSession(session)"
                          variant="outline"
                          class="bg-success/15 text-success"
                        >
                          Resumable
                        </Badge>
                      </div>

                      <div class="flex items-center gap-1.5 text-xs text-muted-foreground">
                        <Clock3 class="size-3.5 shrink-0" aria-hidden="true" />
                        <span>{{ formatRelativeTime(session.createdAt) }}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      </ScrollArea>
    </CardContent>
  </Card>
</template>
