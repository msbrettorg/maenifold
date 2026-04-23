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
  <!-- audit-ok: dynamic-binding (color-mix blend of accent fallback chain not pre-baked as a token) -->
  <footer
    class="flex flex-col gap-[var(--space-3)] py-[var(--space-3)] px-[var(--space-4)] pb-[var(--space-4)] border-t border-[var(--color-border-subtle)]"
    style="background: color-mix(in srgb, var(--accent, var(--surface-highest)) 55%, var(--card, var(--surface-container)));"
  >
    <div class="flex flex-wrap gap-3">
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
      <Button
        v-if="canReconnect && reconnectSessionId"
        variant="outline"
        :disabled="reconnectBusy || busyAction !== null"
        type="button"
        @click="$emit('reconnect')"
      >
        {{ reconnectBusy ? 'Reconnecting…' : 'Reconnect' }}
      </Button>
      <Button
        v-if="canConnectLive"
        variant="outline"
        :disabled="reconnectBusy || busyAction !== null"
        type="button"
        @click="$emit('reconnect')"
      >
        {{ reconnectBusy ? 'Connecting…' : 'Connect to live session' }}
      </Button>
    </div>

    <div
      v-if="forkedSessionId"
      class="flex flex-col gap-1 p-3 border border-[var(--color-border-subtle)] rounded-[var(--radius)] bg-[var(--color-bg-app)]"
      role="status"
    >
      <span class="text-text-secondary text-[length:var(--text-caption)]">Forked session</span>
      <strong class="font-mono break-all">{{ forkedSessionId }}</strong>
    </div>
  </footer>
</template>
