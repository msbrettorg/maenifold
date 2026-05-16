<script setup lang="ts">
import { ChevronDown } from 'lucide-vue-next';
import type { Thought } from '../lib/workflow-types';
import { Badge } from './ui/badge';
import { Card, CardContent } from './ui/card';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from './ui/collapsible';
import MarkdownView from './MarkdownView.vue';

defineProps<{
  thought: Thought;
}>();

function formatRelativeTime(iso: string): string {
  const timestamp = new Date(iso).getTime();
  if (Number.isNaN(timestamp)) return iso;

  const diff = Date.now() - timestamp;
  if (diff < 0) return 'just now';

  const seconds = Math.floor(diff / 1000);
  if (seconds < 60) return `${seconds}s ago`;

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;

  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;

  return new Date(timestamp).toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
  });
}
</script>

<template>
  <li>
    <Collapsible
      :default-open="true"
      class="group/thought relative grid grid-cols-[auto_1fr] items-start gap-2.5"
      role="article"
      :aria-label="`Thought ${thought.number} of ${thought.totalThoughts}`"
    >
      <Badge
        variant="secondary"
        class="mt-0.5 min-w-10 justify-center font-mono text-[10px] tabular-nums"
      >
        {{ thought.number }}/{{ thought.totalThoughts }}
      </Badge>

      <Card class="min-w-0 rounded-md py-0 shadow-none ring-0 border border-border bg-card">
        <CardContent class="px-3 py-2.5">
          <div class="mb-1.5 flex items-center justify-between gap-2">
            <Badge
              v-if="thought.agent"
              variant="outline"
              class="text-[10px] font-normal tracking-wide text-muted-foreground"
            >
              {{ thought.agent }}
            </Badge>
            <span v-else />

            <div class="flex items-center gap-2">
              <time
                class="whitespace-nowrap text-[10px] text-muted-foreground"
                :datetime="thought.timestamp"
                :title="thought.timestamp"
              >
                {{ formatRelativeTime(thought.timestamp) }}
              </time>
              <CollapsibleTrigger
                :aria-label="`Toggle thought ${thought.number}`"
                class="inline-flex h-5 w-5 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              >
                <ChevronDown
                  class="h-3.5 w-3.5 transition-transform duration-200 group-data-[state=closed]/thought:-rotate-90"
                />
              </CollapsibleTrigger>
            </div>
          </div>

          <CollapsibleContent>
            <MarkdownView :content="thought.content" clamp />
          </CollapsibleContent>
        </CardContent>
      </Card>
    </Collapsible>
  </li>
</template>
