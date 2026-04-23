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
    class="flex flex-col gap-4"
    role="group"
    aria-label="Maenifold memory settings"
  >
    <AccordionItem value="general" class="p-4 rounded-[var(--radius)] bg-bg-elevated border border-border-subtle">
      <AccordionTrigger><h4 class="m-0">General</h4></AccordionTrigger>
      <AccordionContent>
        <div class="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-[0.9rem]">
        <div class="flex flex-col gap-[0.45rem]">
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
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Debounce ms</Label>
          <Input :model-value="form.debounceMs" type="number" min="0" step="1" @update:model-value="(val) => { form.debounceMs = Number(val) }" />
        </div>
        <div class="flex gap-[0.65rem] items-center min-h-[42px] p-3 border border-border-subtle rounded-[var(--radius)] bg-bg-app"><Checkbox :checked="form.autoSync" @update:checked="(val: boolean) => { form.autoSync = val }" /><Label>Auto sync</Label></div>
        <div class="flex gap-[0.65rem] items-center min-h-[42px] p-3 border border-border-subtle rounded-[var(--radius)] bg-bg-app"><Checkbox :checked="form.syncLogging" @update:checked="(val: boolean) => { form.syncLogging = val }" /><Label>Sync logging</Label></div>
        <div class="flex gap-[0.65rem] items-center min-h-[42px] p-3 border border-border-subtle rounded-[var(--radius)] bg-bg-app"><Checkbox :checked="form.sessionCleanup" @update:checked="(val: boolean) => { form.sessionCleanup = val }" /><Label>Session cleanup</Label></div>
      </div>
      </AccordionContent>
    </AccordionItem>

    <AccordionItem value="database" class="p-4 rounded-[var(--radius)] bg-bg-elevated border border-border-subtle">
      <AccordionTrigger><h4 class="m-0">Database</h4></AccordionTrigger>
      <AccordionContent>
        <div class="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-[0.9rem]">
        <div class="flex flex-col gap-[0.45rem]">
          <Label>SQLite busy timeout ms</Label>
          <Input :model-value="form.sqliteBusyTimeoutMs" type="number" min="0" step="1" @update:model-value="(val) => { form.sqliteBusyTimeoutMs = Number(val) }" />
        </div>
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Sync lease ms</Label>
          <Input :model-value="form.syncLeaseMs" type="number" min="0" step="1" @update:model-value="(val) => { form.syncLeaseMs = Number(val) }" />
        </div>
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Watcher buffer size</Label>
          <Input :model-value="form.watcherBufferSize" type="number" min="0" step="1" @update:model-value="(val) => { form.watcherBufferSize = Number(val) }" />
        </div>
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Incremental optimize every</Label>
          <Input :model-value="form.incrementalOptimizeEvery" type="number" min="0" step="1" @update:model-value="(val) => { form.incrementalOptimizeEvery = Number(val) }" />
        </div>
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Incremental vacuum minutes</Label>
          <Input :model-value="form.incrementalVacuumMinutes" type="number" min="0" step="1" @update:model-value="(val) => { form.incrementalVacuumMinutes = Number(val) }" />
        </div>
      </div>
      </AccordionContent>
    </AccordionItem>

    <AccordionItem value="session" class="p-4 rounded-[var(--radius)] bg-bg-elevated border border-border-subtle">
      <AccordionTrigger><h4 class="m-0">Session</h4></AccordionTrigger>
      <AccordionContent>
        <div class="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-[0.9rem]">
        <div class="flex flex-col gap-[0.45rem] p-[0.9rem] border border-border-subtle rounded-[var(--radius)] bg-bg-app">
          <div class="flex flex-col gap-[0.45rem]">
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
          <div class="flex gap-[0.65rem] items-center">
            <Checkbox
              :checked="form.sessionAbandonMinutes === null"
              @update:checked="(val: boolean) => onUseDefaultToggle('sessionAbandonMinutes', { target: { checked: val } } as unknown as Event)"
            />
            <Label>Use default</Label>
          </div>
        </div>

        <div class="flex flex-col gap-[0.45rem] p-[0.9rem] border border-border-subtle rounded-[var(--radius)] bg-bg-app">
          <div class="flex flex-col gap-[0.45rem]">
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
          <div class="flex gap-[0.65rem] items-center">
            <Checkbox
              :checked="form.snippetLength === null"
              @update:checked="(val: boolean) => onUseDefaultToggle('snippetLength', { target: { checked: val } } as unknown as Event)"
            />
            <Label>Use default</Label>
          </div>
        </div>

        <div class="flex flex-col gap-[0.45rem]">
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
