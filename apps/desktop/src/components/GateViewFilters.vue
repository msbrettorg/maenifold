<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import type { GateReadinessFilter } from '../composables/useGateTaskFiltering';

const readinessOptions: { value: GateReadinessFilter; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'ready', label: 'Ready' },
  { value: 'blocked', label: 'Blocked' },
];

const props = defineProps<{
  searchText: string;
  selectedProcess: string;
  selectedState: string;
  readinessFilter: GateReadinessFilter;
  issuesOnly: boolean;
  processOptions: string[];
  stateOptions: string[];
}>();

const emit = defineEmits<{
  'update:issuesOnly': [value: boolean];
  'update:readinessFilter': [value: GateReadinessFilter];
  'update:searchText': [value: string];
  'update:selectedProcess': [value: string];
  'update:selectedState': [value: string];
}>();

function setReadiness(value: GateReadinessFilter) {
  if (props.readinessFilter === value) return;
  emit('update:readinessFilter', value);
}

function readinessClass(value: GateReadinessFilter) {
  return props.readinessFilter === value
    ? 'bg-brand/15 text-text-primary ring-1 ring-brand/40'
    : 'text-text-secondary';
}
</script>

<template>
  <div
    class="grid gap-3 rounded bg-surface px-4 py-[0.9rem] grid-cols-[minmax(14rem,1.5fr)_repeat(2,minmax(10rem,1fr))_minmax(13rem,auto)_auto] max-[1100px]:grid-cols-1"
    role="search"
    aria-label="Task filters"
  >
    <Input
      class="border border-border-card bg-surface-container transition-[background,border-color,box-shadow] duration-150 ease-[ease] focus:border-brand/40 focus:bg-surface-high focus-visible:ring-2 focus-visible:ring-brand/15"
      :model-value="props.searchText"
      type="search"
      placeholder="Filter tasks"
      @update:model-value="emit('update:searchText', String($event))"
    />

    <Select :model-value="props.selectedProcess" @update:model-value="(v) => emit('update:selectedProcess', String(v))">
      <SelectTrigger>
        <SelectValue placeholder="All processes" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="all">All processes</SelectItem>
        <SelectItem v-for="value in props.processOptions" :key="value" :value="value">{{ value }}</SelectItem>
      </SelectContent>
    </Select>

    <Select :model-value="props.selectedState" @update:model-value="(v) => emit('update:selectedState', String(v))">
      <SelectTrigger>
        <SelectValue placeholder="All states" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="all">All states</SelectItem>
        <SelectItem v-for="value in props.stateOptions" :key="value" :value="value">{{ value }}</SelectItem>
      </SelectContent>
    </Select>

    <div class="inline-flex min-h-10 items-center gap-1 rounded bg-surface-container px-1.5" role="group" aria-label="Filter by readiness">
      <Button
        v-for="option in readinessOptions"
        :key="option.value"
        type="button"
        variant="outline"
        size="sm"
        :class="readinessClass(option.value)"
        :aria-pressed="props.readinessFilter === option.value"
        @click="setReadiness(option.value)"
      >
        {{ option.label }}
      </Button>
    </div>

    <div
      class="inline-flex min-h-10 items-center gap-2 rounded border border-border-subtle bg-surface-container px-3 whitespace-nowrap text-caption text-text-primary [&_input]:accent-brand"
    >
      <Checkbox :checked="props.issuesOnly" @update:checked="(v: boolean | 'indeterminate') => emit('update:issuesOnly', v === true)" />
      <Label>Issues only</Label>
    </div>
  </div>
</template>
