<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { ChevronDown, ChevronUp, Loader2, Ruler } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import type { PlanEntry } from '../lib/types';

const props = defineProps<{
  entries: PlanEntry[];
}>();

const isExpanded = ref(true);

const entryCount = computed(() => props.entries.length);

const completedCount = computed(
  () => props.entries.filter((entry) => entry.status === 'completed').length
);

const statusLabelMap: Record<PlanEntry['status'], string> = {
  pending: 'Pending',
  in_progress: 'In Progress',
  completed: 'Completed',
};

const priorityVariantMap: Record<
  PlanEntry['priority'],
  'destructive' | 'secondary' | 'outline'
> = {
  high: 'destructive',
  medium: 'secondary',
  low: 'outline',
};

watch(
  () => props.entries,
  () => {
    isExpanded.value = true;
  },
  { deep: true }
);

function toggleExpanded(): void {
  isExpanded.value = !isExpanded.value;
}
</script>

<template>
  <Card class="mb-4 overflow-hidden">
    <CardHeader class="p-0">
      <Button
        type="button"
        variant="ghost"
        class="flex h-auto w-full items-center justify-between gap-3 rounded-none px-4 py-3.5 text-left"
        :aria-expanded="isExpanded"
        @click="toggleExpanded"
      >
        <div class="flex min-w-0 items-center gap-2.5">
          <Ruler class="size-4 shrink-0" :stroke-width="1.5" />
          <span class="font-semibold">Agent Plan</span>
          <span class="text-xs text-muted-foreground">
            {{ completedCount }}/{{ entryCount }} completed
          </span>
        </div>
        <component
          :is="isExpanded ? ChevronUp : ChevronDown"
          class="size-4 text-muted-foreground"
          :stroke-width="1.5"
        />
      </Button>
    </CardHeader>

    <CardContent v-if="isExpanded" class="border-t px-4 py-3">
      <ol class="m-0 flex list-none flex-col gap-3 p-0">
        <li
          v-for="(entry, index) in entries"
          :key="`${index}-${entry.content}`"
          class="flex items-start gap-3"
        >
          <div class="mt-0.5 flex size-6 shrink-0 items-center justify-center">
            <Loader2
              v-if="entry.status === 'in_progress'"
              class="size-4 animate-spin text-primary"
              :stroke-width="2"
              :aria-label="statusLabelMap[entry.status]"
            />
            <Checkbox
              v-else
              :model-value="entry.status === 'completed'"
              :disabled="true"
              :aria-label="statusLabelMap[entry.status]"
            />
          </div>
          <div class="min-w-0 flex-1">
            <div class="mb-1 flex items-center gap-2">
              <Badge
                :variant="priorityVariantMap[entry.priority]"
                :class="entry.status === 'in_progress' ? 'animate-pulse' : ''"
              >
                {{ entry.priority }}
              </Badge>
              <span class="text-xs text-muted-foreground">
                {{ statusLabelMap[entry.status] }}
              </span>
            </div>
            <p
              class="m-0 whitespace-pre-wrap leading-snug text-foreground"
              :class="entry.status === 'completed' ? 'text-muted-foreground line-through' : ''"
            >
              {{ entry.content }}
            </p>
          </div>
        </li>
      </ol>
    </CardContent>
  </Card>
</template>
