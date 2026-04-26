<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

type Fidelity = 'low' | 'medium' | 'high';

defineProps<{
  continuity: boolean;
  inputFidelity: string;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (event: 'update-continuity', value: boolean): void;
  (event: 'update-fidelity', value: Fidelity): void;
}>();

const fidelityOptions: { value: Fidelity; label: string }[] = [
  { value: 'low', label: 'Low' },
  { value: 'medium', label: 'Medium' },
  { value: 'high', label: 'High' },
];
</script>

<template>
  <div class="flex flex-col gap-3 rounded-md border bg-muted/40 p-4">
    <div class="flex items-center gap-3">
      <Checkbox
        id="easter-eggs-continuity"
        :checked="continuity"
        :disabled="disabled"
        @update:checked="(value: boolean | 'indeterminate') => emit('update-continuity', value === true)"
      />
      <Label for="easter-eggs-continuity" class="text-sm font-semibold">Evolve from previous image</Label>
    </div>

    <div v-if="continuity" class="flex flex-col gap-2">
      <Label for="easter-eggs-fidelity" class="text-sm font-semibold">Input fidelity</Label>
      <Select
        :model-value="inputFidelity"
        @update:model-value="(value) => emit('update-fidelity', value as Fidelity)"
      >
        <SelectTrigger id="easter-eggs-fidelity" :disabled="disabled" class="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem v-for="option in fidelityOptions" :key="option.value" :value="option.value">
            {{ option.label }}
          </SelectItem>
        </SelectContent>
      </Select>
    </div>

    <Button
      type="button"
      variant="outline"
      class="self-start"
      disabled
      title="Available when the reset wallpaper backend tool ships."
    >
      Reset wallpaper
    </Button>
  </div>
</template>
