import { ref } from 'vue';
import { defineStore } from 'pinia';
import { mcpHttpClient } from '../lib/mcp/client';

const POLL_INTERVAL_MS = 4000;

export interface ScheduleJob {
  id?: string;
  name?: string;
  schedule?: string;
  cron?: string;
  nextRun?: string;
  next_run?: string;
  status?: string;
  enabled?: boolean;
  [key: string]: unknown;
}

export const useSchedulesStore = defineStore('schedules', () => {
  const jobs = ref<ScheduleJob[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let inFlight: Promise<void> | null = null;

  async function loadJobs(): Promise<void> {
    if (inFlight) {
      return inFlight;
    }

    inFlight = (async () => {
      loading.value = true;

      try {
        const result = await mcpHttpClient.readResource<unknown>('schedule://jobs');
        if (!Array.isArray(result)) {
          throw new Error("MCP resource 'schedule://jobs' did not return a job array");
        }

        jobs.value = result as ScheduleJob[];
        error.value = null;
      } catch (err) {
        error.value = err instanceof Error ? err.message : String(err);
      } finally {
        loading.value = false;
        inFlight = null;
      }
    })();

    return inFlight;
  }

  function startPolling(): void {
    if (pollTimer) return;

    pollTimer = setInterval(() => {
      void loadJobs();
    }, POLL_INTERVAL_MS);
  }

  function stopPolling(): void {
    if (!pollTimer) return;

    clearInterval(pollTimer);
    pollTimer = null;
  }

  return {
    jobs,
    loading,
    error,
    loadJobs,
    startPolling,
    stopPolling,
  };
});
