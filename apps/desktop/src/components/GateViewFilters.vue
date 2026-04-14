<script setup lang="ts">
import { Input } from '@/components/ui/input';

const selectFieldClass =
  'w-full bg-input text-foreground border border-border rounded-md px-3 py-2 text-caption outline-none focus:border-ring focus:ring-2 focus:ring-ring/20';

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

function emitChecked(event: Event) {
  emit('update:issuesOnly', (event.target as HTMLInputElement).checked);
}

function emitSelect(
  eventName: 'update:selectedProcess' | 'update:selectedState' | 'update:selectedLocation',
  event: Event,
) {
  const value = (event.target as HTMLSelectElement).value;
  switch (eventName) {
    case 'update:selectedProcess':
      emit('update:selectedProcess', value);
      break;
    case 'update:selectedState':
      emit('update:selectedState', value);
      break;
    case 'update:selectedLocation':
      emit('update:selectedLocation', value);
      break;
  }
}
</script>

<template>
  <div class="gate-filters panel" role="search" aria-label="Task filters">
    <Input
      :model-value="props.searchText"
      type="search"
      placeholder="Filter tasks"
      @update:model-value="emit('update:searchText', String($event))"
    />

    <select :class="selectFieldClass" :value="props.selectedProcess" @change="(event) => emitSelect('update:selectedProcess', event)">
      <option value="all">All processes</option>
      <option v-for="value in props.processOptions" :key="value" :value="value">{{ value }}</option>
    </select>

    <select :class="selectFieldClass" :value="props.selectedState" @change="(event) => emitSelect('update:selectedState', event)">
      <option value="all">All states</option>
      <option v-for="value in props.stateOptions" :key="value" :value="value">{{ value }}</option>
    </select>

    <select :class="selectFieldClass" :value="props.selectedLocation" @change="(event) => emitSelect('update:selectedLocation', event)">
      <option value="all">All locations</option>
      <option v-for="value in props.locationOptions" :key="value" :value="value">{{ value }}</option>
    </select>

    <label class="gate-toggle">
      <input :checked="props.issuesOnly" type="checkbox" @change="emitChecked" />
      Issues only
    </label>
  </div>
</template>

<style scoped>
.gate-filters {
  display: grid;
  grid-template-columns: minmax(14rem, 1.5fr) repeat(3, minmax(10rem, 1fr)) auto;
  gap: 0.75rem;
  padding: 0.75rem;
}

.gate-toggle {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  white-space: nowrap;
  font-size: var(--text-caption);
  color: var(--muted-foreground);
}

@media (max-width: 1100px) {
  .gate-filters {
    grid-template-columns: 1fr;
  }
}
</style>
