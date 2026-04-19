<script setup lang="ts">
import { Button } from '@/components/ui/button';

defineProps<{
  busyAction: string | null;
  canConnectLive: boolean;
  canPromote: boolean;
  canReconnect: boolean;
  forkedSessionId: string | null;
  graftSubmitting: boolean;
  hasResults: boolean;
  promotionBusy: boolean;
  reconnectBusy: boolean;
  reconnectSessionId: string | null;
  viewNodeStatus: string;
}>();

defineEmits<{
  abandon: [];
  fork: [];
  graft: [];
  promote: [];
  reconnect: [];
  restart: [];
  rewind: [];
}>();
</script>

<template>
  <footer class="zoom-footer">
    <div class="zoom-actions">
      <Button
        variant="ghost"
        :disabled="busyAction !== null"
        type="button"
        @click="$emit('fork')"
      >
        {{ busyAction === 'fork' ? 'Forking...' : 'Fork from node' }}
      </Button>
      <Button
        v-if="['abandoned', 'stuck'].includes(viewNodeStatus)"
        :disabled="busyAction !== null"
        type="button"
        @click="$emit('restart')"
      >
        {{ busyAction === 'restart' ? 'Restarting...' : 'Restart node' }}
      </Button>
      <Button
        v-if="['completed', 'abandoned', 'stuck'].includes(viewNodeStatus)"
        variant="ghost"
        :disabled="busyAction !== null"
        type="button"
        @click="$emit('rewind')"
      >
        {{ busyAction === 'rewind' ? 'Rewinding...' : 'Rewind node' }}
      </Button>
      <Button
        v-if="viewNodeStatus === 'active'"
        variant="destructive"
        :disabled="busyAction !== null"
        type="button"
        @click="$emit('abandon')"
      >
        {{ busyAction === 'abandon' ? 'Abandoning...' : 'Abandon node' }}
      </Button>
      <Button
        v-if="hasResults"
        variant="ghost"
        :disabled="busyAction !== null || graftSubmitting"
        type="button"
        @click="$emit('graft')"
      >
        Graft from node
      </Button>
      <Button
        v-if="canPromote"
        :disabled="promotionBusy || busyAction !== null"
        :title="promotionBusy ? 'Promotion in progress…' : 'Save this completed output to memory'"
        type="button"
        @click="$emit('promote')"
      >
        {{ promotionBusy ? 'Promoting…' : 'Promote to Memory' }}
      </Button>
      <button
        v-if="canReconnect && reconnectSessionId"
        class="zoom-reconnect-btn"
        :disabled="reconnectBusy || busyAction !== null"
        type="button"
        @click="$emit('reconnect')"
      >
        {{ reconnectBusy ? 'Reconnecting…' : 'Reconnect' }}
      </button>
      <button
        v-if="canConnectLive"
        class="zoom-reconnect-btn"
        :disabled="reconnectBusy || busyAction !== null"
        type="button"
        @click="$emit('reconnect')"
      >
        {{ reconnectBusy ? 'Connecting…' : 'Connect to live session' }}
      </button>
    </div>

    <div v-if="forkedSessionId" class="zoom-fork-result" role="status">
      <span>Forked session</span>
      <strong class="zoom-mono">{{ forkedSessionId }}</strong>
    </div>
  </footer>
</template>

<style scoped>
.zoom-footer {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4) var(--space-4);
  border-top: 1px solid var(--border);
  background: color-mix(in srgb, var(--accent, var(--surface-highest)) 55%, var(--card, var(--surface-container)));
}

.zoom-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
}

.zoom-fork-result {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.75rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--background);
}

.zoom-fork-result span {
  color: var(--muted-foreground);
  font-size: var(--text-caption);
}

.zoom-mono {
  font-family: var(--font-code);
  word-break: break-all;
}

.zoom-reconnect-btn {
  padding: 0.5rem 1rem;
  border: none;
  border-radius: 4px;
  background: var(--blue);
  color: var(--primary-foreground);
  font-weight: 600;
  font-size: var(--text-caption);
  cursor: pointer;
  transition: opacity 0.15s ease;
}

.zoom-reconnect-btn:hover:not(:disabled) {
  opacity: 0.88;
}

.zoom-reconnect-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
