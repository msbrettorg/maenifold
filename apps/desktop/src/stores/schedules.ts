import { defineStore } from 'pinia';

interface ScheduleRun {
  status: string;
  timestamp: string;
  error?: string | null;
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

export interface ScheduleInstallStatus {
  id: string;
  installed: boolean;
  enabledInConfig: boolean;
  discrepancy: boolean;
  nextFireTime?: string;
  lastFireResult?: {
    exitCode: number;
    finishedAt: string;
  };
}

export interface PauseStateView {
  paused: boolean;
  pausedAt?: string;
  schedulesPaused?: number;
}

type BadgeVariant = 'default' | 'secondary' | 'destructive' | 'outline';

export const useSchedulesStore = defineStore('schedules', {
  state: () => ({
    installStatuses: {} as Record<string, ScheduleInstallStatus>,
    pauseState: { paused: false } as PauseStateView,
  }),
  getters: {
    isPausedAll: (state): boolean => state.pauseState.paused,
    installStatusFor: (state) => (job: ScheduleJob): ScheduleInstallStatus | undefined =>
      state.installStatuses[job.id],
    installBadgeFor: (state) => (job: ScheduleJob): { label: string; variant: BadgeVariant; class?: string } => {
      const status = state.installStatuses[job.id];
      const installed = status?.installed ?? false;
      const enabledInConfig = status?.enabledInConfig ?? job.enabled;
      if (status?.discrepancy) {
        return { label: 'Discrepancy', variant: 'outline', class: 'border-warning/40 bg-warning/10 text-warning' };
      }
      if (installed && enabledInConfig) {
        return { label: 'Installed', variant: 'default', class: 'bg-success text-primary-foreground' };
      }
      if (!installed && enabledInConfig) {
        return { label: 'Missing', variant: 'outline', class: 'border-warning/40 bg-warning/10 text-warning' };
      }
      return { label: 'Paused', variant: 'secondary' };
    },
  },
  actions: {
    setInstallStatuses(payload: unknown): void {
      const statuses = Array.isArray(payload) ? payload as ScheduleInstallStatus[] : [];
      this.installStatuses = Object.fromEntries(statuses.map((status) => [status.id, status]));
    },
    setPauseState(payload: unknown): void {
      const value = typeof payload === 'object' && payload !== null ? payload as Record<string, unknown> : {};
      const next: PauseStateView = { paused: value.paused === true };
      if (typeof value.pausedAt === 'string') next.pausedAt = value.pausedAt;
      if (typeof value.schedulesPaused === 'number') next.schedulesPaused = value.schedulesPaused;
      this.pauseState = next;
    },
  },
});
