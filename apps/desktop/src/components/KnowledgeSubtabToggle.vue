<script setup lang="ts">
import { TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useKnowledgeStore, type KnowledgeSubtab } from '../stores/knowledge';

const store = useKnowledgeStore();

const SUBTABS: { value: KnowledgeSubtab; label: string }[] = [
  { value: 'memory', label: 'Memory' },
  { value: 'graph', label: 'Graph' },
];
</script>

<template>
  <TabsList variant="line" class="h-10 gap-1 bg-transparent p-0">
    <TabsTrigger
      v-for="tab in SUBTABS"
      :key="tab.value"
      :value="tab.value"
      class="h-10 rounded-none px-3 text-text-secondary hover:text-text-primary data-[state=active]:text-brand! data-[state=active]:hover:text-brand!"
      @click="store.setActiveSubtab(tab.value)"
    >
      {{ tab.label }}
    </TabsTrigger>
  </TabsList>
</template>
