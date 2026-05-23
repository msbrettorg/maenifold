import type { Ref } from 'vue';
import type { ScheduleJob } from '@/stores/schedules';

type ScheduleToolCaller = <T = unknown>(tool: string, payload: Record<string, unknown>) => Promise<T>;

type ConfirmDialogStore = {
  confirm: (title: string, description: string) => Promise<boolean>;
};

export function createSchedulesLifecycle(args: {
  ensureConnected: () => Promise<void>;
  callTool: ScheduleToolCaller;
  confirmDialogStore: ConfirmDialogStore;
  actionJobIds: Ref<Set<string>>;
  isSubmitting: Ref<boolean>;
  error: Ref<string>;
  statusMessage: Ref<string>;
  loadSchedules: () => Promise<void>;
}) {
  const {
    ensureConnected,
    callTool,
    confirmDialogStore,
    actionJobIds,
    isSubmitting,
    error,
    statusMessage,
    loadSchedules,
  } = args;

  async function pauseAll(): Promise<void> {
    isSubmitting.value = true;
    error.value = '';
    statusMessage.value = '';
    try {
      await ensureConnected();
      const result = await callTool<{ message?: string }>('Schedule', { action: 'pause_all' });
      statusMessage.value = result?.message ?? 'Schedules paused.';
      await loadSchedules();
    } catch (pauseError) {
      error.value = pauseError instanceof Error ? pauseError.message : String(pauseError);
    } finally {
      isSubmitting.value = false;
    }
  }

  async function resumeAll(): Promise<void> {
    isSubmitting.value = true;
    error.value = '';
    statusMessage.value = '';
    try {
      await ensureConnected();
      const result = await callTool<{ message?: string }>('Schedule', { action: 'resume_all' });
      statusMessage.value = result?.message ?? 'Schedules resumed.';
      await loadSchedules();
    } catch (resumeError) {
      error.value = resumeError instanceof Error ? resumeError.message : String(resumeError);
    } finally {
      isSubmitting.value = false;
    }
  }

  async function setPauseAll(value: boolean): Promise<void> {
    if (value) await pauseAll();
    else await resumeAll();
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

  async function repairSchedule(job: ScheduleJob): Promise<void> {
    actionJobIds.value.add(job.id);
    error.value = '';
    statusMessage.value = '';
    try {
      await ensureConnected();
      const result = await callTool<{ message?: string }>('Schedule', {
        action: 'update',
        id: job.id,
        schedule: job.schedule,
        agent: job.agent,
        prompt: job.prompt,
        description: job.description ?? job.id,
        enabled: job.enabled,
        auto_grant_permissions: job.auto_grant_permissions,
        allowed_tools: job.allowed_tools,
      });
      statusMessage.value = result?.message ?? `Scheduled task '${job.id}' repaired.`;
      await loadSchedules();
    } catch (repairError) {
      error.value = repairError instanceof Error ? repairError.message : String(repairError);
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

  return {
    pauseAll,
    resumeAll,
    setPauseAll,
    runSchedule,
    repairSchedule,
    deleteSchedule,
  };
}
