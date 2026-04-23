<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, useId } from 'vue';
import { Button } from '@/components/ui/button';
import type {
  SessionConfigOption,
  SessionConfigSelectGroup,
  SessionConfigSelectOption,
} from '../lib/types';
import { getOptionIcon } from './configOptionIcons';

const props = defineProps<{
  option: SessionConfigOption;
}>();

const emit = defineEmits<{
  change: [configId: string, value: string];
}>();

const isOpen = ref(false);
const pickerRef = ref<HTMLElement | null>(null);
const listboxId = useId();

function isGroup(
  option: SessionConfigSelectOption | SessionConfigSelectGroup
): option is SessionConfigSelectGroup {
  return 'group' in option;
}

const flatOptions = computed(() =>
  props.option.options.flatMap((entry) =>
    isGroup(entry) ? entry.options : [entry]
  )
);

const currentOption = computed(() =>
  flatOptions.value.find((entry) => entry.value === props.option.currentValue)
);

const optionPickerLabel = computed(() =>
  props.option.description
    ? `${props.option.name}: ${props.option.description}`
    : `${props.option.name} options`
);

function toggleDropdown() {
  isOpen.value = !isOpen.value;
}

function selectOption(value: string) {
  if (value !== props.option.currentValue) {
    emit('change', props.option.id, value);
  }
  isOpen.value = false;
}

function handleClickOutside(event: MouseEvent) {
  const target = event.target as Node | null;
  if (target && pickerRef.value && !pickerRef.value.contains(target)) {
    isOpen.value = false;
  }
}

onMounted(() => {
  window.addEventListener('click', handleClickOutside);
});

onUnmounted(() => {
  window.removeEventListener('click', handleClickOutside);
});
</script>

<template>
  <div
    ref="pickerRef"
    class="config-option-picker"
    :title="option.description || option.name"
  >
    <Button
      variant="outline"
      size="sm"
      class="config-option-button"
      :aria-controls="listboxId"
      :aria-expanded="isOpen"
      :aria-haspopup="'listbox'"
      :aria-label="optionPickerLabel"
      @click.stop="toggleDropdown"
    >
      <span class="config-option-icon">
        {{ getOptionIcon(option.currentValue, option.category) }}
      </span>
      <span class="config-option-name">
        {{ currentOption?.name || option.currentValue || option.name }}
      </span>
      <span class="dropdown-arrow">{{ isOpen ? '▲' : '▼' }}</span>
    </Button>

    <Transition name="dropdown">
      <div
        v-if="isOpen"
        :id="listboxId"
        class="dropdown-menu"
        role="listbox"
        :aria-label="optionPickerLabel"
        @click.stop
      >
        <template
          v-for="entry in option.options"
          :key="isGroup(entry) ? entry.group : entry.value"
        >
          <div v-if="isGroup(entry)" class="dropdown-group" role="group" :aria-label="entry.name">
            <div class="dropdown-group-header">{{ entry.name }}</div>
            <div
              v-for="groupOption in entry.options"
              :key="groupOption.value"
              :class="['dropdown-item', { selected: groupOption.value === option.currentValue }]"
              role="option"
              :aria-selected="groupOption.value === option.currentValue"
              @click="selectOption(groupOption.value)"
            >
              <span class="item-icon">
                {{ getOptionIcon(groupOption.value, option.category) }}
              </span>
              <div class="item-content">
                <span class="item-name">{{ groupOption.name }}</span>
                <span v-if="groupOption.description" class="item-description">
                  {{ groupOption.description }}
                </span>
              </div>
              <span v-if="groupOption.value === option.currentValue" class="check-mark">✓</span>
            </div>
          </div>
          <div
            v-else
            :class="['dropdown-item', { selected: entry.value === option.currentValue }]"
            role="option"
            :aria-selected="entry.value === option.currentValue"
            @click="selectOption(entry.value)"
          >
            <span class="item-icon">
              {{ getOptionIcon(entry.value, option.category) }}
            </span>
            <div class="item-content">
              <span class="item-name">{{ entry.name }}</span>
              <span v-if="entry.description" class="item-description">
                {{ entry.description }}
              </span>
            </div>
            <span v-if="entry.value === option.currentValue" class="check-mark">✓</span>
          </div>
        </template>
      </div>
    </Transition>
  </div>
</template>

<!-- audit-ok: max-style-rules -->
<style scoped>
.config-option-picker {
  position: relative;
  display: inline-block;
}

.config-option-button {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.375rem 0.625rem;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  background: var(--color-bg-app);
  color: var(--color-text-primary);
  font-size: var(--text-caption);
  cursor: pointer;
  transition: all 0.15s ease;
  max-width: 200px;
}

.config-option-button:hover {
  background: var(--color-bg-surface);
  border-color: var(--color-brand);
}

.config-option-icon {
  font-size: var(--text-body-sm);
  flex-shrink: 0;
}

.config-option-name {
  font-weight: var(--weight-display);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.dropdown-arrow {
  font-size: var(--text-tiny);
  color: var(--color-text-secondary);
  margin-left: 0.25rem;
  flex-shrink: 0;
}

.dropdown-menu {
  position: absolute;
  top: calc(100% + 4px);
  right: 0;
  min-width: 280px;
  max-width: 360px;
  max-height: 320px;
  overflow-y: auto;
  background: var(--color-bg-app);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius);
  box-shadow: var(--shadow-8);
  z-index: 100;
}

@media (prefers-color-scheme: dark) {
  .dropdown-menu {
    box-shadow: var(--shadow-8);
  }
}

.dropdown-group + .dropdown-group,
.dropdown-item + .dropdown-item,
.dropdown-group + .dropdown-item,
.dropdown-item + .dropdown-group {
  border-top: 1px solid var(--color-border-subtle);
}

.dropdown-group-header {
  padding: 0.5rem 0.75rem;
  font-size: var(--text-tiny);
  font-weight: var(--weight-title);
  color: var(--color-text-secondary);
  letter-spacing: 0.04em;
}

.dropdown-item {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 0.625rem 0.75rem;
  cursor: pointer;
  transition: background 0.1s ease;
}

.dropdown-item:hover {
  background: var(--color-bg-surface);
}

.dropdown-item.selected {
  background: color-mix(in srgb, var(--color-brand) 24%, var(--color-bg-elevated));
}

.item-icon {
  font-size: var(--text-body-sm);
  flex-shrink: 0;
  margin-top: 0.125rem;
}

.item-content {
  flex: 1;
  min-width: 0;
}

.item-name {
  display: block;
  font-weight: var(--weight-display);
  color: var(--color-text-primary);
  font-size: var(--text-caption);
}

.item-description {
  display: block;
  font-size: var(--text-tiny);
  color: var(--color-text-secondary);
  margin-top: 0.125rem;
  line-height: 1.3;
}

.check-mark {
  color: var(--color-brand);
  font-weight: var(--weight-title);
  margin-left: auto;
  flex-shrink: 0;
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
