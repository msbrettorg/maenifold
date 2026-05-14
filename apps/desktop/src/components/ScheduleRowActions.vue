<script setup lang="ts">
import { ref } from 'vue';
import { Calendar, Loader2 } from 'lucide-vue-next';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { useMcpClient } from '@/composables/useMcpClient';

interface BackfillResult {
  action: 'backfill';
  scheduleId: string;
  plannedCount: number;
  executedCount: number;
  skippedCount: number;
  runIds: string[];
}

const props = defineProps<{
  scheduleId: string;
}>();

const emit = defineEmits<{
  backfilled: [result: BackfillResult];
}>();

const { callTool } = useMcpClient();
const fieldIdBase = `schedule-backfill-${props.scheduleId}`;
const sevenDaysAgo = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString();
const since = ref(sevenDaysAgo);
const until = ref('');
const running = ref(false);
const error = ref('');
const dialogOpen = ref(false);

async function runBackfill(): Promise<void> {
  if (running.value) return;
  running.value = true;
  error.value = '';
  try {
    const result = await callTool<BackfillResult>('Schedule', {
      action: 'backfill',
      id: props.scheduleId,
      since: since.value,
      until: until.value || undefined,
    });
    emit('backfilled', result);
    dialogOpen.value = false;
  } catch (backfillError) {
    error.value = backfillError instanceof Error ? backfillError.message : String(backfillError);
  } finally {
    running.value = false;
  }
}
</script>

<template>
  <AlertDialog :open="dialogOpen" @update:open="(value: boolean) => { dialogOpen = value; }">
    <AlertDialogTrigger as-child>
      <Button type="button" variant="outline" size="sm" :disabled="running">
        <Calendar class="h-3.5 w-3.5" />
        Backfill
      </Button>
    </AlertDialogTrigger>
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle>Backfill schedule runs?</AlertDialogTitle>
        <AlertDialogDescription>
          Create placeholder run markers for missed {{ scheduleId }} occurrences. Agents will not be invoked.
        </AlertDialogDescription>
      </AlertDialogHeader>

      <div class="grid gap-3">
        <div class="grid gap-2">
          <Label :for="`${fieldIdBase}-since`">Since</Label>
          <Input :id="`${fieldIdBase}-since`" v-model="since" type="text" required />
        </div>
        <div class="grid gap-2">
          <Label :for="`${fieldIdBase}-until`">Until (optional)</Label>
          <Input :id="`${fieldIdBase}-until`" v-model="until" type="text" />
        </div>
        <p v-if="error" class="text-sm text-destructive" role="alert">{{ error }}</p>
      </div>

      <AlertDialogFooter>
        <AlertDialogCancel :disabled="running">Cancel</AlertDialogCancel>
        <AlertDialogAction :disabled="running || !since" @click.prevent="runBackfill">
          <Loader2 v-if="running" class="h-3.5 w-3.5 animate-spin" />
          Run backfill
        </AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
