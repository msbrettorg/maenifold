<script setup lang="ts">
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import ScheduleCronPreview from '@/components/ScheduleCronPreview.vue';
import type { ScheduleForm } from '@/composables/useSchedulesSettings';
import type { SchedulePlatformSupport } from '@/composables/useSchedulePlatform';

const props = defineProps<{
  open: boolean;
  form: ScheduleForm;
  mode: 'create' | 'edit';
  title: string;
  description: string;
  agentNames: string[];
  isSubmitting: boolean;
  canSubmit: boolean;
  schedulePlatform: SchedulePlatformSupport;
}>();

const emit = defineEmits<{
  'update:open': [open: boolean];
  submit: [];
  'update-preview-valid': [valid: boolean];
}>();
</script>

<template>
  <Dialog :open="props.open" @update:open="(open) => emit('update:open', open)">
    <DialogContent class="sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle>{{ title }}</DialogTitle>
        <DialogDescription>{{ description }}</DialogDescription>
      </DialogHeader>

      <form class="grid gap-4" @submit.prevent="emit('submit')">
        <div class="grid gap-2">
          <Label for="schedule-id">ID</Label>
          <Input id="schedule-id" v-model="form.id" :disabled="mode === 'edit' || isSubmitting" placeholder="daily-triage" />
        </div>
        <div class="grid gap-2">
          <Label for="schedule-expression">Schedule</Label>
          <Input id="schedule-expression" v-model="form.schedule" :disabled="isSubmitting" placeholder="every weekday at 9am" />
          <ScheduleCronPreview :expression="form.schedule" @preview-valid="(valid) => emit('update-preview-valid', valid)" />
        </div>
        <div class="grid gap-2">
          <Label for="schedule-agent">Agent</Label>
          <Select :model-value="form.agent" :disabled="isSubmitting || agentNames.length === 0" @update:model-value="(value) => { form.agent = String(value ?? ''); }">
            <SelectTrigger id="schedule-agent">
              <SelectValue :placeholder="agentNames.length === 0 ? 'No agents available' : 'Select an agent'" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem v-for="agent in agentNames" :key="agent" :value="agent">{{ agent }}</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div class="grid gap-2">
          <Label for="schedule-prompt">Prompt</Label>
          <Textarea id="schedule-prompt" v-model="form.prompt" :disabled="isSubmitting" placeholder="Summarize overnight alerts and send the findings to the team." />
        </div>
        <div class="grid gap-2">
          <Label for="schedule-description">Description</Label>
          <Input id="schedule-description" v-model="form.description" :disabled="isSubmitting" placeholder="Optional description" />
        </div>
        <div class="flex items-center justify-between rounded-md border border-destructive/30 bg-destructive/5 px-4 py-3">
          <div class="flex flex-col gap-1">
            <Label for="schedule-auto-grant">Automatic permissions</Label>
            <p class="text-sm text-muted-foreground">
              Off by default. Turn on only for trusted prompts that may approve sensitive ACP tool requests unattended.
            </p>
          </div>
          <Switch
            id="schedule-auto-grant"
            :model-value="form.autoGrantPermissions"
            :disabled="isSubmitting"
            @update:model-value="(value: boolean) => { form.autoGrantPermissions = value; }"
          />
        </div>
        <div class="grid gap-2">
          <Label for="schedule-allowed-tools">Allowed tools</Label>
          <Textarea id="schedule-allowed-tools" v-model="form.allowedTools" :disabled="isSubmitting" placeholder="Optional comma- or newline-separated tool names, e.g. ReadMemory, SearchMemories" />
          <p class="text-sm text-muted-foreground">
            Threaded to ACP as <code>allowedTools</code>; leave blank for no per-schedule allowlist.
          </p>
        </div>
        <div class="flex items-center justify-between rounded-md border bg-muted/40 px-4 py-3">
          <div class="flex flex-col gap-1">
            <Label for="schedule-enabled">Enabled</Label>
            <p class="text-sm text-muted-foreground">
              <template v-if="schedulePlatform.supportsEnabledSchedules">
                Install this schedule with {{ schedulePlatform.scheduler }}, or pause it without deleting its configuration.
              </template>
              <template v-else>{{ schedulePlatform.unsupportedEnabledMessage }}</template>
            </p>
          </div>
          <Switch
            id="schedule-enabled"
            :model-value="form.enabled"
            :disabled="isSubmitting || !schedulePlatform.supportsEnabledSchedules"
            @update:model-value="(value: boolean) => { form.enabled = value; }"
          />
        </div>

        <DialogFooter>
          <Button type="button" variant="outline" :disabled="isSubmitting" @click="emit('update:open', false)">Cancel</Button>
          <Button type="submit" :disabled="!canSubmit">
            {{ isSubmitting ? 'Saving…' : mode === 'create' ? 'Create schedule' : 'Save changes' }}
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
