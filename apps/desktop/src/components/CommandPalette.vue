<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import type { AvailableCommand } from '../lib/types';

const props = defineProps<{
  commands: AvailableCommand[];
  filter: string;
  visible: boolean;
}>();

const emit = defineEmits<{
  select: [command: AvailableCommand];
  close: [];
}>();

const commandListId = 'command-palette-list';
const commandInputId = 'command-palette-input';
const selectedCommandName = ref('');

const filteredCommands = computed(() => {
  const filterText = props.filter.toLowerCase();
  if (!filterText) return props.commands;
  return props.commands.filter(cmd => 
    cmd.name.toLowerCase().startsWith(filterText) ||
    (cmd.description ?? '').toLowerCase().includes(filterText)
  );
});

const selectedCommand = computed(() =>
  filteredCommands.value.find((cmd) => cmd.name === selectedCommandName.value) ?? filteredCommands.value[0]
);

const commandFilterValue = computed(() => (props.filter ? `/${props.filter}` : '/'));

function commandOptionId(commandName: string): string {
  return `command-palette-option-${commandName.replace(/[^a-z0-9]+/gi, '-').toLowerCase()}`;
}

function formatCommandName(commandName: string): string {
  const normalized = commandName.replace(/[-_]+/g, ' ').trim().toLowerCase();
  return normalized ? normalized.charAt(0).toUpperCase() + normalized.slice(1) : commandName;
}

function syncSelection() {
  const firstCommand = filteredCommands.value[0];
  if (!firstCommand) {
    selectedCommandName.value = '';
    return;
  }

  if (!filteredCommands.value.some((cmd) => cmd.name === selectedCommandName.value)) {
    selectedCommandName.value = firstCommand.name;
  }
}

watch([() => props.filter, () => props.visible, filteredCommands], syncSelection, { immediate: true });

function moveSelection(offset: number) {
  if (filteredCommands.value.length === 0) return;

  const currentIndex = filteredCommands.value.findIndex((cmd) => cmd.name === selectedCommandName.value);
  const nextIndex = currentIndex === -1
    ? 0
    : (currentIndex + offset + filteredCommands.value.length) % filteredCommands.value.length;

  selectedCommandName.value = filteredCommands.value[nextIndex]?.name ?? '';
}

function handleKeyDown(event: KeyboardEvent) {
  if (!props.visible || filteredCommands.value.length === 0) return;

  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault();
      moveSelection(1);
      break;
    case 'ArrowUp':
      event.preventDefault();
      moveSelection(-1);
      break;
    case 'Enter':
      event.preventDefault();
      if (selectedCommand.value) {
        emit('select', selectedCommand.value);
      }
      break;
    case 'Escape':
      event.preventDefault();
      emit('close');
      break;
    case 'Tab':
      event.preventDefault();
      if (selectedCommand.value) {
        emit('select', selectedCommand.value);
      }
      break;
  }
}

defineExpose({ handleKeyDown });

function selectCommand(cmd: AvailableCommand) {
  selectedCommandName.value = cmd.name;
  emit('select', cmd);
}

function handleSelectionChange(value: unknown) {
  if (typeof value === 'string') {
    selectedCommandName.value = value;
  }
}
</script>

<template>
  <div v-if="visible" class="absolute bottom-full left-0 w-[400px] mb-1.5 bg-surface-container rounded-md shadow-fluent-16 max-h-80 overflow-visible z-[100] border border-border-subtle">
    <div class="p-0 rounded-md bg-transparent">
      <div class="flex items-center py-2.5 px-4 text-text-secondary">
        <Input
          :id="commandInputId"
          class="h-auto w-full rounded-none border-0 bg-transparent px-0 py-0 text-body-sm text-text-text-primary shadow-none ring-0 font-inherit focus:outline-none focus-visible:border-transparent focus-visible:ring-0"
          type="text"
          role="combobox"
          aria-label="Command palette"
          aria-autocomplete="list"
          :aria-controls="commandListId"
          :aria-activedescendant="selectedCommand ? commandOptionId(selectedCommand.name) : undefined"
          :aria-expanded="visible"
          :model-value="commandFilterValue"
          readonly
          tabindex="-1"
        />
      </div>

      <div :id="commandListId" class="max-h-none overflow-visible px-2 pb-2" role="listbox" :aria-labelledby="commandInputId">
        <ScrollArea class="max-h-80">
          <div v-if="filteredCommands.length === 0" class="p-4 text-center text-body-sm text-text-secondary">
            No matching commands.
          </div>
          <Button
            v-for="cmd in filteredCommands"
            :id="commandOptionId(cmd.name)"
            :key="cmd.name"
            variant="ghost"
            size="sm"
            role="option"
            :aria-selected="selectedCommandName === cmd.name"
            class="mt-1 flex h-auto w-full items-center justify-start gap-3 rounded-sm bg-transparent px-3 py-2 text-left transition-colors duration-150 ease-out hover:bg-surface-high"
            :class="{ 'bg-surface-high': selectedCommandName === cmd.name }"
            :title="cmd.description"
            @mouseenter="handleSelectionChange(cmd.name)"
            @click="selectCommand(cmd)"
          >
            <span class="min-w-[120px] shrink-0 overflow-hidden whitespace-nowrap text-ellipsis text-body-sm font-semibold text-text-text-primary">{{ formatCommandName(cmd.name) }}</span>
            <span class="flex-1 overflow-hidden whitespace-nowrap text-ellipsis text-body-sm text-text-secondary">{{ cmd.description }}</span>
            <span v-if="cmd.input?.hint" class="whitespace-nowrap font-mono text-caption text-text-secondary">{{ cmd.input.hint }}</span>
          </Button>
        </ScrollArea>
      </div>
    </div>
  </div>
</template>
