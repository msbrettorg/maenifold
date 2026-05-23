<script setup lang="ts">
import { computed, ref } from 'vue';
import { AlertCircle } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { useDaemonHealth } from '../composables/useDaemonHealth';
import { getSharedMcpClient } from '../composables/useSharedMcpClient';
import { useSessionStore } from '../stores/session';

const daemonHealth = useDaemonHealth();
const sessionStore = useSessionStore();
const { daemonOnline, lastErrorMessage } = daemonHealth;
const connectionError = computed(() => sessionStore.connectionError);
const mcpClient = getSharedMcpClient();
const showStartHelp = ref(false);

const message = computed(() => {
  if (connectionError.value) return `Connection issue: ${connectionError.value}`;
  if (!daemonOnline.value) return 'Daemon offline.';
  return '';
});
const isVisible = computed(() => !daemonOnline.value || !!connectionError.value);

async function retry(): Promise<void> {
  await daemonHealth.forceCheck();
  if (connectionError.value) {
    try {
      await mcpClient.reconnectSession();
      sessionStore.clearConnectionError();
    } catch (error) {
      sessionStore.connectionError = error instanceof Error ? error.message : String(error);
    }
  }
}
</script>

<template>
  <div
    v-if="isVisible"
    class="sticky top-0 z-50 flex min-h-10 flex-col gap-2 bg-destructive px-4 py-2 text-destructive-foreground shadow"
    role="alert"
  >
    <div class="flex min-h-6 items-center gap-3">
      <AlertCircle class="h-4 w-4 shrink-0" />
      <p class="min-w-0 flex-1 truncate text-sm font-medium">
        {{ message }}
      </p>
      <span v-if="!daemonOnline && lastErrorMessage" class="hidden truncate text-xs opacity-80 md:inline">
        {{ lastErrorMessage }}
      </span>
      <Button type="button" variant="outline" size="xs" @click="retry">
        Retry
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="xs"
        :aria-expanded="showStartHelp"
        @click="showStartHelp = !showStartHelp"
      >
        How to start
      </Button>
    </div>
    <div v-if="showStartHelp" class="rounded-md bg-bg-app/20 px-3 py-2 text-xs">
      Start the daemon by running
      <code class="rounded bg-bg-app/30 px-1.5 py-0.5 font-mono">maenifold --http</code>
      in your terminal.
    </div>
  </div>
</template>
