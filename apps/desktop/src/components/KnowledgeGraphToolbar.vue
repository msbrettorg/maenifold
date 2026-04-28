<script setup lang="ts">
import { Network, RefreshCw, RotateCcw, Search } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { CommunityNode } from '../lib/graph-types';

defineProps<{
  title: string;
  nodeCount: number;
  filter: string;
  selectedCommunity: string;
  communities: CommunityNode[];
  isLoading: boolean;
}>();

const emit = defineEmits<{
  'update:filter': [value: string | number];
  reset: [];
  reload: [];
  selectCommunity: [value: string | number | null];
}>();

function selectCommunity(value: unknown): void {
  if (typeof value === 'string' || typeof value === 'number' || value === null) {
    emit('selectCommunity', value);
  }
}
</script>

<template>
  <div class="flex flex-wrap items-center gap-2 border-b border-border-subtle px-3 py-2">
    <Network class="size-4 text-text-secondary" aria-hidden="true" />
    <p class="min-w-36 truncate text-sm font-semibold text-text-primary">{{ title }}</p>
    <Badge variant="secondary">{{ nodeCount }} nodes</Badge>

    <div class="relative min-w-48 flex-1">
      <Search class="pointer-events-none absolute top-1/2 left-2 size-4 -translate-y-1/2 text-text-tertiary" aria-hidden="true" />
      <Input
        :model-value="filter"
        class="h-8 border-border-subtle bg-bg-surface pl-8 text-sm text-text-primary placeholder:text-text-secondary"
        placeholder="Filter graph"
        @update:model-value="emit('update:filter', $event)"
      />
    </div>

    <Select
      :model-value="selectedCommunity"
      :disabled="isLoading || communities.length === 0"
      @update:model-value="selectCommunity"
    >
      <SelectTrigger class="h-8 w-48 border-border-subtle bg-bg-surface text-text-secondary">
        <SelectValue placeholder="Community cluster" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="all">All communities</SelectItem>
        <SelectItem
          v-for="community in communities"
          :key="community.id"
          :value="String(community.id)"
        >
          {{ community.anchor }}
        </SelectItem>
      </SelectContent>
    </Select>

    <Button type="button" size="sm" variant="ghost" class="h-8 text-text-secondary hover:text-text-primary" @click="emit('reset')">
      <RotateCcw class="size-4" aria-hidden="true" />
      Reset
    </Button>
    <Button type="button" size="sm" variant="ghost" class="h-8 text-text-secondary hover:text-text-primary" @click="emit('reload')">
      <RefreshCw class="size-4" aria-hidden="true" />
      Reload
    </Button>
  </div>
</template>
