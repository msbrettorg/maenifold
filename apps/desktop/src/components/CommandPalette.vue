<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Command, CommandItem, CommandList } from '@/components/ui/command';
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
  <div v-if="visible" class="command-palette">
    <Command
      :model-value="selectedCommandName"
      class="command-root"
      @update:model-value="handleSelectionChange"
    >
      <div class="command-input-row">
        <input
          class="command-input"
          type="text"
          role="combobox"
          aria-label="Command palette"
          aria-autocomplete="list"
          :aria-controls="commandListId"
          :aria-expanded="visible"
          :value="commandFilterValue"
          readonly
          tabindex="-1"
        >
      </div>

      <CommandList :id="commandListId" class="command-list">
        <div v-if="filteredCommands.length === 0" class="command-empty">
          No matching commands.
        </div>
        <CommandItem
          v-for="cmd in filteredCommands"
          :id="commandOptionId(cmd.name)"
          :key="cmd.name"
          :value="cmd.name"
          class="command-item"
          @mouseenter="selectedCommandName = cmd.name"
          @select="selectCommand(cmd)"
        >
          <div class="command-row">
            <span class="command-name">{{ formatCommandName(cmd.name) }}</span>
            <span class="command-description">{{ cmd.description }}</span>
            <span v-if="cmd.input?.hint" class="command-hint">{{ cmd.input.hint }}</span>
          </div>
        </CommandItem>
      </CommandList>
    </Command>

    <div v-if="selectedCommand?.description" class="command-tooltip">
      {{ selectedCommand.description }}
    </div>
  </div>
</template>

<style scoped>
.command-palette {
  position: absolute;
  bottom: 100%;
  left: 0;
  width: 400px;
  margin-bottom: 6px;
  background: var(--surface-container);
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: var(--radius);
  box-shadow: 0 0 32px rgba(173, 198, 255, 0.08);
  max-height: 320px;
  overflow: visible;
  z-index: 100;
}

.command-root {
  padding: 0;
  border-radius: var(--radius);
  background: transparent;
}

.command-input-row {
  display: flex;
  align-items: center;
  padding: 10px 16px;
  color: var(--text-dim);
}

.command-input {
  width: 100%;
  border: none;
  background: transparent;
  color: var(--color-text-primary);
  font: inherit;
  font-size: var(--text-body-sm);
}

.command-input:focus {
  outline: none;
}

.command-list {
  max-height: 320px;
  overflow-y: auto;
  padding: 0 0.5rem 0.5rem;
}

.command-item {
  margin-top: 0.25rem;
  border-radius: 4px;
  background: transparent;
  transition: background-color 150ms ease;
}

.command-item[data-selected='true'],
.command-item:hover {
  background: var(--surface-high);
}

.command-empty {
  padding: 1rem;
  color: var(--text-dim);
  font-size: var(--text-body-sm);
  text-align: center;
}

.command-tooltip {
  position: absolute;
  left: calc(100% + 8px);
  top: 0;
  width: 220px;
  background: var(--surface-container);
  color: var(--color-text-primary);
  border: 1px solid rgba(66, 71, 83, 0.15);
  border-radius: var(--radius);
  box-shadow: 0 0 32px rgba(173, 198, 255, 0.08);
  padding: 8px 12px;
  font-size: var(--text-body-sm);
  white-space: normal;
  z-index: 200;
  word-break: break-word;
}

.command-row {
  display: flex;
  align-items: center;
  width: 100%;
  gap: 0.75rem;
}

.command-name {
  color: var(--color-text-primary);
  font-size: var(--text-body-sm);
  min-width: 120px;
  font-weight: var(--weight-title);
  flex-shrink: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  text-transform: none;
}

.command-description {
  color: var(--text-secondary);
  font-size: var(--text-body-sm);
  flex: 1 1 auto;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.command-hint {
  color: var(--text-dim);
  font-family: var(--font-code);
  font-size: var(--text-caption);
  white-space: nowrap;
}
</style>
