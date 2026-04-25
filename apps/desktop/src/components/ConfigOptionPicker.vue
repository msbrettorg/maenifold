<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, useId } from 'vue';
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
const listboxRef = ref<HTMLElement | null>(null);
const optionRefs = new Map<string, HTMLElement>();
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

const activeValue = ref<string | null>(null);

const activeOptionId = computed(() =>
  activeValue.value ? optionElementId(activeValue.value) : undefined
);

const optionPickerLabel = computed(() =>
  props.option.description
    ? `${props.option.name}: ${props.option.description}`
    : `${props.option.name} options`
);

function optionElementId(value: string) {
  const index = flatOptions.value.findIndex((entry) => entry.value === value);
  return index >= 0 ? `${listboxId}-option-${index}` : undefined;
}

function setOptionRef(value: string, element: unknown) {
  if (element instanceof HTMLElement) {
    optionRefs.set(value, element);
  } else {
    optionRefs.delete(value);
  }
}

function setActiveValue(value: string | null) {
  activeValue.value = value;
  if (value) {
    nextTick(() => {
      optionRefs.get(value)?.scrollIntoView({ block: 'nearest' });
    });
  }
}

function getInitialActiveValue() {
  return currentOption.value?.value ?? flatOptions.value[0]?.value ?? null;
}

function openDropdown() {
  if (isOpen.value) return;
  optionRefs.clear();
  isOpen.value = true;

  nextTick(() => {
    setActiveValue(getInitialActiveValue());
    listboxRef.value?.focus();
  });
}

function closeDropdown() {
  isOpen.value = false;
}

function toggleDropdown() {
  if (isOpen.value) {
    closeDropdown();
  } else {
    openDropdown();
  }
}

function selectOption(value: string) {
  if (value !== props.option.currentValue) {
    emit('change', props.option.id, value);
  }
  closeDropdown();
}

function moveActiveValue(delta: number) {
  if (flatOptions.value.length === 0) return;
  const currentIndex = activeValue.value
    ? flatOptions.value.findIndex((entry) => entry.value === activeValue.value)
    : -1;
  const nextIndex = currentIndex < 0
    ? 0
    : Math.min(Math.max(currentIndex + delta, 0), flatOptions.value.length - 1);
  setActiveValue(flatOptions.value[nextIndex]?.value ?? null);
}

function handleListboxKeydown(event: KeyboardEvent) {
  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault();
      moveActiveValue(1);
      break;
    case 'ArrowUp':
      event.preventDefault();
      moveActiveValue(-1);
      break;
    case 'Home':
      event.preventDefault();
      setActiveValue(flatOptions.value[0]?.value ?? null);
      break;
    case 'End':
      event.preventDefault();
      setActiveValue(flatOptions.value.at(-1)?.value ?? null);
      break;
    case 'Enter':
    case ' ':
      if (activeValue.value) {
        event.preventDefault();
        selectOption(activeValue.value);
      }
      break;
    case 'Escape':
      event.preventDefault();
      closeDropdown();
      break;
  }
}

function handleClickOutside(event: MouseEvent) {
  const target = event.target as Node | null;
  if (target && pickerRef.value && !pickerRef.value.contains(target)) {
    closeDropdown();
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
    class="relative inline-block"
    :title="option.description || option.name"
  >
    <Button
      variant="outline"
      size="sm"
      class="flex items-center gap-1.5 px-2.5 py-1.5 max-w-[200px] border border-border-subtle rounded-lg bg-bg-app text-text-primary text-xs cursor-pointer transition-all duration-150 ease-in-out hover:bg-bg-surface hover:border-brand"
      :aria-controls="listboxId"
      :aria-expanded="isOpen"
      :aria-haspopup="'listbox'"
      :aria-label="optionPickerLabel"
      @click.stop="toggleDropdown"
    >
      <span class="text-body-sm shrink-0">
        {{ getOptionIcon(option.currentValue, option.category) }}
      </span>
      <span class="font-medium whitespace-nowrap overflow-hidden text-ellipsis">
        {{ currentOption?.name || option.currentValue || option.name }}
      </span>
      <span class="ml-1 shrink-0 text-xs text-text-secondary">{{ isOpen ? '▲' : '▼' }}</span>
    </Button>

    <Transition
      enter-active-class="transition-all duration-150 ease-out"
      enter-from-class="opacity-0 -translate-y-1"
      leave-active-class="transition-all duration-150 ease-out"
      leave-to-class="opacity-0 -translate-y-1"
    >
      <div
        v-if="isOpen"
        :id="listboxId"
        ref="listboxRef"
        class="absolute top-[calc(100%+4px)] right-0 z-[100] max-h-[min(320px,calc(100vh-6rem))] min-w-[280px] max-w-[360px] overflow-y-auto overscroll-contain rounded-lg border border-border-subtle bg-bg-app shadow-fluent-8 outline-none"
        role="listbox"
        tabindex="0"
        :aria-label="optionPickerLabel"
        :aria-activedescendant="activeOptionId"
        @click.stop
        @keydown="handleListboxKeydown"
      >
        <template
          v-for="(entry, entryIndex) in option.options"
          :key="isGroup(entry) ? entry.group : entry.value"
        >
          <div
            v-if="isGroup(entry)"
            class="[&:not(:first-child)]:border-t [&:not(:first-child)]:border-border-subtle"
            role="group"
            :aria-label="entry.name"
          >
            <div class="px-3 py-2 text-xs font-semibold tracking-[0.04em] text-text-secondary">
              {{ entry.name }}
            </div>
            <div
              v-for="(groupOption, groupIndex) in entry.options"
              :key="groupOption.value"
              :id="optionElementId(groupOption.value)"
              :ref="(element) => setOptionRef(groupOption.value, element)"
              :class="[
                'flex items-start gap-2 px-3 py-2.5 cursor-pointer transition-colors duration-100 ease-in-out hover:bg-bg-surface',
                groupIndex > 0 && 'border-t border-border-subtle',
                groupOption.value === option.currentValue && 'bg-brand/25',
                groupOption.value === activeValue && 'outline outline-1 outline-brand/60',
              ]"
              role="option"
              :aria-selected="groupOption.value === option.currentValue"
              @click="selectOption(groupOption.value)"
            >
              <span class="mt-0.5 shrink-0 text-body-sm">
                {{ getOptionIcon(groupOption.value, option.category) }}
              </span>
              <div class="flex-1 min-w-0">
                <span class="block text-xs font-medium text-text-primary">{{ groupOption.name }}</span>
                <span
                  v-if="groupOption.description"
                  class="mt-0.5 block text-xs leading-snug text-text-secondary"
                >
                  {{ groupOption.description }}
                </span>
              </div>
              <span
                v-if="groupOption.value === option.currentValue"
                class="ml-auto shrink-0 font-semibold text-brand"
              >✓</span>
            </div>
          </div>
          <div
            v-else
            :id="optionElementId(entry.value)"
            :ref="(element) => setOptionRef(entry.value, element)"
            :class="[
              'flex items-start gap-2 px-3 py-2.5 cursor-pointer transition-colors duration-100 ease-in-out hover:bg-bg-surface',
              entryIndex > 0 && 'border-t border-border-subtle',
              entry.value === option.currentValue && 'bg-brand/25',
              entry.value === activeValue && 'outline outline-1 outline-brand/60',
            ]"
            role="option"
            :aria-selected="entry.value === option.currentValue"
            @click="selectOption(entry.value)"
          >
            <span class="mt-0.5 shrink-0 text-body-sm">
              {{ getOptionIcon(entry.value, option.category) }}
            </span>
            <div class="flex-1 min-w-0">
              <span class="block text-xs font-medium text-text-primary">{{ entry.name }}</span>
              <span
                v-if="entry.description"
                class="mt-0.5 block text-xs leading-snug text-text-secondary"
              >
                {{ entry.description }}
              </span>
            </div>
            <span
              v-if="entry.value === option.currentValue"
              class="ml-auto shrink-0 font-semibold text-brand"
            >✓</span>
          </div>
        </template>
      </div>
    </Transition>
  </div>
</template>
