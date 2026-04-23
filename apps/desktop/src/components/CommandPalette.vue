<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Command, CommandItem, CommandList } from '@/components/ui/command';
import { Input } from '@/components/ui/input';
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
    <Command
      :model-value="selectedCommandName"
      class="p-0 rounded-md bg-transparent"
      @update:model-value="handleSelectionChange"
    >
      <div class="flex items-center py-2.5 px-4 text-text-secondary">
        <Input
          class="w-full bg-transparent text-text-text-primary text-[length:var(--text-body-sm)] border-0 shadow-none ring-0 focus-visible:border-transparent focus-visible:ring-0 focus:outline-none h-auto px-0 py-0 rounded-none font-inherit"
          type="text"
          role="combobox"
          aria-label="Command palette"
          aria-autocomplete="list"
          :aria-controls="commandListId"
          :aria-expanded="visible"
          :model-value="commandFilterValue"
          readonly
          tabindex="-1"
        />
      </div>

      <CommandList :id="commandListId" class="max-h-80 overflow-y-auto px-2 pb-2">
        <div v-if="filteredCommands.length === 0" class="p-4 text-text-secondary text-[length:var(--text-body-sm)] text-center">
          No matching commands.
        </div>
        <CommandItem
          v-for="cmd in filteredCommands"
          :id="commandOptionId(cmd.name)"
          :key="cmd.name"
          :value="cmd.name"
          class="mt-1 rounded-sm bg-transparent transition-colors duration-150 ease-out data-[selected=true]:bg-surface-high hover:bg-surface-high"
          @mouseenter="selectedCommandName = cmd.name"
          @select="selectCommand(cmd)"
        >
          <div class="flex items-center w-full gap-3">
            <span class="text-text-text-primary text-[length:var(--text-body-sm)] min-w-[120px] font-[var(--weight-title)] shrink-0 whitespace-nowrap overflow-hidden text-ellipsis">{{ formatCommandName(cmd.name) }}</span>
            <span class="text-text-secondary text-[length:var(--text-body-sm)] flex-1 whitespace-nowrap overflow-hidden text-ellipsis">{{ cmd.description }}</span>
            <span v-if="cmd.input?.hint" class="text-text-secondary font-mono text-[length:var(--text-caption)] whitespace-nowrap">{{ cmd.input.hint }}</span>
          </div>
        </CommandItem>
      </CommandList>
    </Command>

    <div
      v-if="selectedCommand?.description"
      class="absolute left-[calc(100%+8px)] top-0 w-[220px] bg-surface-container text-text-text-primary rounded-md shadow-fluent-16 py-2 px-3 text-[length:var(--text-body-sm)] whitespace-normal z-[200] break-words border border-border-subtle"
    >
      {{ selectedCommand.description }}
    </div>
  </div>
</template>
