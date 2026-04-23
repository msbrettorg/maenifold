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
  <div class="gate-filters" role="search" aria-label="Task filters">
    <Input
      class="gate-filters__search"
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

    <div class="gate-toggle">
      <Checkbox :checked="props.issuesOnly" @update:checked="(v: boolean | 'indeterminate') => emit('update:issuesOnly', v === true)" />
      <Label>Issues only</Label>
    </div>
  </div>
</template>

<style scoped>
.gate-filters {
  display: grid;
  grid-template-columns: minmax(14rem, 1.5fr) repeat(3, minmax(10rem, 1fr)) auto;
  gap: 0.75rem;
  padding: 0.9rem 1rem;
  background: var(--surface);
  border-radius: 0.25rem;
}

.gate-filters :deep(.gate-filters__search) {
  border: 1px solid var(--color-border-card);
  background: var(--surface-container);
  transition: background 160ms ease, border-color 160ms ease, box-shadow 160ms ease;
}

.gate-filters :deep(.gate-filters__search:focus) {
  border-color: color-mix(in srgb, var(--color-brand) 40%, transparent);
  background: var(--surface-high);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-brand) 15%, transparent); /* audit-ok: focus-ring */
}

.gate-toggle {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  min-height: 2.5rem;
  padding: 0 0.75rem;
  white-space: nowrap;
  font-size: var(--text-caption);
  color: var(--color-text-primary);
  background: var(--surface-container);
  border: 1px solid var(--color-border-subtle);
  border-radius: 0.25rem;
}

.gate-toggle input {
  accent-color: var(--color-brand);
}

@media (max-width: 1100px) {
  .gate-filters {
    grid-template-columns: 1fr;
  }
}
</style>
