<script setup lang="ts">
const props = defineProps<{
  hasGateRoot: boolean;
  todoCount: number;
  doneCount: number;
  totalCount: number;
  shownCount: number;
  lastRefreshLabel: string;
  selectedGateRoot: string;
  gateRoots: string[];
  isReady: boolean;
  isLoading: boolean;
}>();

const emit = defineEmits<{
  browse: [];
  'change-root': [value: string];
  refresh: [];
  remove: [];
}>();

function handleRootChange(event: Event) {
  emit('change-root', (event.target as HTMLSelectElement).value);
}
</script>

<template>
  <header class="gate-toolbar" role="toolbar" aria-label="Gate actions">
    <div v-if="props.hasGateRoot" class="gate-stats">
      <span class="gate-chip gate-chip--todo">{{ props.todoCount }} todo</span>
      <span class="gate-chip gate-chip--done">{{ props.doneCount }} done</span>
      <span class="gate-chip">{{ props.totalCount }} total</span>
      <span v-if="props.shownCount !== props.totalCount" class="gate-chip gate-chip--filtered">{{ props.shownCount }} shown</span>
      <span v-if="props.lastRefreshLabel" class="gate-refreshed">{{ props.lastRefreshLabel }}</span>
    </div>
    <p v-else class="gate-meta">Register a root to inspect .gate/gate.db.</p>

    <div class="gate-toolbar__actions">
      <select class="field-select gate-root-select" :value="props.selectedGateRoot" :disabled="!props.isReady" @change="handleRootChange">
        <option value="">Select root…</option>
        <option v-for="root in props.gateRoots" :key="root" :value="root">{{ root }}</option>
      </select>
      <button type="button" class="btn btn-ghost" @click="emit('browse')">Browse…</button>
      <button type="button" class="btn btn-ghost" :disabled="!props.hasGateRoot" @click="emit('remove')">Remove</button>
      <button type="button" class="btn btn-ghost" :disabled="!props.hasGateRoot || props.isLoading" @click="emit('refresh')">
        {{ props.isLoading ? 'Refreshing…' : 'Refresh' }}
      </button>
    </div>
  </header>
</template>

<style scoped>
.gate-toolbar,
.gate-toolbar__actions,
.gate-stats {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.gate-toolbar {
  justify-content: space-between;
  gap: 1rem;
}

.gate-chip {
  padding: 0.25rem 0.6rem;
  font-family: var(--font-code);
  font-size: var(--text-tiny);
  color: var(--muted-foreground);
  border-radius: var(--radius-md);
  background: var(--muted);
}

.gate-chip--todo {
  color: var(--color-status-active);
  background: color-mix(in srgb, var(--color-status-active) 12%, var(--muted));
}

.gate-chip--done {
  color: var(--color-status-completed);
  background: color-mix(in srgb, var(--color-status-completed) 12%, var(--muted));
}

.gate-chip--filtered {
  color: var(--ring);
  background: color-mix(in srgb, var(--ring) 10%, var(--muted));
}

.gate-refreshed,
.gate-meta {
  color: var(--muted-foreground);
}

.gate-refreshed {
  font-size: var(--text-tiny);
}

.gate-root-select {
  min-width: 14rem;
  max-width: 24rem;
}

@media (max-width: 1100px) {
  .gate-toolbar {
    flex-direction: column;
  }
}
</style>
