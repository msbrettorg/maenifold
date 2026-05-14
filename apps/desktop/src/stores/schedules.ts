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

type BadgeVariant = 'default' | 'secondary' | 'destructive' | 'outline';

export const useSchedulesStore = defineStore('schedules', {
  state: () => ({
    installStatuses: {} as Record<string, ScheduleInstallStatus>,
  }),
  getters: {
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
  },
});
