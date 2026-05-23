import { computed, onMounted, reactive, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { useMcpClient } from '@/composables/useMcpClient';
import { detectSchedulePlatform } from '@/composables/useSchedulePlatform';
import {
  allowedToolsLabel,
  formatLastRun,
  formatPausedAt,
  permissionLabel,
  permissionVariant,
  scheduleLabel,
  statusFor,
} from '@/composables/schedules/useSchedulesFormatting';
import { createSchedulesCrud } from '@/composables/schedules/useSchedulesCrud';
import { createSchedulesLifecycle } from '@/composables/schedules/useSchedulesLifecycle';
import { useSchedulesValidation } from '@/composables/schedules/useSchedulesValidation';
import { useConfigStore } from '@/stores/config';
import { useConfirmDialogStore } from '@/stores/confirm-dialog';
import { useSchedulesStore, type ScheduleJob } from '@/stores/schedules';

export interface ScheduleForm {
  id: string;
  schedule: string;
  agent: string;
  prompt: string;
  description: string;
  enabled: boolean;
  autoGrantPermissions: boolean;
  allowedTools: string;
  mcpAction: string;
  mcpParameters: string;
}

type DialogMode = 'create' | 'edit';

export type { PermissionBadgeVariant } from '@/composables/schedules/useSchedulesFormatting';

export function useSchedulesSettings() {
  const configStore = useConfigStore();
  const confirmDialogStore = useConfirmDialogStore();
  const schedulesStore = useSchedulesStore();
  const { agentNames } = storeToRefs(configStore);
  const { pauseState, isPausedAll } = storeToRefs(schedulesStore);
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
    mcpAction: '',
    mcpParameters: '',
  });

  const dialogTitle = computed(() => dialogMode.value === 'create' ? 'Create schedule' : 'Edit schedule');
  const dialogDescription = computed(() => dialogMode.value === 'create'
    ? 'Create a recurring agent task using a cron expression or natural language schedule.'
    : 'Update the agent, cadence, and prompt for this recurring task.');
  const { mcpActionSet, mcpParametersValid, parseCurrentMcpParameters } = useSchedulesValidation(form);
  const canSubmit = computed(() => {
    const hasRunnableTarget = mcpActionSet.value || !!form.prompt.trim();
    return !!form.id.trim()
      && !!form.schedule.trim()
      && !!form.agent
      && hasRunnableTarget
      && (!mcpActionSet.value || mcpParametersValid.value)
      && !isSubmitting.value
      && agentNames.value.length > 0
      && previewIsValid.value
      && (!form.enabled || schedulePlatform.supportsEnabledSchedules);
  });

  async function ensureConnected(): Promise<void> {
    if (!connected.value) await connect();
  }

  function isActionInProgress(id: string): boolean {
    return actionJobIds.value.has(id);
  }
  const { resetForm, loadSchedules, openCreateDialog, openEditDialog, saveSchedule } = createSchedulesCrud({
    configStore,
    agentNames,
    ensureConnected,
    callTool,
    schedulesStore,
    jobs,
    isLoading,
    isSubmitting,
    error,
    statusMessage,
    dialogOpen,
    dialogMode,
    previewIsValid,
    form,
    schedulePlatform,
    canSubmit,
    parseCurrentMcpParameters,
  });
  const { pauseAll, resumeAll, setPauseAll, runSchedule, repairSchedule, deleteSchedule } = createSchedulesLifecycle({
    ensureConnected,
    callTool,
    confirmDialogStore,
    actionJobIds,
    isSubmitting,
    error,
    statusMessage,
    loadSchedules,
  });

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
    installStatusFor: schedulesStore.installStatusFor,
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
    pauseState,
    isPausedAll,
    scheduleLabel,
    statusFor,
    installBadgeFor: schedulesStore.installBadgeFor,
    permissionLabel,
    permissionVariant,
    allowedToolsLabel,
    formatLastRun,
    formatPausedAt,
    loadSchedules,
    openCreateDialog,
    openEditDialog,
    saveSchedule,
    pauseAll,
    resumeAll,
    setPauseAll,
    runSchedule,
    repairSchedule,
    deleteSchedule,
  };
}
