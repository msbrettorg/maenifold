<script setup lang="ts">
defineProps<{
  busyAction: string | null;
  canPromote: boolean;
  forkedSessionId: string | null;
  graftSubmitting: boolean;
  hasResults: boolean;
  promotionBusy: boolean;
  viewNodeStatus: string;
}>();

defineEmits<{
  abandon: [];
  fork: [];
  graft: [];
  promote: [];
  restart: [];
  rewind: [];
}>();
</script>

<template>
  <footer class="zoom-footer">
    <div class="zoom-actions">
      <button
        :class="['btn', 'btn-ghost', { 'btn--busy': busyAction === 'fork' }]"
        :disabled="busyAction !== null"
        type="button"
        @click="$emit('fork')"
      >
        {{ busyAction === 'fork' ? 'Forking...' : 'Fork from node' }}
      </button>
      <button
        v-if="['abandoned', 'stuck'].includes(viewNodeStatus)"
        :class="['btn', 'btn-primary', { 'btn--busy': busyAction === 'restart' }]"
        :disabled="busyAction !== null"
        type="button"
        @click="$emit('restart')"
      >
        {{ busyAction === 'restart' ? 'Restarting...' : 'Restart node' }}
      </button>
      <button
        v-if="['completed', 'abandoned', 'stuck'].includes(viewNodeStatus)"
        :class="['btn', 'btn-ghost', { 'btn--busy': busyAction === 'rewind' }]"
        :disabled="busyAction !== null"
        type="button"
        @click="$emit('rewind')"
      >
        {{ busyAction === 'rewind' ? 'Rewinding...' : 'Rewind node' }}
      </button>
      <button
        v-if="viewNodeStatus === 'active'"
        :class="['btn', 'btn-danger', { 'btn--busy': busyAction === 'abandon' }]"
        :disabled="busyAction !== null"
        type="button"
        @click="$emit('abandon')"
      >
        {{ busyAction === 'abandon' ? 'Abandoning...' : 'Abandon node' }}
      </button>
      <button
        v-if="hasResults"
        class="btn btn-ghost"
        :disabled="busyAction !== null || graftSubmitting"
        type="button"
        @click="$emit('graft')"
      >
        Graft from node
      </button>
      <button
        v-if="canPromote"
        :class="['btn', 'btn-primary', { 'btn--busy': promotionBusy }]"
        :disabled="promotionBusy || busyAction !== null"
        :title="promotionBusy ? 'Promotion in progress…' : 'Save this completed output to memory'"
        type="button"
        @click="$emit('promote')"
      >
        {{ promotionBusy ? 'Promoting…' : 'Promote to Memory' }}
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
  background: color-mix(in srgb, var(--accent, #353534) 55%, var(--card, #201f1f));
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
  border-radius: 8px;
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
</style>
