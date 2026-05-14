<script setup lang="ts">
import { computed, ref } from 'vue';
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
  isPausedAll: boolean;
}>();

const emit = defineEmits<{
  'update:open': [open: boolean];
  submit: [];
  'update-preview-valid': [valid: boolean];
}>();

const mcpActionOptions = [
  'maenifold.gate.gate_Status',
  'maenifold.gate.gate_Sync',
  'maenifold.memory.ReadMemory',
  'maenifold.memory.ListMemories',
  'maenifold.memory.MemoryStatus',
  'maenifold.graph.BuildContext',
  'maenifold.graph.RecentActivity',
];
const customMcpAction = ref(false);
const selectedMcpAction = computed(() => {
  if (customMcpAction.value) return '__custom__';
  if (!props.form.mcpAction) return '__none__';
  return mcpActionOptions.includes(props.form.mcpAction) ? props.form.mcpAction : '__custom__';
});
const mcpParametersInvalid = computed(() => {
  if (!props.form.mcpAction.trim()) return false;
  const raw = props.form.mcpParameters.trim();
  if (!raw) return false;
  try {
    const parsed = JSON.parse(raw) as unknown;
    return !(typeof parsed === 'object' && !Array.isArray(parsed) && parsed !== null);
  } catch {
    return true;
  }
});

function selectMcpAction(value: unknown): void {
  const selected = String(value ?? '__none__');
  if (selected === '__none__') {
    customMcpAction.value = false;
    props.form.mcpAction = '';
    return;
  }
  if (selected === '__custom__') {
    customMcpAction.value = true;
    props.form.mcpAction = '';
    return;
  }
  customMcpAction.value = false;
  props.form.mcpAction = selected;
}
</script>

<template>
  <Dialog :open="props.open" @update:open="(open) => emit('update:open', open)">
    <DialogContent class="sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle>{{ title }}</DialogTitle>
        <DialogDescription>{{ description }}</DialogDescription>
      </DialogHeader>

      <form class="grid gap-4" @submit.prevent="emit('submit')">
        <div v-if="isPausedAll" class="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          ⚠ Pause-all is active. Saving with "Enabled" turned on will fail. Resume schedules first to enable individual jobs.
        </div>
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
        <div class="grid gap-2">
          <Label for="schedule-mcp-action">MCP action</Label>
          <Select :model-value="selectedMcpAction" :disabled="isSubmitting" @update:model-value="selectMcpAction">
            <SelectTrigger id="schedule-mcp-action">
              <SelectValue placeholder="Agent prompt (no MCP action)" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__none__">Agent prompt (no MCP action)</SelectItem>
              <SelectItem v-for="action in mcpActionOptions" :key="action" :value="action">{{ action }}</SelectItem>
              <SelectItem value="__custom__">(custom)</SelectItem>
            </SelectContent>
          </Select>
          <Input
            v-if="selectedMcpAction === '__custom__'"
            id="schedule-mcp-action-custom"
            v-model="form.mcpAction"
            :disabled="isSubmitting"
            placeholder="maenifold.gate.gate_Status"
          />
          <Textarea
            id="schedule-mcp-parameters"
            v-model="form.mcpParameters"
            :disabled="isSubmitting"
            class="font-mono"
            placeholder="{&#10;  &quot;root&quot;: &quot;/workspace&quot;&#10;}"
          />
          <p class="text-sm text-muted-foreground">
            Parameters must be a JSON object. Allowlisted examples include maenifold.gate.gate_Status and maenifold.graph.BuildContext.
          </p>
          <p v-if="form.mcpAction.trim()" class="text-sm text-muted-foreground">
            MCP action mode: prompt is ignored at fire time.
          </p>
          <p v-if="mcpParametersInvalid" class="text-sm text-destructive">
            Parameters must be a JSON object.
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
          <Button type="submit" :disabled="!canSubmit || mcpParametersInvalid">
            {{ isSubmitting ? 'Saving…' : mode === 'create' ? 'Create schedule' : 'Save changes' }}
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
