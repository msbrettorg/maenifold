<script setup lang="ts">
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { MaenifoldSettings } from '../lib/types';

const props = defineProps<{
  form: MaenifoldSettings;
  onCommunityDefaultToggle: (event: Event) => void;
  onUpdateCommunityRecompute: (event: Event) => void;
}>();

function emitCommunityRecompute(value: string | number) {
  const synthetic = {
    target: { value: String(value) },
  } as unknown as Event;
  props.onUpdateCommunityRecompute(synthetic);
}

function emitCommunityDefaultToggle(checked: boolean | 'indeterminate') {
  const synthetic = {
    target: { checked: checked === true },
  } as unknown as Event;
  props.onCommunityDefaultToggle(synthetic);
}
</script>

<template>
  <div class="settings-groups" role="group" aria-label="Maenifold graph settings">
    <details class="settings-card details-section" open>
      <summary><h4>Decay</h4></summary>
      <div class="section-grid">
        <div class="form-group">
          <Label>Grace days sequential</Label>
          <Input
            :model-value="form.decay.graceDaysSequential"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.decay.graceDaysSequential = Number(val)"
          />
        </div>
        <div class="form-group">
          <Label>Grace days workflows</Label>
          <Input
            :model-value="form.decay.graceDaysWorkflows"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.decay.graceDaysWorkflows = Number(val)"
          />
        </div>
        <div class="form-group">
          <Label>Grace days default</Label>
          <Input
            :model-value="form.decay.graceDaysDefault"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.decay.graceDaysDefault = Number(val)"
          />
        </div>
        <div class="form-group">
          <Label>Half life days</Label>
          <Input
            :model-value="form.decay.halfLifeDays"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.decay.halfLifeDays = Number(val)"
          />
        </div>
        <div class="form-group">
          <Label>Decay function</Label>
          <Select v-model="form.decay.function">
            <SelectTrigger class="w-full">
              <SelectValue placeholder="Select decay function" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="power-law">power-law</SelectItem>
              <SelectItem value="exponential">exponential</SelectItem>
              <SelectItem value="linear">linear</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>
    </details>

    <details class="settings-card details-section" open>
      <summary><h4>Community Detection</h4></summary>
      <div class="section-grid">
        <div class="form-group">
          <Label>Louvain gamma</Label>
          <Input
            :model-value="form.community.louvainGamma"
            type="number"
            min="0"
            step="0.1"
            @update:model-value="(val: string | number) => form.community.louvainGamma = Number(val)"
          />
        </div>
        <div class="form-group">
          <Label>Min shared neighbors</Label>
          <Input
            :model-value="form.community.minSharedNeighbors"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.community.minSharedNeighbors = Number(val)"
          />
        </div>
        <div class="form-group">
          <Label>Min overlap</Label>
          <Input
            :model-value="form.community.minOverlap"
            type="number"
            min="0"
            max="1"
            step="0.05"
            @update:model-value="(val: string | number) => form.community.minOverlap = Number(val)"
          />
        </div>
        <div class="form-group">
          <Label>Max siblings</Label>
          <Input
            :model-value="form.community.maxSiblings"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.community.maxSiblings = Number(val)"
          />
        </div>
        <div class="form-group">
          <Label>Debounce ms</Label>
          <Input
            :model-value="form.community.debounceMs"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.community.debounceMs = Number(val)"
          />
        </div>
        <div class="form-group">
          <Label>NMI threshold</Label>
          <Input
            :model-value="form.community.nmiThreshold"
            type="number"
            min="0"
            max="1"
            step="0.05"
            @update:model-value="(val: string | number) => form.community.nmiThreshold = Number(val)"
          />
        </div>
        <div class="optional-field">
          <div class="form-group">
            <Label>Recompute file threshold</Label>
            <Input
              :disabled="form.community.recomputeFileThreshold === null"
              :model-value="form.community.recomputeFileThreshold ?? 0"
              type="number"
              min="0"
              step="1"
              @update:model-value="emitCommunityRecompute"
            />
          </div>
          <Label class="checkbox-inline">
            <Checkbox
              :model-value="form.community.recomputeFileThreshold === null"
              @update:model-value="emitCommunityDefaultToggle"
            />
            <span>Use default</span>
          </Label>
        </div>
      </div>
    </details>

    <details class="settings-card details-section" open>
      <summary><h4>Vector / Embedding</h4></summary>
      <div class="section-grid">
        <Label class="checkbox-group"><Checkbox v-model="form.vector.embeddingLogs" /><span>Embedding logs</span></Label>
        <Label class="checkbox-group"><Checkbox v-model="form.vector.vectorLogs" /><span>Vector logs</span></Label>
        <Label class="checkbox-group"><Checkbox v-model="form.vector.disableVector" /><span>Disable vector</span></Label>
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

.form-group input,
.form-group select {
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
</style>
