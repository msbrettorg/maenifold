<script setup lang="ts">
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

const props = defineProps<{
  searchText: string;
  selectedProcess: string;
  selectedState: string;
  selectedLocation: string;
  issuesOnly: boolean;
  processOptions: string[];
  stateOptions: string[];
  locationOptions: string[];
}>();

const emit = defineEmits<{
  'update:issuesOnly': [value: boolean];
  'update:searchText': [value: string];
  'update:selectedLocation': [value: string];
  'update:selectedProcess': [value: string];
  'update:selectedState': [value: string];
}>();
</script>

<template>
  <div
    class="grid gap-3 px-4 py-[0.9rem] bg-[var(--surface)] rounded grid-cols-[minmax(14rem,1.5fr)_repeat(3,minmax(10rem,1fr))_auto] max-[1100px]:grid-cols-1"
    role="search"
    aria-label="Task filters"
  >
    <Input
      class="border border-[var(--color-border-card)] bg-[var(--surface-container)] transition-[background,border-color,box-shadow] duration-150 ease-[ease] focus:border-[color-mix(in_srgb,var(--color-brand)_40%,transparent)] focus:bg-[var(--surface-high)] focus:shadow-[0_0_0_2px_color-mix(in_srgb,var(--color-brand)_15%,transparent)]"
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

    <Select :model-value="props.selectedLocation" @update:model-value="(v) => emit('update:selectedLocation', String(v))">
      <SelectTrigger>
        <SelectValue placeholder="All locations" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="all">All locations</SelectItem>
        <SelectItem v-for="value in props.locationOptions" :key="value" :value="value">{{ value }}</SelectItem>
      </SelectContent>
    </Select>

    <div
      class="inline-flex items-center gap-2 min-h-10 px-3 whitespace-nowrap text-[length:var(--text-caption)] text-[var(--color-text-primary)] bg-[var(--surface-container)] border border-[var(--color-border-subtle)] rounded [&_input]:accent-[var(--color-brand)]"
    >
      <Checkbox :checked="props.issuesOnly" @update:checked="(v: boolean | 'indeterminate') => emit('update:issuesOnly', v === true)" />
      <Label>Issues only</Label>
    </div>
  </div>
</template>
