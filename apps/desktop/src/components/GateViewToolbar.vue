<script setup lang="ts">
import { computed } from 'vue';
import { Button } from '@/components/ui/button';
import { getWorkspaceLabel } from '../lib/gate-display';

const selectFieldClass =
  'w-full rounded-md border border-white/10 bg-[var(--surface-container)] px-3 py-2 text-caption text-foreground outline-none transition focus:border-[var(--blue)]/40 focus:bg-[var(--surface-high)] focus:ring-2 focus:ring-[var(--blue)]/15';

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

const workspaceOptions = computed(() => props.gateRoots.map((root) => ({
  value: root,
  label: getWorkspaceLabel(root),
})));

const currentWorkspaceLabel = computed(() => (
  props.selectedGateRoot ? getWorkspaceLabel(props.selectedGateRoot) : 'No workspace selected'
));

function handleRootChange(event: Event) {
  emit('change-root', (event.target as HTMLSelectElement).value);
}
</script>

<template>
  <header class="gate-toolbar" role="toolbar" aria-label="Gate actions">
    <div class="gate-toolbar__copy">
      <div class="gate-toolbar__copy-block">
        <p class="gate-toolbar__title">Gates</p>
        <p class="gate-meta" :title="props.selectedGateRoot || undefined">
          {{ props.hasGateRoot ? `Workspace ${currentWorkspaceLabel}` : 'Register a workspace to inspect gate tasks.' }}
        </p>
      </div>
      <div v-if="props.hasGateRoot" class="gate-stats">
        <span class="gate-chip gate-chip--todo">{{ props.todoCount }} todo</span>
        <span class="gate-chip gate-chip--done">{{ props.doneCount }} done</span>
        <span class="gate-chip gate-chip--total">{{ props.totalCount }} total</span>
        <span v-if="props.shownCount !== props.totalCount" class="gate-chip gate-chip--filtered">{{ props.shownCount }} shown</span>
        <span v-if="props.lastRefreshLabel" class="gate-refreshed">{{ props.lastRefreshLabel }}</span>
      </div>
    </div>

    <div class="gate-toolbar__actions">
      <select :class="[selectFieldClass, 'gate-root-select']" :value="props.selectedGateRoot" :disabled="!props.isReady" @change="handleRootChange">
        <option value="">Select root…</option>
        <option v-for="root in workspaceOptions" :key="root.value" :value="root.value">{{ root.label }}</option>
      </select>
      <Button variant="ghost" @click="emit('browse')">Browse…</Button>
      <Button variant="ghost" :disabled="!props.hasGateRoot" @click="emit('remove')">Remove</Button>
      <Button variant="ghost" :disabled="!props.hasGateRoot || props.isLoading" @click="emit('refresh')">
        {{ props.isLoading ? 'Refreshing…' : 'Refresh' }}
      </Button>
    </div>
  </header>
</template>

<style scoped>
.gate-toolbar,
.gate-toolbar__actions,
.gate-stats,
.gate-toolbar__copy,
.gate-toolbar__copy-block {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.gate-toolbar {
  justify-content: space-between;
  gap: 1rem;
  padding: 0.9rem 1rem;
  background: var(--surface);
  border-radius: 0.25rem;
}

.gate-toolbar__copy,
.gate-toolbar__copy-block {
  flex-direction: column;
  align-items: flex-start;
}

.gate-toolbar__copy {
  gap: 0.75rem;
}

.gate-toolbar__title {
  font-family: var(--font-display);
  font-size: 1rem;
  font-weight: 500;
  color: var(--color-text-primary);
}

.gate-chip {
  padding: 0.28rem 0.65rem;
  font-family: var(--font-code);
  font-size: var(--text-tiny);
  color: var(--color-text-primary);
  border-radius: 4px;
  background: var(--surface-highest);
}

.gate-chip--todo {
  background: var(--blue);
}

.gate-chip--done {
  background: var(--sage);
}

.gate-chip--total {
  background: var(--outline-variant);
}

.gate-chip--filtered {
  background: var(--amber);
}

.gate-meta,
.gate-refreshed {
  color: var(--color-text-secondary);
}

.gate-refreshed {
  font-family: var(--font-code);
  font-size: var(--text-tiny);
}

.gate-root-select {
  min-width: 14rem;
  max-width: 18rem;
}

@media (max-width: 1100px) {
  .gate-toolbar {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
