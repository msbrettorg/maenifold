<script setup lang="ts">
// Teams tab is display/replay only. It MUST NOT call sessionStore.createSession
// or otherwise mutate activeSessionId — the Teams Monitor ACP session is owned
// by the backend (see crates/maenifold-core/src/teams/supervisor.rs,
// TEAMS_SESSION_ID = "teams-monitor"). Configure it in Settings → Teams.
// A read-only replay view will land in a follow-up task; until then this tab
// renders a status placeholder so it cannot leak into the Agents tab.
import { computed, onMounted, ref } from 'vue';
import { useMcpClient } from '@/composables/useMcpClient';
import {
  parseTeamsStatusPayload,
  formatTeamsConnectionState,
  type TeamsMonitorStatus,
  type TeamsStatus,
} from '@/composables/useTeamsStatus';

const { callTool, connect, connected } = useMcpClient();
const status = ref<TeamsStatus>(parseTeamsStatusPayload({}));
const error = ref('');

const headline = computed(() =>
  status.value.enabled
    ? `Teams monitor active · ${formatTeamsConnectionState(status.value.connectionState)}`
    : 'Teams monitor is disabled. Enable it in Settings → Teams.',
);
const detail = computed(
  () => `${status.value.matchCount} matches · ${status.value.activeDispatches} active dispatches`,
);

async function loadStatus(): Promise<void> {
  try {
    if (!connected.value) await connect();
    const payload = await callTool<TeamsMonitorStatus>('TeamsMonitor', { action: 'status' });
    status.value = parseTeamsStatusPayload(payload);
  } catch (loadError) {
    error.value = loadError instanceof Error ? loadError.message : String(loadError);
  }
}

onMounted(() => { void loadStatus(); });
</script>

<template>
  <div class="flex flex-1 flex-col items-center justify-center gap-2 p-8 text-center">
    <p class="text-sm font-semibold text-text-primary">{{ headline }}</p>
    <p class="text-xs text-text-secondary">{{ detail }}</p>
    <p v-if="error" class="text-xs text-destructive">{{ error }}</p>
  </div>
</template>
