import { computed, onMounted, reactive, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { useMcpClient } from '@/composables/useMcpClient';
import { allowedToolsLabel, parseAllowedTools, permissionLabel, permissionVariant } from '@/composables/useSchedulePermissions';
import { detectSchedulePlatform } from '@/composables/useSchedulePlatform';
import { useConfigStore } from '@/stores/config';
import { useConfirmDialogStore } from '@/stores/confirm-dialog';

interface ScheduleRun {
  status: string;
  timestamp: string;
  error?: string | null;
}

export interface ScheduleForm {
  id: string;
  schedule: string;
  agent: string;
  prompt: string;
  description: string;
  enabled: boolean;
  autoGrantPermissions: boolean;
  allowedTools: string;
}

export interface ScheduleJob {
  id: string;
  description?: string;
  schedule: string;
  schedule_human?: string | null;
  agent: string;
  prompt: string;
  enabled: boolean;
  auto_grant_permissions: boolean;
  allowed_tools: string[];
  last_run?: ScheduleRun | null;
}

type DialogMode = 'create' | 'edit';

const timestampFormatter = new Intl.DateTimeFormat(undefined, {
  dateStyle: 'medium',
  timeStyle: 'short',
});

export function useSchedulesSettings() {
  const configStore = useConfigStore();
  const confirmDialogStore = useConfirmDialogStore();
  const { agentNames } = storeToRefs(configStore);
  const { callTool, connect, connected } = useMcpClient();
  const jobs = ref<ScheduleJob[]>([]);
  const isLoading = ref(true);
  const isSubmitting = ref(false);
  const actionJobIds = ref(new Set<string>());
  const error = ref('');
  const statusMessage = ref('');
  const dialogOpen = ref(false);
  const dialogMode = ref<DialogMode>('create');
  const previewIsValid = ref(false);
  const schedulePlatform = detectSchedulePlatform();
  const form = reactive<ScheduleForm>({
    id: '',
    schedule: '',
    agent: '',
    prompt: '',
    description: '',
    enabled: true,
    autoGrantPermissions: false,
    allowedTools: '',
  });

  const dialogTitle = computed(() => dialogMode.value === 'create' ? 'Create schedule' : 'Edit schedule');
  const dialogDescription = computed(() => dialogMode.value === 'create'
    ? 'Create a recurring agent task using a cron expression or natural language schedule.'
    : 'Update the agent, cadence, and prompt for this recurring task.');
  const canSubmit = computed(() =>
    !!form.id.trim()
    && !!form.schedule.trim()
    && !!form.agent
    && !!form.prompt.trim()
    && !isSubmitting.value
    && agentNames.value.length > 0
    && previewIsValid.value
    && (!form.enabled || schedulePlatform.supportsEnabledSchedules),
  );

  function resetForm(): void {
    form.id = '';
    form.schedule = '';
    form.agent = agentNames.value[0] ?? '';
    form.prompt = '';
    form.description = '';
    form.enabled = schedulePlatform.supportsEnabledSchedules;
    form.autoGrantPermissions = false;
    form.allowedTools = '';
    previewIsValid.value = false;
  }

  function ensureDialogAgent(): void {
    if (!form.agent && agentNames.value.length > 0) form.agent = agentNames.value[0];
  }

  function normalizeJobs(payload: unknown): ScheduleJob[] {
    return Array.isArray(payload) ? payload as ScheduleJob[] : [];
  }

  function scheduleLabel(job: ScheduleJob): string {
    return job.schedule_human?.trim() || job.schedule;
  }

  function statusFor(job: ScheduleJob): { label: string; variant: 'default' | 'secondary' | 'destructive' } {
    if (!job.enabled) return { label: 'Paused', variant: 'secondary' };
    if (job.last_run?.status === 'failed' || job.last_run?.status === 'timeout') {
      return { label: 'Failed', variant: 'destructive' };
    }
    return { label: 'Active', variant: 'default' };
  }

  function formatLastRun(timestamp?: string | null): string {
    if (!timestamp) return 'Never';
    const parsed = new Date(timestamp);
    return Number.isNaN(parsed.getTime()) ? timestamp : timestampFormatter.format(parsed);
  }

  async function ensureConnected(): Promise<void> {
    if (!connected.value) await connect();
  }

  async function loadSchedules(): Promise<void> {
    isLoading.value = true;
    error.value = '';
    try {
      if (!configStore.loading && agentNames.value.length === 0) await configStore.loadConfig();
      await ensureConnected();
      jobs.value = normalizeJobs(await callTool('Schedule', { action: 'list' }));
      ensureDialogAgent();
    } catch (loadError) {
      error.value = loadError instanceof Error ? loadError.message : String(loadError);
    } finally {
      isLoading.value = false;
    }
  }

  function openCreateDialog(): void {
    dialogMode.value = 'create';
    resetForm();
    ensureDialogAgent();
    previewIsValid.value = false;
    dialogOpen.value = true;
  }

  function openEditDialog(job: ScheduleJob): void {
    dialogMode.value = 'edit';
    form.id = job.id;
    form.schedule = job.schedule || job.schedule_human || '';
    form.agent = job.agent;
    form.prompt = job.prompt;
    form.description = job.description && job.description !== job.id ? job.description : '';
    form.enabled = schedulePlatform.supportsEnabledSchedules ? job.enabled : false;
    form.autoGrantPermissions = Boolean(job.auto_grant_permissions);
    form.allowedTools = job.allowed_tools?.join(', ') ?? '';
    ensureDialogAgent();
    previewIsValid.value = false;
    dialogOpen.value = true;
  }

  function isActionInProgress(id: string): boolean {
    return actionJobIds.value.has(id);
  }

  async function saveSchedule(): Promise<void> {
    if (!canSubmit.value) return;
    isSubmitting.value = true;
    error.value = '';
    statusMessage.value = '';
    try {
      await ensureConnected();
      const payload: Record<string, unknown> = {
        action: dialogMode.value === 'create' ? 'create' : 'update',
        id: form.id.trim(),
        schedule: form.schedule.trim(),
        agent: form.agent,
        prompt: form.prompt.trim(),
        enabled: form.enabled,
        auto_grant_permissions: form.autoGrantPermissions,
        allowed_tools: parseAllowedTools(form.allowedTools),
      };
      const description = form.description.trim();
      if (description) payload.description = description;
      const result = await callTool<{ message?: string }>('Schedule', payload);
      statusMessage.value = result?.message ?? `Schedule ${dialogMode.value === 'create' ? 'created' : 'updated'}.`;
      dialogOpen.value = false;
      await loadSchedules();
    } catch (saveError) {
      error.value = saveError instanceof Error ? saveError.message : String(saveError);
    } finally {
      isSubmitting.value = false;
    }
  }

  async function runSchedule(job: ScheduleJob): Promise<void> {
    actionJobIds.value.add(job.id);
    error.value = '';
    statusMessage.value = '';
    try {
      await ensureConnected();
      const result = await callTool<{ message?: string }>('Schedule', { action: 'run_now', id: job.id });
      statusMessage.value = result?.message ?? `Scheduled task '${job.id}' executed.`;
      await loadSchedules();
    } catch (runError) {
      error.value = runError instanceof Error ? runError.message : String(runError);
    } finally {
      actionJobIds.value.delete(job.id);
    }
  }

  async function deleteSchedule(job: ScheduleJob): Promise<void> {
    const confirmed = await confirmDialogStore.confirm(
      `Delete schedule "${job.id}"?`,
      `This will permanently delete the "${job.id}" schedule. This action cannot be undone.`,
    );
    if (!confirmed) return;

    actionJobIds.value.add(job.id);
    error.value = '';
    statusMessage.value = '';
    try {
      await ensureConnected();
      const result = await callTool<{ message?: string }>('Schedule', { action: 'delete', id: job.id });
      statusMessage.value = result?.message ?? `Scheduled task '${job.id}' deleted.`;
      await loadSchedules();
    } catch (deleteError) {
      error.value = deleteError instanceof Error ? deleteError.message : String(deleteError);
    } finally {
      actionJobIds.value.delete(job.id);
    }
  }

  onMounted(() => {
    resetForm();
    void loadSchedules();
  });

  return {
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
    previewIsValid,
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
  };
}
