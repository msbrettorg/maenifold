<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { CalendarClock } from '@lucide/vue';
import { useAppShellStore } from '../stores/appShell';
import { useSchedulesStore, type ScheduleJob } from '../stores/schedules';

const shell = useAppShellStore();
const schedulesStore = useSchedulesStore();
const { jobs, loading, error } = storeToRefs(schedulesStore);

const isSchedulesTabActive = computed(() => shell.activeTab === 'schedules');

function readString(value: unknown): string | null {
  return typeof value === 'string' && value.trim().length > 0 ? value : null;
}

function jobKey(job: ScheduleJob, index: number): string {
  return readString(job.id) ?? readString(job.name) ?? `schedule-job-${index}`;
}

function jobName(job: ScheduleJob): string {
  return readString(job.name) ?? readString(job.id) ?? 'Unnamed job';
}

function jobIdentity(job: ScheduleJob): string | null {
  const id = readString(job.id);
  const name = readString(job.name);
  return id && id !== name ? id : null;
}

function jobSchedule(job: ScheduleJob): string {
  return readString(job.schedule) ?? readString(job.cron) ?? '—';
}

function jobNextRun(job: ScheduleJob): string {
  return readString(job.nextRun) ?? readString(job.next_run) ?? '—';
}

function jobStatus(job: ScheduleJob): string {
  if (readString(job.status)) {
    return readString(job.status) as string;
  }

  if (typeof job.enabled === 'boolean') {
    return job.enabled ? 'enabled' : 'disabled';
  }

  return 'unknown';
}

function syncPolling(active: boolean): void {
  if (active) {
    void schedulesStore.loadJobs();
    schedulesStore.startPolling();
    return;
  }

  schedulesStore.stopPolling();
}

onMounted(() => {
  void schedulesStore.loadJobs();
  schedulesStore.startPolling();
});

watch(isSchedulesTabActive, (active) => {
  syncPolling(active);
});

onBeforeUnmount(() => {
  schedulesStore.stopPolling();
});
</script>

<template>
  <div class="schedules">
    <div class="schedules-inner">
      <section class="schedules-hero">
        <div class="schedules-icon-wrap">
          <CalendarClock :size="34" :stroke-width="1.5" />
        </div>
        <div class="schedules-copy">
          <h1 class="schedules-title">Schedules</h1>
          <p class="schedules-description">
            Read-only scheduled jobs loaded from the daemon over MCP resources.
          </p>
        </div>
      </section>

      <p v-if="error" class="schedules-banner" role="status">
        {{ error }}
      </p>

      <section class="schedules-card">
        <div class="schedules-card-header">
          <div>
            <h2 class="schedules-card-title">Scheduled jobs</h2>
            <p class="schedules-card-subtitle">Polling every 4 seconds while this tab is active.</p>
          </div>
          <span class="schedules-count">{{ jobs.length }}</span>
        </div>

        <div v-if="loading && jobs.length === 0" class="schedules-state">
          Loading scheduled jobs…
        </div>
        <div v-else-if="jobs.length === 0" class="schedules-state">
          No scheduled jobs
        </div>
        <div v-else class="schedules-table-wrap">
          <table class="schedules-table">
            <thead>
              <tr>
                <th scope="col">Job</th>
                <th scope="col">Schedule</th>
                <th scope="col">Next run</th>
                <th scope="col">Status</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(job, index) in jobs" :key="jobKey(job, index)">
                <td>
                  <div class="job-primary">{{ jobName(job) }}</div>
                  <div v-if="jobIdentity(job)" class="job-secondary">{{ jobIdentity(job) }}</div>
                </td>
                <td>{{ jobSchedule(job) }}</td>
                <td>{{ jobNextRun(job) }}</td>
                <td>
                  <span class="status-pill">{{ jobStatus(job) }}</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.schedules {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 2rem;
  background:
    radial-gradient(70% 50% at 50% 0%, color-mix(in srgb, var(--text-accent) 9%, transparent) 0%, transparent 70%),
    var(--bg-main);
}

.schedules-inner {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: 100%;
  max-width: 980px;
  margin: 0 auto;
}

.schedules-hero {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1.25rem 1.35rem;
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: color-mix(in srgb, var(--bg-sidebar) 70%, var(--bg-main));
}

.schedules-icon-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 64px;
  height: 64px;
  border-radius: 18px;
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--text-accent) 25%, var(--border-color));
  box-shadow: 0 10px 30px color-mix(in srgb, var(--text-accent) 16%, transparent);
}

.schedules-copy {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.schedules-title {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 600;
  letter-spacing: -0.02em;
  color: var(--text-primary);
}

.schedules-description {
  margin: 0;
  color: var(--text-secondary);
  line-height: 1.5;
}

.schedules-banner {
  margin: 0;
  padding: 0.85rem 1rem;
  border-radius: 12px;
  border: 1px solid color-mix(in srgb, var(--bg-warning) 45%, var(--border-color));
  background: color-mix(in srgb, var(--bg-warning) 10%, var(--bg-main));
  color: var(--text-primary);
}

.schedules-card {
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: var(--bg-main);
  overflow: hidden;
}

.schedules-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 1rem 1.15rem;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 55%, var(--bg-main));
}

.schedules-card-title {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text-primary);
}

.schedules-card-subtitle {
  margin: 0.2rem 0 0;
  font-size: 0.78rem;
  color: var(--text-muted);
}

.schedules-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 2rem;
  height: 2rem;
  padding: 0 0.6rem;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--bg-sidebar);
  color: var(--text-primary);
  font-size: 0.85rem;
  font-weight: 600;
}

.schedules-state {
  padding: 2.5rem 1rem;
  text-align: center;
  color: var(--text-secondary);
}

.schedules-table-wrap {
  overflow-x: auto;
}

.schedules-table {
  width: 100%;
  border-collapse: collapse;
}

.schedules-table th,
.schedules-table td {
  padding: 0.9rem 1.15rem;
  text-align: left;
  border-bottom: 1px solid var(--border-color);
  vertical-align: top;
}

.schedules-table th {
  font-size: 0.75rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--text-muted);
  background: color-mix(in srgb, var(--bg-sidebar) 35%, var(--bg-main));
}

.schedules-table td {
  font-size: 0.9rem;
  color: var(--text-primary);
}

.schedules-table tbody tr:hover {
  background: var(--bg-hover);
}

.schedules-table tbody tr:last-child td {
  border-bottom: none;
}

.job-primary {
  font-weight: 600;
  color: var(--text-primary);
}

.job-secondary {
  margin-top: 0.2rem;
  font-size: 0.76rem;
  color: var(--text-muted);
}

.status-pill {
  display: inline-flex;
  align-items: center;
  padding: 0.2rem 0.55rem;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--bg-sidebar);
  color: var(--text-secondary);
  font-size: 0.78rem;
  line-height: 1.3;
}

@media (max-width: 720px) {
  .schedules {
    padding: 1rem;
  }

  .schedules-hero {
    align-items: flex-start;
  }

  .schedules-card-header {
    align-items: flex-start;
    flex-direction: column;
  }
}
</style>
