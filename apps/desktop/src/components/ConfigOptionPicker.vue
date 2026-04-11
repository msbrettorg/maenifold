<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
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
    <button
      class="config-option-button"
      @click.stop="toggleDropdown"
    >
      <span class="config-option-icon">
        {{ getOptionIcon(option.currentValue, option.category) }}
      </span>
      <span class="config-option-name">
        {{ currentOption?.name || option.currentValue || option.name }}
      </span>
      <span class="dropdown-arrow">{{ isOpen ? '▲' : '▼' }}</span>
    </button>

    <Transition name="dropdown">
      <div v-if="isOpen" class="dropdown-menu" @click.stop>
        <template
          v-for="entry in option.options"
          :key="isGroup(entry) ? entry.group : entry.value"
        >
          <div v-if="isGroup(entry)" class="dropdown-group">
            <div class="dropdown-group-header">{{ entry.name }}</div>
            <div
              v-for="groupOption in entry.options"
              :key="groupOption.value"
              :class="['dropdown-item', { selected: groupOption.value === option.currentValue }]"
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
  border: 1px solid var(--border-default);
  border-radius: 6px;
  background: var(--bg-main);
  color: var(--text-primary);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: all 0.15s ease;
  max-width: 200px;
}

.config-option-button:hover {
  background: var(--bg-surface);
  border-color: var(--border-focus);
}

.config-option-icon {
  font-size: var(--text-base);
  flex-shrink: 0;
}

.config-option-name {
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.dropdown-arrow {
  font-size: var(--text-xs);
  color: var(--text-muted);
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
  background: var(--bg-main);
  border: 1px solid var(--border-default);
  border-radius: 8px;
  box-shadow: 0 4px 12px color-mix(in srgb, var(--bg-main) 15%, transparent);
  z-index: 100;
}

@media (prefers-color-scheme: dark) {
  .dropdown-menu {
    box-shadow: 0 4px 12px color-mix(in srgb, var(--bg-main) 40%, transparent);
  }
}

.dropdown-group + .dropdown-group,
.dropdown-item + .dropdown-item,
.dropdown-group + .dropdown-item,
.dropdown-item + .dropdown-group {
  border-top: 1px solid var(--border-default);
}

.dropdown-group-header {
  padding: 0.5rem 0.75rem;
  font-size: var(--text-xs);
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
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
  background: var(--bg-surface);
}

.dropdown-item.selected {
  background: color-mix(in srgb, var(--border-focus) 24%, var(--bg-panel));
}

.item-icon {
  font-size: var(--text-base);
  flex-shrink: 0;
  margin-top: 0.125rem;
}

.item-content {
  flex: 1;
  min-width: 0;
}

.item-name {
  display: block;
  font-weight: 500;
  color: var(--text-primary);
  font-size: var(--text-sm);
}

.item-description {
  display: block;
  font-size: var(--text-xs);
  color: var(--text-muted);
  margin-top: 0.125rem;
  line-height: 1.3;
}

.check-mark {
  color: var(--border-focus);
  font-weight: bold;
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
