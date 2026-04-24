<script setup lang="ts">
import { Button } from '@/components/ui/button';
import type { Component } from 'vue';
import type { ToolPaletteOption } from '../lib/tool-palette-helpers';

interface HighlightPart {
  text: string;
  match: boolean;
}

const props = defineProps<{
  tool: ToolPaletteOption;
  optionId: string;
  optionClasses: string[];
  selected: boolean;
  isPinned: boolean;
  isRecent: boolean;
  healthIcon: Component;
  healthLabel: string;
  healthBadgeToneClass: string;
  healthTagToneClass: string;
  isOption?: boolean;
  nameParts?: HighlightPart[];
  descriptionParts?: HighlightPart[];
}>();

defineEmits<{
  select: [tool: ToolPaletteOption];
}>();

const tagBaseClass =
  'inline-flex items-center px-1.5 py-0.5 border border-border-subtle rounded bg-text-primary/5 text-text-secondary text-xs';
const flexRowClass = 'flex flex-wrap gap-1.5 items-center';
</script>

<template>
  <Button
    :id="optionId"
    variant="ghost"
    :class="optionClasses"
    :role="isOption ? 'option' : undefined"
    :aria-selected="isOption ? selected : undefined"
    @click="$emit('select', props.tool)"
  >
    <div class="flex justify-between items-start gap-2">
      <div :class="flexRowClass">
        <span :class="['shrink-0 text-sm', healthBadgeToneClass]" :aria-label="healthLabel">
          <component :is="healthIcon" class="size-4" :stroke-width="1.5" />
        </span>
        <strong v-if="nameParts">
          <template v-for="(part, index) in nameParts" :key="`name-${index}`">
            <mark v-if="part.match" class="p-0 bg-status-active/25 text-inherit">{{ part.text }}</mark>
            <span v-else>{{ part.text }}</span>
          </template>
        </strong>
        <strong v-else>{{ tool.displayName }}</strong>
      </div>
      <span :class="[tagBaseClass, 'bg-text-primary/10']">{{ tool.server }}</span>
    </div>
    <span class="block text-text-secondary text-sm m-0 line-clamp-2">
      <template v-if="descriptionParts">
        <template v-for="(part, index) in descriptionParts" :key="`desc-${index}`">
          <mark v-if="part.match" class="p-0 bg-status-active/25 text-inherit">{{ part.text }}</mark>
          <span v-else>{{ part.text }}</span>
        </template>
      </template>
      <template v-else>{{ tool.description || 'No description.' }}</template>
    </span>
    <span v-if="tool.kind === 'server_state' && tool.error" class="text-status-abandoned text-sm m-0">{{ tool.error }}</span>
    <span :class="flexRowClass">
      <span :class="[tagBaseClass, healthTagToneClass]">{{ healthLabel }}</span>
      <span v-if="isPinned" :class="tagBaseClass">Pinned</span>
      <span v-if="isRecent" :class="tagBaseClass">Recent</span>
    </span>
  </Button>
</template>
