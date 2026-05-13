<script setup lang="ts">
import { Button } from '@/components/ui/button';

defineProps<{
  busyAction: string | null;
  canConnectLive: boolean;
  canPromote: boolean;
  canTakeRefinement: boolean;
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
  takeRefinement: [];
  discardRefinement: [];
  reconnect: [];
  restart: [];
  rewind: [];
}>();
</script>

<template>
  <footer class="flex flex-col gap-3 px-4 py-3 pb-4 border-t border-border-subtle bg-surface-highest/60">
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
        v-if="canTakeRefinement"
        :disabled="busyAction !== null"
        type="button"
        @click="$emit('takeRefinement')"
      >
        Take refined output
      </Button>
      <Button
        v-if="canTakeRefinement"
        variant="ghost"
        :disabled="busyAction !== null"
        type="button"
        @click="$emit('discardRefinement')"
      >
        Discard refinement
      </Button>
      <Button
        v-if="reconnectSessionId"
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
      class="flex flex-col gap-1 p-3 border border-border-subtle rounded-md bg-surface"
      role="status"
    >
      <span class="text-xs text-text-secondary">Forked session</span>
      <strong class="font-mono text-sm break-all">{{ forkedSessionId }}</strong>
    </div>
  </footer>
</template>
