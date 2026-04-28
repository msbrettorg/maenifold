<script setup lang="ts">
import { computed, ref } from 'vue';
import { Search } from 'lucide-vue-next';
import { storeToRefs } from 'pinia';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { useKnowledgeStore } from '../stores/knowledge';

const store = useKnowledgeStore();
const { graphLoading, communityGraph, activeCommunityId } = storeToRefs(store);
const communityFilter = ref('');

const communities = computed(() => communityGraph.value?.nodes ?? []);
const filteredCommunities = computed(() => {
  const query = communityFilter.value.trim().toLowerCase();
  if (!query) return communities.value;

  return communities.value.filter((community) => (
    community.anchor.toLowerCase().includes(query)
      || `community ${community.id}`.includes(query)
  ));
});

function selectCommunity(communityId: number): void {
  void store.focusCommunity(communityId);
}
</script>

<template>
  <aside class="flex h-full min-h-0 w-full flex-col border-r border-border-subtle bg-bg-sidebar" aria-label="Graph communities">
    <div class="border-b border-border-subtle px-3 py-3">
      <p class="text-sm font-semibold text-text-primary">Communities</p>
      <p class="mt-1 text-xs text-text-secondary">
        Focus the graph by cluster.
      </p>
      <div class="relative mt-3">
        <Search class="pointer-events-none absolute top-1/2 left-2 size-4 -translate-y-1/2 text-text-tertiary" aria-hidden="true" />
        <Input
          v-model="communityFilter"
          class="h-8 border-border-subtle bg-bg-surface pl-8 text-sm text-text-primary placeholder:text-text-secondary"
          placeholder="Filter communities"
        />
      </div>
    </div>

    <ScrollArea class="min-h-0 flex-1 px-2 py-2">
      <div class="grid gap-1">
        <template v-if="graphLoading && communities.length === 0">
          <Skeleton class="h-10 rounded-lg" />
          <Skeleton class="h-10 rounded-lg" />
          <Skeleton class="h-10 rounded-lg" />
        </template>

        <template v-else>
          <Button
            v-for="community in filteredCommunities"
            :key="community.id"
            type="button"
            variant="ghost"
            class="h-auto min-h-10 justify-start gap-2 rounded-lg px-2 py-2 text-left"
            :class="activeCommunityId === community.id ? 'bg-bg-surface text-text-primary' : 'text-text-secondary hover:bg-bg-surface-hover hover:text-text-primary'"
            @click="selectCommunity(community.id)"
          >
            <span class="min-w-0 flex-1">
              <span class="block truncate text-sm">{{ community.anchor }}</span>
              <span class="block text-xs text-text-tertiary">Community {{ community.id }}</span>
            </span>
            <span class="ml-auto text-xs text-text-tertiary">{{ community.size }}</span>
          </Button>

          <p v-if="filteredCommunities.length === 0" class="px-2 py-3 text-sm text-text-secondary">
            No communities match this filter.
          </p>
        </template>
      </div>
    </ScrollArea>
  </aside>
</template>
