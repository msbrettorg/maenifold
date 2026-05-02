<script setup lang="ts">
import { ref } from 'vue';
import { CalendarClock, HelpCircle, Plus } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

const emit = defineEmits<{
  create: [];
}>();

const helpOpen = ref(false);

const examples = [
  {
    title: 'Sync graph nightly at 2am',
    summary: 'Rebuild WikiLink routing while the app is quiet.',
    schedule: '0 2 * * *',
    command: 'GatewayExecute maenifold.graph Sync',
  },
  {
    title: 'Daily memory status at 9am',
    summary: 'Check corpus health before the workday starts.',
    schedule: '0 9 * * *',
    command: 'GatewayExecute maenifold.system MemoryStatus',
  },
  {
    title: 'Hourly recent-activity refresh',
    summary: 'Review new memory and thinking activity each hour.',
    schedule: '0 * * * *',
    command: 'GatewayExecute maenifold.system RecentActivity filter=all',
  },
];
</script>

<template>
  <Card class="border-dashed bg-muted/20">
    <CardHeader class="gap-3 text-center">
      <div class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-primary/10 text-primary">
        <CalendarClock class="h-6 w-6" />
      </div>
      <div class="space-y-1">
        <CardTitle class="text-xl">Create your first schedule</CardTitle>
        <CardDescription>
          Schedules run a maenifold command on a recurring cron.
        </CardDescription>
      </div>
    </CardHeader>
    <CardContent class="flex flex-col gap-5">
      <div class="grid gap-3 lg:grid-cols-3" aria-label="Example schedules">
        <Card
          v-for="example in examples"
          :key="example.title"
          size="sm"
          tabindex="0"
          class="group border-border/70 bg-bg-elevated transition-colors hover:bg-bg-surface-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
        >
          <CardHeader class="gap-1">
            <CardTitle class="text-sm">{{ example.title }}</CardTitle>
            <CardDescription>{{ example.summary }}</CardDescription>
          </CardHeader>
          <CardContent class="grid gap-2 text-xs text-muted-foreground">
            <div class="grid gap-2 rounded-md bg-muted/40 px-3 py-2">
              <div>
                <span class="font-medium text-text-primary">Cron:</span>
                <code class="ml-1">{{ example.schedule }}</code>
              </div>
              <div>
                <span class="font-medium text-text-primary">Command:</span>
                <code class="ml-1 break-all">{{ example.command }}</code>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      <div class="flex flex-col items-center justify-center gap-3 sm:flex-row">
        <Button type="button" @click="emit('create')">
          <Plus class="h-4 w-4" />
          Create your first schedule
        </Button>
        <Button type="button" variant="ghost" @click="helpOpen = true">
          <HelpCircle class="h-4 w-4" />
          What is a schedule?
        </Button>
      </div>
    </CardContent>
  </Card>

  <Dialog v-model:open="helpOpen">
    <DialogContent class="sm:max-w-lg">
      <DialogHeader>
        <DialogTitle>What is a schedule?</DialogTitle>
        <DialogDescription>
          A schedule installs a recurring task that asks an ACP agent to run a maenifold command at a cron interval.
        </DialogDescription>
      </DialogHeader>
      <div class="grid gap-3 text-sm text-muted-foreground">
        <p>
          Safe defaults: keep automatic permissions off, start with read-only maintenance commands, and use an allow-list when a prompt needs tools.
        </p>
        <p>
          Good first schedules include graph sync, memory status checks, and recent activity reviews. You can run any schedule manually before enabling it.
        </p>
      </div>
      <DialogFooter>
        <Button type="button" @click="helpOpen = false">Got it</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
