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
  <p
    v-if="uiError"
    class="px-4 py-[0.9rem] border border-[var(--color-border-subtle)] rounded-[var(--radius-lg)] bg-[var(--color-bg-elevated)] text-[var(--color-status-abandoned)]"
    role="alert"
  >
    {{ uiError }}
  </p>
  <p
    v-else-if="!hasGateRoot"
    class="px-4 py-[0.9rem] border border-[var(--color-border-subtle)] rounded-[var(--radius-lg)] bg-[var(--color-bg-elevated)] text-[var(--color-text-secondary)]"
    role="status"
  >
    No Gate root selected.
  </p>
  <p
    v-else-if="isLoading && !snapshot"
    class="px-4 py-[0.9rem] border border-[var(--color-border-subtle)] rounded-[var(--radius-lg)] bg-[var(--color-bg-elevated)] text-[var(--color-text-secondary)]"
    role="status"
  >
    Loading gate snapshot…
  </p>
  <p
    v-else-if="snapshot && !snapshot.exists"
    class="px-4 py-[0.9rem] border border-[var(--color-border-subtle)] rounded-[var(--radius-lg)] bg-[var(--color-bg-elevated)] text-[var(--color-text-secondary)]"
    role="status"
  >
    No .gate/gate.db found in {{ selectedGateRoot }}.
  </p>
</template>
