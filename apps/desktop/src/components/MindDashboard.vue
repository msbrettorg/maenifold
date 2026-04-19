<script setup lang="ts">
import { computed, ref } from 'vue';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';

interface CorpusStats {
  concepts: number;
  memories: number;
  edges: number;
}

interface CommunitySummary {
  name: string;
  members: number;
  signal: 'thinking' | 'exploring' | 'remembering';
}

const hasMcpResources = false;

// TODO(T-178.0): replace placeholder refs with useMcpResources-backed reads:
// - graph://stats
// - graph://corpus/snapshot
// - graph://corpus/communities
const stats = ref<CorpusStats>({
  concepts: 1420,
  memories: 386,
  edges: 9210,
});

const communities = ref<CommunitySummary[]>([
  { name: 'runtime-orchestration', members: 122, signal: 'thinking' },
  { name: 'workflow-design', members: 97, signal: 'exploring' },
  { name: 'memory-governance', members: 84, signal: 'remembering' },
  { name: 'agent-protocol', members: 72, signal: 'thinking' },
]);

const conceptQuery = ref('');
const filteredCommunities = computed(() => {
  const normalized = conceptQuery.value.trim().toLowerCase();
  if (!normalized) {
    return communities.value;
  }
  return communities.value.filter((community) => community.name.toLowerCase().includes(normalized));
});

function signalBadgeVariant(signal: CommunitySummary['signal']) {
  if (signal === 'thinking') return 'default';
  if (signal === 'exploring') return 'secondary';
  return 'outline';
}
</script>

<template>
  <section
    class="flex min-h-0 flex-1 flex-col gap-4 overflow-auto bg-[var(--color-neutral-background-2)] p-4 text-[var(--color-neutral-foreground-1)]"
    aria-label="Mind Dashboard"
  >
    <header class="flex items-end justify-between gap-3">
      <div>
        <h1 class="text-xl font-semibold tracking-tight text-[var(--color-neutral-foreground-1)]">Mind Dashboard</h1>
        <p class="text-sm text-[var(--color-neutral-foreground-3)]">
          Corpus state, communities, and concept exploration in one view.
        </p>
      </div>
      <Badge
        variant="outline"
        class="rounded-[4px] border-[var(--color-neutral-stroke-1)] bg-transparent text-[var(--color-neutral-foreground-3)]"
      >
        {{ hasMcpResources ? 'Live resources' : 'Placeholder data' }}
      </Badge>
    </header>

    <div class="grid min-h-0 flex-1 grid-cols-1 gap-4 xl:grid-cols-[1.3fr_1fr_1fr]">
      <Card
        class="rounded-[4px] border-[var(--color-neutral-stroke-1)] bg-[var(--color-neutral-background-3)] py-0 text-[var(--color-neutral-foreground-1)] shadow-none ring-0"
      >
        <CardHeader class="gap-1 border-b border-[var(--color-neutral-stroke-1)] py-3">
          <CardTitle class="text-base">Corpus Stats</CardTitle>
        </CardHeader>
        <CardContent class="grid gap-3 py-3 sm:grid-cols-3">
          <div class="rounded-[4px] border border-[var(--color-neutral-stroke-1)] bg-[var(--color-neutral-background-2)] p-3">
            <p class="text-xs text-[var(--color-neutral-foreground-3)]">Concepts</p>
            <p class="text-2xl font-semibold">{{ stats.concepts.toLocaleString() }}</p>
          </div>
          <div class="rounded-[4px] border border-[var(--color-neutral-stroke-1)] bg-[var(--color-neutral-background-2)] p-3">
            <p class="text-xs text-[var(--color-neutral-foreground-3)]">Memories</p>
            <p class="text-2xl font-semibold">{{ stats.memories.toLocaleString() }}</p>
          </div>
          <div class="rounded-[4px] border border-[var(--color-neutral-stroke-1)] bg-[var(--color-neutral-background-2)] p-3">
            <p class="text-xs text-[var(--color-neutral-foreground-3)]">Edges</p>
            <p class="text-2xl font-semibold">{{ stats.edges.toLocaleString() }}</p>
          </div>
        </CardContent>
      </Card>

      <Card
        class="rounded-[4px] border-[var(--color-neutral-stroke-1)] bg-[var(--color-neutral-background-3)] py-0 text-[var(--color-neutral-foreground-1)] shadow-none ring-0"
      >
        <CardHeader class="gap-1 border-b border-[var(--color-neutral-stroke-1)] py-3">
          <CardTitle class="text-base">Communities</CardTitle>
        </CardHeader>
        <CardContent class="grid gap-2 py-3">
          <div
            v-for="community in filteredCommunities"
            :key="community.name"
            class="flex items-center justify-between rounded-[4px] border border-[var(--color-neutral-stroke-1)] bg-[var(--color-neutral-background-2)] px-3 py-2"
          >
            <div class="min-w-0">
              <p class="truncate text-sm">{{ community.name }}</p>
              <p class="text-xs text-[var(--color-neutral-foreground-3)]">{{ community.members }} concepts</p>
            </div>
            <Badge :variant="signalBadgeVariant(community.signal)" class="rounded-[4px] capitalize">
              {{ community.signal }}
            </Badge>
          </div>
        </CardContent>
      </Card>

      <Card
        class="rounded-[4px] border-[var(--color-neutral-stroke-1)] bg-[var(--color-neutral-background-3)] py-0 text-[var(--color-neutral-foreground-1)] shadow-none ring-0"
      >
        <CardHeader class="gap-1 border-b border-[var(--color-neutral-stroke-1)] py-3">
          <CardTitle class="text-base">Concept Explorer</CardTitle>
        </CardHeader>
        <CardContent class="grid gap-3 py-3">
          <Input
            v-model="conceptQuery"
            placeholder="Filter communities or concept names"
            class="h-9 rounded-[4px] border-[var(--color-neutral-stroke-1)] bg-[var(--color-neutral-background-2)] text-[var(--color-neutral-foreground-1)] placeholder:text-[var(--color-neutral-foreground-3)]"
          />
          <p class="text-xs text-[var(--color-neutral-foreground-3)]">
            Connect this widget to graph://concept/{name} once useMcpResources is available.
          </p>
          <Button
            type="button"
            variant="outline"
            class="h-9 rounded-[4px] border-[var(--color-neutral-stroke-1)] bg-transparent text-[var(--color-neutral-foreground-1)] shadow-none"
          >
            Open graph focus
          </Button>
        </CardContent>
      </Card>
    </div>
  </section>
</template>
