<script setup lang="ts">
import { Loader2, Pencil, Play, Plus, RefreshCw, Trash2 } from 'lucide-vue-next';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Textarea } from '@/components/ui/textarea';
import { useSchedulesSettings } from '@/composables/useSchedulesSettings';

const {
  agentNames,
  jobs,
  isLoading,
  isSubmitting,
  isActionInProgress,
  error,
  statusMessage,
  dialogOpen,
  dialogMode,
  dialogTitle,
  dialogDescription,
  canSubmit,
  form,
  schedulePlatform,
  scheduleLabel,
  statusFor,
  permissionLabel,
  permissionVariant,
  allowedToolsLabel,
  formatLastRun,
  loadSchedules,
  openCreateDialog,
  openEditDialog,
  saveSchedule,
  runSchedule,
  deleteSchedule,
} = useSchedulesSettings();
</script>

<template>
  <section class="flex h-full flex-col gap-4 overflow-auto p-6" aria-label="Schedules">
    <div class="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
      <div class="flex flex-col gap-1">
        <h2 class="text-lg font-semibold text-text-primary">Schedules</h2>
        <p class="text-sm text-text-secondary">Recurring agent tasks installed via macOS launchd or Linux systemd user timers.</p>
      </div>
      <div class="flex gap-2">
        <Button type="button" variant="outline" size="sm" :disabled="isLoading || isSubmitting" @click="loadSchedules">
          <RefreshCw class="h-4 w-4" />
          Refresh
        </Button>
        <Button type="button" size="sm" :disabled="isLoading" @click="openCreateDialog">
          <Plus class="h-4 w-4" />
          New schedule
        </Button>
      </div>
    </div>

    <div v-if="error" class="flex items-center justify-between gap-3 rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
      <span>{{ error }}</span>
      <Button type="button" variant="outline" size="sm" @click="loadSchedules">Retry</Button>
    </div>
    <p v-else-if="statusMessage" class="text-sm text-primary" role="status">{{ statusMessage }}</p>

    <div class="rounded-md border px-4 py-3 text-sm" :class="schedulePlatform.supportsEnabledSchedules ? 'border-primary/30 bg-primary/5 text-text-primary' : 'border-destructive/30 bg-destructive/10 text-destructive'">
      <div class="font-medium">{{ schedulePlatform.label }}</div>
      <p class="mt-1">
        {{ schedulePlatform.description }}
        <span class="text-muted-foreground">Supported backends: macOS launchd, Linux systemd user timers. Windows is unsupported.</span>
      </p>
    </div>

    <div v-if="isLoading" class="flex items-center gap-2 rounded-md border bg-muted/40 px-4 py-6 text-sm text-muted-foreground">
      <Loader2 class="h-4 w-4 animate-spin" />
      Loading schedules…
    </div>

    <div v-else class="rounded-md border">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Name</TableHead>
            <TableHead>Schedule</TableHead>
            <TableHead>Agent</TableHead>
            <TableHead>Permissions</TableHead>
            <TableHead>Status</TableHead>
            <TableHead>Last run</TableHead>
            <TableHead class="text-right">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow v-if="jobs.length === 0">
            <TableCell colspan="7" class="py-8 text-center text-sm text-muted-foreground">
              No schedules configured yet.
            </TableCell>
          </TableRow>
          <TableRow v-for="job in jobs" :key="job.id">
            <TableCell class="font-medium text-text-primary">{{ job.id }}</TableCell>
            <TableCell class="max-w-[18rem] text-sm text-muted-foreground">{{ scheduleLabel(job) }}</TableCell>
            <TableCell>{{ job.agent }}</TableCell>
            <TableCell>
              <div class="flex flex-col gap-1">
                <Badge :variant="permissionVariant(job)">{{ permissionLabel(job) }}</Badge>
                <span class="text-xs text-muted-foreground">{{ allowedToolsLabel(job) }}</span>
              </div>
            </TableCell>
            <TableCell><Badge :variant="statusFor(job).variant">{{ statusFor(job).label }}</Badge></TableCell>
            <TableCell class="max-w-[18rem] text-sm text-muted-foreground">
              <div>{{ formatLastRun(job.last_run?.timestamp) }}</div>
              <p v-if="job.last_run?.error" class="mt-1 text-xs text-destructive">{{ job.last_run.error }}</p>
            </TableCell>
            <TableCell>
              <div class="flex flex-wrap justify-end gap-2">
                <Button type="button" variant="outline" size="sm" :disabled="isActionInProgress(job.id) || isSubmitting" @click="openEditDialog(job)">
                  <Pencil class="h-3.5 w-3.5" />
                  Edit
                </Button>
                <Button type="button" variant="outline" size="sm" :disabled="isActionInProgress(job.id) || isSubmitting" @click="runSchedule(job)">
                  <Loader2 v-if="isActionInProgress(job.id)" class="h-3.5 w-3.5 animate-spin" />
                  <Play v-else class="h-3.5 w-3.5" />
                  Run now
                </Button>
                <Button type="button" variant="destructive" size="sm" :disabled="isActionInProgress(job.id) || isSubmitting" @click="deleteSchedule(job)">
                  <Trash2 class="h-3.5 w-3.5" />
                  Delete
                </Button>
              </div>
            </TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </div>

    <Dialog :open="dialogOpen" @update:open="(open) => dialogOpen = open">
      <DialogContent class="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{{ dialogTitle }}</DialogTitle>
          <DialogDescription>{{ dialogDescription }}</DialogDescription>
        </DialogHeader>

        <form class="grid gap-4" @submit.prevent="saveSchedule">
          <div class="grid gap-2">
            <Label for="schedule-id">ID</Label>
            <Input id="schedule-id" v-model="form.id" :disabled="dialogMode === 'edit' || isSubmitting" placeholder="daily-triage" />
          </div>
          <div class="grid gap-2">
            <Label for="schedule-expression">Schedule</Label>
            <Input id="schedule-expression" v-model="form.schedule" :disabled="isSubmitting" placeholder="every weekday at 9am" />
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
            <Textarea
              id="schedule-allowed-tools"
              v-model="form.allowedTools"
              :disabled="isSubmitting"
              placeholder="Optional comma- or newline-separated tool names, e.g. ReadMemory, SearchMemories"
            />
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
                <template v-else>
                  {{ schedulePlatform.unsupportedEnabledMessage }}
                </template>
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
            <Button type="button" variant="outline" :disabled="isSubmitting" @click="dialogOpen = false">Cancel</Button>
            <Button type="submit" :disabled="!canSubmit">
              {{ isSubmitting ? 'Saving…' : dialogMode === 'create' ? 'Create schedule' : 'Save changes' }}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  </section>
</template>
