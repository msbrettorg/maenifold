<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, useId } from 'vue';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
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
        class="absolute top-[calc(100%+4px)] right-0 z-[100] min-w-[280px] max-w-[360px] rounded-lg border border-border-subtle bg-bg-app shadow-fluent-8"
        role="listbox"
        :aria-label="optionPickerLabel"
        @click.stop
      >
        <ScrollArea class="max-h-[320px]">
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
              :class="[
                'flex items-start gap-2 px-3 py-2.5 cursor-pointer transition-colors duration-100 ease-in-out hover:bg-bg-surface',
                groupIndex > 0 && 'border-t border-border-subtle',
                groupOption.value === option.currentValue && 'bg-brand/25',
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
            :class="[
              'flex items-start gap-2 px-3 py-2.5 cursor-pointer transition-colors duration-100 ease-in-out hover:bg-bg-surface',
              entryIndex > 0 && 'border-t border-border-subtle',
              entry.value === option.currentValue && 'bg-brand/25',
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
        </ScrollArea>
      </div>
    </Transition>
  </div>
</template>
