<script setup lang="ts">
import ToolPaletteToolOption from './ToolPaletteToolOption.vue';
import type { ToolPaletteOption, ToolPaletteToolOptionProps } from '../lib/tool-palette-helpers';

interface CategoryEntry {
  category: { id: string; label: string; description: string };
  tools: ToolPaletteOption[];
}

defineProps<{
  hasCatalog: boolean;
  isSearching: boolean;
  searchQuery: string;
  showCategories: boolean;
  showQuickAdd: boolean;
  visibleTools: ToolPaletteOption[];
  quickAddTools: ToolPaletteOption[];
  categorizedTools: CategoryEntry[];
  activeDescendantId: string | undefined;
  splitRowClass: string;
  tagBaseClass: string;
  optionProps: (tool: ToolPaletteOption, opts?: { idSuffix?: string; isOption?: boolean; highlight?: boolean }) => ToolPaletteToolOptionProps;
}>();

const emit = defineEmits<{
  select: [tool: ToolPaletteOption];
  paletteKeydown: [event: KeyboardEvent];
}>();
</script>

<template>
  <section v-if="showQuickAdd" class="grid gap-3" aria-label="Quick Add">
    <div :class="splitRowClass">
      <div>
        <strong>Quick Add</strong>
        <p class="m-0 text-text-secondary text-sm">Reuse recent and pinned tools without leaving intent-based browse mode.</p>
      </div>
    </div>
    <div class="grid gap-1.5">
      <ToolPaletteToolOption
        v-for="tool in quickAddTools"
        :key="`quick-add-${tool.key}`"
        v-bind="optionProps(tool, { idSuffix: '-quick-add' })"
        @select="(t: ToolPaletteOption) => emit('select', t)"
      />
    </div>
  </section>

  <div
    class="grid gap-1.5 max-h-[min(18rem,38vh)] p-0.5 overflow-y-auto rounded-md outline-none"
    role="listbox"
    :aria-label="isSearching ? 'Tool search results' : 'Browse by intent tools'"
    :aria-activedescendant="activeDescendantId"
    tabindex="0"
    @keydown="(e: KeyboardEvent) => emit('paletteKeydown', e)"
  >
    <p v-if="!hasCatalog" class="m-0 text-text-secondary text-sm">Refresh to load tools from MCP servers.</p>
    <p v-else-if="visibleTools.length === 0 && isSearching" class="m-0 text-text-secondary text-sm">No tools match “{{ searchQuery }}”.</p>
    <p v-else-if="visibleTools.length === 0" class="m-0 text-text-secondary text-sm">No tools are available to browse yet. Refresh to load the tool catalog.</p>
    <template v-else-if="showCategories">
      <div
        v-for="(entry, categoryIndex) in categorizedTools"
        :key="entry.category.id"
        :class="['grid gap-2', categoryIndex > 0 ? 'pt-2 border-t border-border-subtle/65' : '']"
        role="group"
        :aria-label="entry.category.label"
      >
        <div :class="splitRowClass">
          <div>
            <strong>{{ entry.category.label }}</strong>
            <p class="m-0 text-text-secondary text-sm">{{ entry.category.description }}</p>
          </div>
          <span :class="tagBaseClass">{{ entry.tools.length }}</span>
        </div>
        <ToolPaletteToolOption
          v-for="tool in entry.tools"
          :key="tool.key"
          v-bind="optionProps(tool, { isOption: true })"
          @select="(t: ToolPaletteOption) => emit('select', t)"
        />
      </div>
    </template>
    <ToolPaletteToolOption
      v-else
      v-for="tool in visibleTools"
      :key="tool.key"
      v-bind="optionProps(tool, { isOption: true, highlight: true })"
      @select="(t: ToolPaletteOption) => emit('select', t)"
    />
  </div>
</template>
