<script setup lang="ts">
import { computed } from 'vue';
import type { Component } from 'vue';
import { RefreshCw, AlertTriangle, Brain, CheckCircle, ChevronDown } from 'lucide-vue-next';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import type { ThinkingSession } from '../lib/workflow-types';
import KnowledgeNoteRenderer from './KnowledgeNoteRenderer.vue';
import ThinkingThoughtItem from './ThinkingThoughtItem.vue';

const props = defineProps<{
  node: ThinkingSession;
}>();

const thoughtCountLabel = computed(
  () => `${props.node.thoughts.length} thought${props.node.thoughts.length === 1 ? '' : 's'}`,
);

const shortSessionId = computed(() =>
  props.node.sessionId.length <= 18
    ? props.node.sessionId
    : `${props.node.sessionId.slice(0, 8)}…${props.node.sessionId.slice(-6)}`,
);

const statusMetaMap: Record<
  ThinkingSession['status'],
  { label: string; icon: Component; className: string; iconClass: string }
> = {
  active: {
    label: 'Active',
    icon: RefreshCw,
    className: 'bg-blue-500/10 text-blue-600 border-blue-500/30 dark:text-blue-400',
    iconClass: 'animate-spin',
  },
  completed: {
    label: 'Completed',
    icon: CheckCircle,
    className: 'bg-emerald-500/10 text-emerald-600 border-emerald-500/30 dark:text-emerald-400',
    iconClass: '',
  },
  abandoned: {
    label: 'Abandoned',
    icon: AlertTriangle,
    className: 'bg-amber-500/10 text-amber-600 border-amber-500/30 dark:text-amber-400',
    iconClass: '',
  },
};

const statusMeta = computed(() => statusMetaMap[props.node.status]);
</script>

<template>
  <Card class="w-[280px] overflow-hidden border-l-4 border-l-primary shadow-md">
    <Collapsible>
      <CollapsibleTrigger
        class="group/trigger w-full text-left transition-colors hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <CardHeader class="flex flex-row items-start justify-between gap-3 p-4">
          <div class="flex min-w-0 items-center gap-2.5">
            <span
              class="inline-flex size-7 shrink-0 items-center justify-center rounded bg-primary/10 text-primary"
              aria-hidden="true"
            >
              <Brain class="size-4" :stroke-width="1.5" />
            </span>
            <div class="flex min-w-0 flex-col">
              <CardTitle class="text-sm font-semibold leading-tight">
                Thinking session
              </CardTitle>
              <span
                class="truncate font-mono text-[10px] leading-tight text-muted-foreground"
                :title="node.sessionId"
              >
                {{ shortSessionId }}
              </span>
            </div>
          </div>

          <div class="flex min-w-0 flex-wrap items-center justify-end gap-1.5">
            <Badge variant="secondary" class="text-[10px]">
              {{ thoughtCountLabel }}
            </Badge>
            <Badge
              variant="outline"
              :class="['text-[10px] font-semibold', statusMeta.className]"
              :title="node.status"
            >
              <component
                :is="statusMeta.icon"
                class="size-3"
                :class="statusMeta.iconClass"
                :stroke-width="1.5"
                aria-hidden="true"
              />
              {{ statusMeta.label }}
            </Badge>
            <ChevronDown
              class="size-4 shrink-0 text-muted-foreground transition-transform duration-200 group-data-[state=open]/trigger:rotate-180"
              :stroke-width="1.5"
              aria-hidden="true"
            />
          </div>
        </CardHeader>
      </CollapsibleTrigger>

      <CollapsibleContent
        class="overflow-hidden data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:animate-in data-[state=open]:fade-in-0"
      >
        <CardContent class="px-4 pb-4 pt-0">
          <ul
            v-if="node.thoughts.length"
            class="relative m-0 flex list-none flex-col gap-3 border-l-2 border-primary/30 pl-3"
          >
            <ThinkingThoughtItem
              v-for="thought in node.thoughts"
              :key="`${thought.number}-${thought.timestamp}`"
              :thought="thought"
            />
          </ul>
          <div
            v-else
            class="py-3 text-xs text-muted-foreground"
          >
            No thoughts recorded for this session.
          </div>

          <section
            v-if="node.conclusion"
            class="mt-4 rounded-md border border-primary/20 bg-primary/5 p-3"
          >
            <div
              class="mb-2 text-[10px] font-semibold uppercase tracking-wider text-primary"
            >
              Conclusion
            </div>
            <KnowledgeNoteRenderer :content="node.conclusion" />
          </section>
        </CardContent>
      </CollapsibleContent>
    </Collapsible>
  </Card>
</template>
