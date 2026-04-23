<script setup lang="ts">
import { computed } from 'vue';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { getWorkspaceLabel } from '../lib/gate-display';

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
</script>

<template>
  <header
    class="flex items-center flex-wrap gap-4 justify-between px-4 py-[0.9rem] bg-[var(--surface)] rounded max-[1100px]:flex-col max-[1100px]:items-stretch"
    role="toolbar"
    aria-label="Gate actions"
  >
    <div class="flex flex-col items-start flex-wrap gap-3">
      <div class="flex flex-col items-start flex-wrap gap-2">
        <p class="font-display text-base font-medium text-text-primary">Gates</p>
        <p class="text-text-secondary" :title="props.selectedGateRoot || undefined">
          {{ props.hasGateRoot ? `Workspace ${currentWorkspaceLabel}` : 'Register a workspace to inspect gate tasks.' }}
        </p>
      </div>
      <div v-if="props.hasGateRoot" class="flex items-center flex-wrap gap-2">
        <span class="px-[0.65rem] py-[0.28rem] font-mono text-[length:var(--text-tiny)] text-text-primary rounded-[4px] bg-[var(--color-brand)]">{{ props.todoCount }} todo</span>
        <span class="px-[0.65rem] py-[0.28rem] font-mono text-[length:var(--text-tiny)] text-text-primary rounded-[4px] bg-[var(--sage)]">{{ props.doneCount }} done</span>
        <span class="px-[0.65rem] py-[0.28rem] font-mono text-[length:var(--text-tiny)] text-text-primary rounded-[4px] bg-[var(--outline-variant)]">{{ props.totalCount }} total</span>
        <span
          v-if="props.shownCount !== props.totalCount"
          class="px-[0.65rem] py-[0.28rem] font-mono text-[length:var(--text-tiny)] text-text-primary rounded-[4px] bg-[var(--amber)]"
        >{{ props.shownCount }} shown</span>
        <span
          v-if="props.lastRefreshLabel"
          class="text-text-secondary font-mono text-[length:var(--text-tiny)]"
        >{{ props.lastRefreshLabel }}</span>
      </div>
    </div>

    <div class="flex items-center flex-wrap gap-2">
      <Select :model-value="props.selectedGateRoot || ''" :disabled="!props.isReady" @update:model-value="(v) => emit('change-root', String(v))">
        <SelectTrigger class="min-w-[14rem] max-w-[18rem]">
          <SelectValue placeholder="Select root…" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem v-for="root in workspaceOptions" :key="root.value" :value="root.value">{{ root.label }}</SelectItem>
        </SelectContent>
      </Select>
      <Button variant="ghost" @click="emit('browse')">Browse…</Button>
      <Button variant="ghost" :disabled="!props.hasGateRoot" @click="emit('remove')">Remove</Button>
      <Button variant="ghost" :disabled="!props.hasGateRoot || props.isLoading" @click="emit('refresh')">
        {{ props.isLoading ? 'Refreshing…' : 'Refresh' }}
      </Button>
    </div>
  </header>
</template>
