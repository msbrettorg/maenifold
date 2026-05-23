import type { Ref } from 'vue';
import type { ScheduleForm } from '@/composables/useSchedulesSettings';
import type { McpParametersParseResult } from '@/composables/schedules/useSchedulesValidation';
import { parseAllowedTools } from '@/composables/schedules/useSchedulesValidation';
import type { ScheduleJob } from '@/stores/schedules';

type SchedulePlatform = {
  supportsEnabledSchedules: boolean;
};

type ScheduleGatewayExecute = <T = unknown>(
  domain: string,
  command: string,
  parameters: Record<string, unknown>,
) => Promise<T>;

type SchedulesConfigStore = {
  loading: boolean;
  loadConfig: () => Promise<void>;
};

type SchedulesStore = {
  setInstallStatuses: (payload: unknown) => void;
  setPauseState: (payload: unknown) => void;
};

export function createSchedulesCrud(args: {
  configStore: SchedulesConfigStore;
  agentNames: Ref<string[]>;
  ensureConnected: () => Promise<void>;
  gatewayExecute: ScheduleGatewayExecute;
  schedulesStore: SchedulesStore;
  jobs: Ref<ScheduleJob[]>;
  isLoading: Ref<boolean>;
  isSubmitting: Ref<boolean>;
  error: Ref<string>;
  statusMessage: Ref<string>;
  dialogOpen: Ref<boolean>;
  dialogMode: Ref<'create' | 'edit'>;
  previewIsValid: Ref<boolean>;
  form: ScheduleForm;
  schedulePlatform: SchedulePlatform;
  canSubmit: Ref<boolean>;
  parseCurrentMcpParameters: () => McpParametersParseResult;
}) {
  const {
    configStore,
    agentNames,
    ensureConnected,
    gatewayExecute,
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
  } = args;

  function resetForm(): void {
    form.id = '';
    form.schedule = '';
    form.agent = agentNames.value[0] ?? '';
    form.prompt = '';
    form.description = '';
    form.enabled = schedulePlatform.supportsEnabledSchedules;
    form.autoGrantPermissions = false;
    form.allowedTools = '';
    form.mcpAction = '';
    form.mcpParameters = '';
    previewIsValid.value = false;
  }

  function ensureDialogAgent(): void {
    if (!form.agent && agentNames.value.length > 0) form.agent = agentNames.value[0];
  }

  function normalizeJobs(payload: unknown): ScheduleJob[] {
    return Array.isArray(payload) ? payload as ScheduleJob[] : [];
  }

  async function loadSchedules(): Promise<void> {
    isLoading.value = true;
    error.value = '';
    try {
      if (!configStore.loading && agentNames.value.length === 0) await configStore.loadConfig();
      await ensureConnected();
      jobs.value = normalizeJobs(await gatewayExecute('maenifold.system', 'Schedule', { action: 'list' }));
      try {
        schedulesStore.setInstallStatuses(await gatewayExecute('maenifold.system', 'Schedule', { action: 'verify_installed' }));
      } catch (probeError) {
        console.error('Schedule verify_installed probe failed', probeError);
        schedulesStore.setInstallStatuses([]);
      }
      schedulesStore.setPauseState(await gatewayExecute('maenifold.system', 'Schedule', { action: 'pause_status' }));
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
    form.mcpAction = job.mcp_action ?? '';
    form.mcpParameters = job.mcp_parameters == null ? '' : JSON.stringify(job.mcp_parameters, null, 2);
    ensureDialogAgent();
    previewIsValid.value = false;
    dialogOpen.value = true;
  }

  async function saveSchedule(): Promise<void> {
    const parsedMcpParameters = parseCurrentMcpParameters();
    if (form.mcpAction.trim() && !parsedMcpParameters.ok) {
      error.value = 'Parameters must be a JSON object.';
      return;
    }
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
      const mcpAction = form.mcpAction.trim();
      if (mcpAction) {
        payload.mcp_action = mcpAction;
        payload.mcp_parameters = parsedMcpParameters.ok ? parsedMcpParameters.value : {};
      }
      const result = await gatewayExecute<{ message?: string }>('maenifold.system', 'Schedule', payload);
      statusMessage.value = result?.message ?? `Schedule ${dialogMode.value === 'create' ? 'created' : 'updated'}.`;
      dialogOpen.value = false;
      await loadSchedules();
    } catch (saveError) {
      error.value = saveError instanceof Error ? saveError.message : String(saveError);
    } finally {
      isSubmitting.value = false;
    }
  }

  return {
    resetForm,
    ensureDialogAgent,
    loadSchedules,
    openCreateDialog,
    openEditDialog,
    saveSchedule,
  };
}
