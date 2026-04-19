<script setup lang="ts">
import type { MaenifoldSettings } from '../lib/types';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

defineProps<{
  form: MaenifoldSettings;
  onUpdatePipelineDriver: (event: Event) => void;
  onPipelineDriverDefaultToggle: (event: Event) => void;
}>();
</script>

<template>
  <div class="settings-groups" role="group" aria-label="Maenifold workflow settings">
    <details class="settings-card details-section" open>
      <summary><h4>Workflows &amp; Debug</h4></summary>
      <div class="section-grid">
        <div class="checkbox-group">
          <Checkbox :checked="form.debug.trace" @update:checked="(val: boolean) => { form.debug.trace = val }" />
          <Label>Trace logging</Label>
        </div>

        <div class="optional-field optional-field-full">
          <div class="form-group">
            <Label>Pipeline driver</Label>
            <Input
              :disabled="form.debug.pipelineDriver === null"
              :model-value="form.debug.pipelineDriver ?? ''"
              type="text"
              placeholder="inline"
              @update:model-value="(val) => onUpdatePipelineDriver({ target: { value: String(val) } } as unknown as Event)"
            />
          </div>
          <div class="checkbox-inline">
            <Checkbox
              :checked="form.debug.pipelineDriver === null"
              @update:checked="(val: boolean) => onPipelineDriverDefaultToggle({ target: { checked: val } } as unknown as Event)"
            />
            <Label>Use default</Label>
          </div>
        </div>
      </div>
    </details>
  </div>
</template>

<style scoped>
.settings-groups {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.settings-card {
  padding: 1rem;
  border-radius: var(--radius);
  background: var(--color-bg-elevated);
}

.details-section {
  border: 1px solid var(--color-border-subtle);
}

.details-section summary {
  cursor: pointer;
  list-style: none;
}

.details-section summary::-webkit-details-marker {
  display: none;
}

.details-section[open] summary {
  margin-bottom: 0.75rem;
}

.details-section summary h4 {
  margin: 0;
}

.section-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 0.9rem;
}

.form-group,
.optional-field {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.form-group input {
  width: 100%;
  padding: 0.5rem;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: var(--color-bg-app);
  color: var(--color-text-primary);
}

.checkbox-group,
.checkbox-inline {
  display: flex;
  gap: 0.65rem;
  align-items: center;
}

.checkbox-group {
  min-height: 42px;
  padding: 0.75rem;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: var(--color-bg-app);
}

.optional-field {
  padding: 0.9rem;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: var(--color-bg-app);
}

.optional-field-full {
  grid-column: 1 / -1;
}
</style>
