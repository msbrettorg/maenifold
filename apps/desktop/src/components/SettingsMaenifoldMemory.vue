<script setup lang="ts">
import type { MaenifoldSettings } from '../lib/types';
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

type OptionalNumberField = 'sessionAbandonMinutes' | 'snippetLength';

defineProps<{
  form: MaenifoldSettings;
  onUpdateOptionalNumber: (field: OptionalNumberField, event: Event) => void;
  onUseDefaultToggle: (field: OptionalNumberField, event: Event, defaultValue?: number) => void;
  onUpdateSyncRole: (event: Event) => void;
}>();
</script>

<template>
  <Accordion
    type="multiple"
    :default-value="['general', 'database', 'session']"
    class="settings-groups"
    role="group"
    aria-label="Maenifold memory settings"
  >
    <AccordionItem value="general" class="settings-card details-section">
      <AccordionTrigger><h4>General</h4></AccordionTrigger>
      <AccordionContent>
        <div class="section-grid">
        <div class="form-group">
          <Label>Log rotation</Label>
          <Select :model-value="form.logRotation" @update:model-value="(val) => form.logRotation = val as typeof form.logRotation">
            <SelectTrigger class="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="daily">daily</SelectItem>
              <SelectItem value="hourly">hourly</SelectItem>
              <SelectItem value="never">never</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div class="form-group">
          <Label>Debounce ms</Label>
          <Input :model-value="form.debounceMs" type="number" min="0" step="1" @update:model-value="(val) => { form.debounceMs = Number(val) }" />
        </div>
        <div class="checkbox-group"><Checkbox :checked="form.autoSync" @update:checked="(val: boolean) => { form.autoSync = val }" /><Label>Auto sync</Label></div>
        <div class="checkbox-group"><Checkbox :checked="form.syncLogging" @update:checked="(val: boolean) => { form.syncLogging = val }" /><Label>Sync logging</Label></div>
        <div class="checkbox-group"><Checkbox :checked="form.sessionCleanup" @update:checked="(val: boolean) => { form.sessionCleanup = val }" /><Label>Session cleanup</Label></div>
      </div>
      </AccordionContent>
    </AccordionItem>

    <AccordionItem value="database" class="settings-card details-section">
      <AccordionTrigger><h4>Database</h4></AccordionTrigger>
      <AccordionContent>
        <div class="section-grid">
        <div class="form-group">
          <Label>SQLite busy timeout ms</Label>
          <Input :model-value="form.sqliteBusyTimeoutMs" type="number" min="0" step="1" @update:model-value="(val) => { form.sqliteBusyTimeoutMs = Number(val) }" />
        </div>
        <div class="form-group">
          <Label>Sync lease ms</Label>
          <Input :model-value="form.syncLeaseMs" type="number" min="0" step="1" @update:model-value="(val) => { form.syncLeaseMs = Number(val) }" />
        </div>
        <div class="form-group">
          <Label>Watcher buffer size</Label>
          <Input :model-value="form.watcherBufferSize" type="number" min="0" step="1" @update:model-value="(val) => { form.watcherBufferSize = Number(val) }" />
        </div>
        <div class="form-group">
          <Label>Incremental optimize every</Label>
          <Input :model-value="form.incrementalOptimizeEvery" type="number" min="0" step="1" @update:model-value="(val) => { form.incrementalOptimizeEvery = Number(val) }" />
        </div>
        <div class="form-group">
          <Label>Incremental vacuum minutes</Label>
          <Input :model-value="form.incrementalVacuumMinutes" type="number" min="0" step="1" @update:model-value="(val) => { form.incrementalVacuumMinutes = Number(val) }" />
        </div>
      </div>
      </AccordionContent>
    </AccordionItem>

    <AccordionItem value="session" class="settings-card details-section">
      <AccordionTrigger><h4>Session</h4></AccordionTrigger>
      <AccordionContent>
        <div class="section-grid">
        <div class="optional-field">
          <div class="form-group">
            <Label>Session abandon minutes</Label>
            <Input
              :disabled="form.sessionAbandonMinutes === null"
              :model-value="form.sessionAbandonMinutes ?? 0"
              type="number"
              min="0"
              step="1"
              @update:model-value="(val) => onUpdateOptionalNumber('sessionAbandonMinutes', { target: { value: String(val) } } as unknown as Event)"
            />
          </div>
          <div class="checkbox-inline">
            <Checkbox
              :checked="form.sessionAbandonMinutes === null"
              @update:checked="(val: boolean) => onUseDefaultToggle('sessionAbandonMinutes', { target: { checked: val } } as unknown as Event)"
            />
            <Label>Use default</Label>
          </div>
        </div>

        <div class="optional-field">
          <div class="form-group">
            <Label>Snippet length</Label>
            <Input
              :disabled="form.snippetLength === null"
              :model-value="form.snippetLength ?? 0"
              type="number"
              min="0"
              step="1"
              @update:model-value="(val) => onUpdateOptionalNumber('snippetLength', { target: { value: String(val) } } as unknown as Event)"
            />
          </div>
          <div class="checkbox-inline">
            <Checkbox
              :checked="form.snippetLength === null"
              @update:checked="(val: boolean) => onUseDefaultToggle('snippetLength', { target: { checked: val } } as unknown as Event)"
            />
            <Label>Use default</Label>
          </div>
        </div>

        <div class="form-group">
          <Label>Sync role</Label>
          <Select :model-value="form.syncRole ?? '__auto__'" @update:model-value="(val) => onUpdateSyncRole({ target: { value: String(val) } } as unknown as Event)">
            <SelectTrigger class="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__auto__">auto</SelectItem>
              <SelectItem value="primary">primary</SelectItem>
              <SelectItem value="replica">replica</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>
      </AccordionContent>
    </AccordionItem>
  </Accordion>
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
