<script setup lang="ts">
import { computed, ref, useTemplateRef } from 'vue';
import { HelpCircle, MoreHorizontal } from 'lucide-vue-next';
import { SHORTCUTS } from '../composables/useGateKeyboard';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import type { GateTask } from '../lib/gate';
import { getWorkspaceLabel } from '../lib/gate-display';

const props = defineProps<{
  hasGateRoot: boolean;
  todoCount: number;
  doneCount: number;
  totalCount: number;
  shownCount: number;
  lastRefreshLabel: string;
  selectedGateRoot: string;
  selectedLocation: string;
  gateRoots: string[];
  isReady: boolean;
  isLoading: boolean;
  tasks: GateTask[];
  sprintOptions: number[];
  selectedSprints: Set<number>;
  searchText: string;
}>();

const emit = defineEmits<{
  browse: [];
  'change-root': [value: string];
  refresh: [];
  remove: [];
  'new-task': [];
  'update:selectedLocation': [value: string];
  'toggle-sprint': [sprint: number];
  'update:searchText': [value: string];
}>();

const helpOpen = ref(false);
const searchInputRef = useTemplateRef<InstanceType<typeof Input>>('searchInput');

defineExpose({
  searchInputEl: computed(() => (searchInputRef.value as any)?.$el ?? null),
  openHelp: () => { helpOpen.value = true; },
});

const currentRootBasename = computed(() =>
  props.selectedGateRoot ? getWorkspaceLabel(props.selectedGateRoot) : 'No workspace',
);

const otherRoots = computed(() =>
  props.gateRoots.filter((r) => r !== props.selectedGateRoot),
);

const stageCounts = computed(() => {
  const counts = { pending: 0, swe: 0, red: 0, blue: 0, done: 0 };
  for (const task of props.tasks) {
    const s = (task.state ?? '').toLowerCase();
    if (task.location === 'done' || s === 'done' || s === 'completed') {
      counts.done++;
    } else if (s === 'pending') {
      counts.pending++;
    } else if (s === 'swe' || s === 'execute' || s === 'remediate' || s === 'swe-confess') {
      counts.swe++;
    } else if (s === 'red-team' || s === 'review' || s === 'red-team-confess' || s === 'red-team-audit') {
      counts.red++;
    } else if (s === 'blue-team' || s === 'verify' || s === 'blue-team-confess') {
      counts.blue++;
    }
  }
  return counts;
});

function sprintActive(sprint: number) {
  return props.selectedSprints.has(sprint);
}
</script>

<template>
  <header
    class="flex items-center flex-wrap gap-2 px-4 py-2 bg-surface rounded"
    role="toolbar"
    aria-label="Gate actions"
  >
    <!-- Compact root chip + kebab -->
    <div class="flex items-center gap-1 shrink-0">
      <span
        class="px-2 py-[0.2rem] font-mono text-tiny text-text-primary bg-surface-high rounded-[4px] max-w-[10rem] truncate"
        :title="props.selectedGateRoot || undefined"
      >{{ currentRootBasename }}</span>
      <DropdownMenu>
        <DropdownMenuTrigger as-child>
          <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6"
            :disabled="!props.isReady"
            aria-label="Workspace options"
          >
            <MoreHorizontal class="h-3.5 w-3.5" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          <DropdownMenuSub v-if="otherRoots.length > 0">
            <DropdownMenuSubTrigger>Switch root</DropdownMenuSubTrigger>
            <DropdownMenuSubContent>
              <DropdownMenuItem
                v-for="root in otherRoots"
                :key="root"
                @click="emit('change-root', root)"
              >{{ getWorkspaceLabel(root) }}</DropdownMenuItem>
            </DropdownMenuSubContent>
          </DropdownMenuSub>
          <DropdownMenuItem v-else disabled>Switch root</DropdownMenuItem>
          <DropdownMenuItem @click="emit('browse')">Browse…</DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem :disabled="!props.hasGateRoot" @click="emit('remove')">Remove</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>

    <!-- Sprint chip strip (top 5 sprints, multi-select OR) -->
    <div v-if="props.hasGateRoot && props.sprintOptions.length > 0" class="flex items-center gap-1 flex-wrap">
      <Button
        v-for="sprint in props.sprintOptions"
        :key="sprint"
        type="button"
        variant="ghost"
        :class="[
          'px-[0.55rem] py-[0.2rem] h-auto font-mono text-tiny rounded-[4px] transition-[box-shadow,opacity] duration-150',
          sprintActive(sprint)
            ? 'bg-primary text-primary-foreground ring-2 ring-primary ring-offset-1 ring-offset-surface'
            : 'bg-surface-high text-text-secondary opacity-80 hover:opacity-100',
        ]"
        :aria-pressed="sprintActive(sprint)"
        @click="emit('toggle-sprint', sprint)"
      >S{{ sprint }}</Button>
    </div>

    <!-- Stage count badges (absolute totals, not sprint-filtered) -->
    <div v-if="props.hasGateRoot" class="flex items-center gap-1 font-mono text-tiny text-text-secondary">
      <span class="px-[0.5rem] py-[0.15rem] bg-surface-high text-text-secondary rounded-[4px]">Pending {{ stageCounts.pending }}</span>
      <span class="px-[0.5rem] py-[0.15rem] bg-surface-high text-brand rounded-[4px]">SWE {{ stageCounts.swe }}</span>
      <span class="px-[0.5rem] py-[0.15rem] bg-surface-high text-warning rounded-[4px]">Red {{ stageCounts.red }}</span>
      <span class="px-[0.5rem] py-[0.15rem] bg-surface-high text-info rounded-[4px]">Blue {{ stageCounts.blue }}</span>
      <span
        v-if="stageCounts.done > 0"
        class="px-[0.5rem] py-[0.15rem] bg-surface-high text-success rounded-[4px]"
      >Done {{ stageCounts.done }}</span>
    </div>

    <!-- Right: search + new task + ? -->
    <div class="flex items-center gap-2 ml-auto">
      <Input
        v-if="props.hasGateRoot"
        ref="searchInput"
        :model-value="props.searchText"
        type="search"
        placeholder="Search…"
        class="h-7 w-36 text-tiny"
        @update:model-value="(v) => emit('update:searchText', String(v))"
      />
      <Button
        variant="ghost"
        size="sm"
        :disabled="!props.hasGateRoot"
        data-testid="gate-new-task-trigger"
        @click="emit('new-task')"
      >+ New task</Button>
      <Button
        variant="ghost"
        size="icon"
        class="h-7 w-7"
        aria-label="Keyboard shortcuts"
        @click="helpOpen = true"
      >
        <HelpCircle class="h-4 w-4" />
      </Button>
    </div>

    <!-- Help sheet -->
    <Sheet v-model:open="helpOpen">
      <SheetContent side="right">
        <SheetHeader>
          <SheetTitle>Keyboard shortcuts</SheetTitle>
        </SheetHeader>
        <p class="text-text-secondary mt-4 mb-2 text-tiny">Gate tab keyboard shortcuts</p>
        <dl class="space-y-1">
          <div
            v-for="shortcut in SHORTCUTS"
            :key="shortcut.key"
            class="flex items-center justify-between gap-4"
          >
            <kbd class="font-mono text-tiny bg-surface-high px-1.5 py-0.5 rounded shrink-0">{{ shortcut.key }}</kbd>
            <span class="text-text-secondary text-tiny text-right">{{ shortcut.description }}</span>
          </div>
        </dl>
      </SheetContent>
    </Sheet>
  </header>
</template>
