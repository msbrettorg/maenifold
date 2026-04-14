<script setup lang="ts">
import { Input } from '@/components/ui/input';

const selectFieldClass =
  'w-full rounded-md border border-white/10 bg-[#201f1f] px-3 py-2 text-caption text-foreground outline-none transition focus:border-[#4d8eff]/40 focus:bg-[#2a2a2a] focus:ring-2 focus:ring-[#4d8eff]/15';

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
  <div class="gate-filters" role="search" aria-label="Task filters">
    <Input
      class="gate-filters__search"
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
  padding: 0.9rem 1rem;
  background: #1c1b1b;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 0.25rem;
}

.gate-filters :deep(.gate-filters__search) {
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: #201f1f;
  transition: background 160ms ease, border-color 160ms ease, box-shadow 160ms ease;
}

.gate-filters :deep(.gate-filters__search:focus) {
  border-color: rgb(77 142 255 / 0.4);
  background: #2a2a2a;
  box-shadow: 0 0 0 2px rgb(77 142 255 / 0.15);
}

.gate-toggle {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  min-height: 2.5rem;
  padding: 0 0.75rem;
  white-space: nowrap;
  font-size: var(--text-caption);
  color: var(--foreground);
  background: #201f1f;
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: 0.25rem;
}

.gate-toggle input {
  accent-color: #4d8eff;
}

@media (max-width: 1100px) {
  .gate-filters {
    grid-template-columns: 1fr;
  }
}
</style>
