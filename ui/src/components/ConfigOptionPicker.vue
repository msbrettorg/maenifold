<script setup lang="ts">
import { ref, computed } from 'vue';
import type { SessionConfigOption, ConfigSelectOption } from '../lib/types';
import { flattenSelectOptions } from '../lib/types';

const props = defineProps<{
  option: SessionConfigOption;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  change: [value: string | boolean];
}>();

const isOpen = ref(false);

// Category-based icon hint per spec
// (https://agentclientprotocol.com/protocol/session-config-options#option-categories)
const categoryIcon = computed((): string => {
  switch (props.option.category) {
    case 'mode': return '⚙️';
    case 'model': return '🤖';
    case 'thought_level': return '💭';
    default: return '🔧';
  }
});

const flatSelectOptions = computed((): ConfigSelectOption[] => {
  return props.option.type === 'select'
    ? flattenSelectOptions(props.option.options)
    : [];
});

const currentSelectName = computed((): string => {
  if (props.option.type !== 'select') return '';
  const current = flatSelectOptions.value.find(
    (o) => o.value === props.option.currentValue,
  );
  return current?.name ?? String(props.option.currentValue);
});

function toggleDropdown(): void {
  if (!props.disabled) {
    isOpen.value = !isOpen.value;
  }
}

function selectValue(value: string): void {
  if (props.option.type !== 'select') return;
  if (value !== props.option.currentValue) {
    emit('change', value);
  }
  isOpen.value = false;
}

function toggleBoolean(): void {
  if (props.disabled || props.option.type !== 'boolean') return;
  emit('change', !props.option.currentValue);
}

function handleClickOutside(event: MouseEvent): void {
  const target = event.target as HTMLElement;
  if (!target.closest(`.config-option-picker[data-config-id="${cssEscape(props.option.id)}"]`)) {
    isOpen.value = false;
  }
}

// Minimal CSS.escape shim — Tauri WebView2 provides CSS.escape but be defensive.
function cssEscape(value: string): string {
  if (typeof CSS !== 'undefined' && typeof CSS.escape === 'function') {
    return CSS.escape(value);
  }
  return value.replace(/[^a-zA-Z0-9_-]/g, '_');
}

if (typeof window !== 'undefined') {
  window.addEventListener('click', handleClickOutside);
}
</script>

<template>
  <div
    class="config-option-picker"
    :data-config-id="option.id"
    :class="{ disabled }"
    :title="option.description || option.name"
  >
    <!-- Boolean toggle: single button that flips on click -->
    <button
      v-if="option.type === 'boolean'"
      class="config-button boolean"
      :class="{ active: option.currentValue }"
      :disabled="disabled"
      @click.stop="toggleBoolean"
    >
      <span class="config-icon">{{ categoryIcon }}</span>
      <span class="config-name">{{ option.name }}</span>
      <span class="boolean-state">{{ option.currentValue ? 'ON' : 'OFF' }}</span>
    </button>

    <!-- Select dropdown: current value + chevron, dropdown lists options -->
    <template v-else>
      <button
        class="config-button select"
        :disabled="disabled"
        @click.stop="toggleDropdown"
      >
        <span class="config-icon">{{ categoryIcon }}</span>
        <span class="config-name">{{ currentSelectName }}</span>
        <span class="dropdown-arrow">{{ isOpen ? '▲' : '▼' }}</span>
      </button>

      <Transition name="dropdown">
        <div v-if="isOpen" class="dropdown-menu" @click.stop>
          <div class="dropdown-header">
            <span class="header-name">{{ option.name }}</span>
            <span v-if="option.description" class="header-description">
              {{ option.description }}
            </span>
          </div>
          <div
            v-for="opt in flatSelectOptions"
            :key="opt.value"
            :class="['dropdown-item', { selected: opt.value === option.currentValue }]"
            @click="selectValue(opt.value)"
          >
            <div class="item-content">
              <span class="item-name">{{ opt.name }}</span>
              <span v-if="opt.description" class="item-description">
                {{ opt.description }}
              </span>
            </div>
            <span v-if="opt.value === option.currentValue" class="check-mark">✓</span>
          </div>
        </div>
      </Transition>
    </template>
  </div>
</template>

<style scoped>
.config-option-picker {
  position: relative;
  display: inline-block;
}

.config-option-picker.disabled {
  opacity: 0.6;
  pointer-events: none;
}

.config-button {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.375rem 0.625rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-main);
  color: var(--text-primary);
  font-size: 0.8rem;
  cursor: pointer;
  transition: all 0.15s ease;
  max-width: 220px;
}

.config-button:hover:not(:disabled) {
  background: var(--bg-hover);
  border-color: var(--text-accent);
}

.config-button:disabled {
  cursor: not-allowed;
}

.config-button.boolean.active {
  background: var(--bg-user);
  border-color: var(--text-accent);
}

.config-icon {
  font-size: 0.9rem;
  flex-shrink: 0;
}

.config-name {
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.boolean-state {
  font-size: 0.7rem;
  font-weight: 600;
  margin-left: 0.25rem;
  padding: 0.0625rem 0.375rem;
  border-radius: 4px;
  background: var(--bg-hover);
  color: var(--text-muted);
}

.config-button.boolean.active .boolean-state {
  background: var(--text-accent);
  color: var(--bg-main);
}

.dropdown-arrow {
  font-size: 0.6rem;
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
  max-height: 360px;
  overflow-y: auto;
  background: var(--bg-main);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  z-index: 100;
}

@media (prefers-color-scheme: dark) {
  .dropdown-menu {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }
}

@media (max-width: 800px) {
  .dropdown-menu {
    position: fixed;
    top: calc(env(safe-area-inset-top, 0px) + 4rem);
    left: 0.5rem;
    right: 0.5rem;
    bottom: auto;
    min-width: 0;
    max-width: none;
    max-height: calc(100vh - 6rem);
  }
}

.dropdown-header {
  padding: 0.625rem 0.75rem;
  background: var(--bg-hover);
  border-bottom: 1px solid var(--border-color);
}

.header-name {
  display: block;
  font-weight: 600;
  color: var(--text-primary);
  font-size: 0.85rem;
}

.header-description {
  display: block;
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-top: 0.125rem;
  line-height: 1.3;
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
  background: var(--bg-hover);
}

.dropdown-item.selected {
  background: var(--bg-user);
}

.dropdown-item + .dropdown-item {
  border-top: 1px solid var(--border-color);
}

.item-content {
  flex: 1;
  min-width: 0;
}

.item-name {
  display: block;
  font-weight: 500;
  color: var(--text-primary);
  font-size: 0.85rem;
}

.item-description {
  display: block;
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-top: 0.125rem;
  line-height: 1.3;
}

.check-mark {
  color: var(--text-accent);
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
