<script setup lang="ts">
import type { GateSnapshot } from '../lib/gate';

defineProps<{
  uiError: string;
  hasGateRoot: boolean;
  isLoading: boolean;
  snapshot: GateSnapshot | null;
  selectedGateRoot: string;
}>();
</script>

<template>
  <p v-if="uiError" class="gate-state gate-state--error" role="alert">{{ uiError }}</p>
  <p v-else-if="!hasGateRoot" class="gate-state" role="status">No Gate root selected.</p>
  <p v-else-if="isLoading && !snapshot" class="gate-state" role="status">Loading gate snapshot…</p>
  <p v-else-if="snapshot && !snapshot.exists" class="gate-state" role="status">No .gate/gate.db found in {{ selectedGateRoot }}.</p>
</template>

<style scoped>
.gate-state {
  padding: 0.9rem 1rem;
  color: var(--muted-foreground);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--card);
}

.gate-state--error {
  color: var(--color-status-abandoned);
}
</style>
