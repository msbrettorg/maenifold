<script setup lang="ts">
import type { PropType } from 'vue';

interface ToolPaletteItem {
  server: string;
  status: 'ok' | 'error';
  error?: string | null;
  namespacedName: string;
  displayName: string;
  description?: string | null;
}

defineProps({
  toolName: { type: String, required: true },
  loading: { type: Boolean, required: true },
  error: { type: String as PropType<string | null>, default: null },
  hasCatalog: { type: Boolean, required: true },
  searchQuery: { type: String, required: true },
  isSearching: { type: Boolean, required: true },
  searchResults: { type: Array as PropType<ToolPaletteItem[]>, required: true },
  visibleTools: { type: Array as PropType<ToolPaletteItem[]>, required: true },
  activeDescendantId: { type: String, default: undefined },
  selectedToolName: { type: String as PropType<string | null>, default: null },
  recentTools: { type: Array as PropType<string[]>, required: true },
  isToolPinned: { type: Function as PropType<(name: string) => boolean>, required: true },
  optionId: { type: Function as PropType<(name: string) => string>, required: true },
  highlightText: { type: Function as PropType<(text: string, query: string) => Array<{ text: string; match: boolean }>>, required: true },
  refreshCatalog: { type: Function as PropType<() => void | Promise<void>>, required: true },
  clearSearch: { type: Function as PropType<() => void>, required: true },
  handleSearchKeydown: { type: Function as PropType<(event: KeyboardEvent) => void>, required: true },
  handlePaletteKeydown: { type: Function as PropType<(event: KeyboardEvent) => void>, required: true },
  selectTool: { type: Function as PropType<(name: string) => void>, required: true },
  updateToolName: { type: Function as PropType<(event: Event) => void>, required: true },
});

const emit = defineEmits<{ 'update:searchQuery': [value: string] }>();
</script>

<template>
  <div class="tool-palette__header">
    <div>
      <span>Tool Palette</span>
      <p>Search across servers, or reuse your recent and pinned tools from Quick Add.</p>
    </div>
    <button type="button" class="btn btn-ghost" :disabled="loading" @click="refreshCatalog">
      {{ loading ? 'Refreshing…' : 'Refresh' }}
    </button>
  </div>

  <p v-if="error && !hasCatalog" class="tool-palette__status tool-palette__status--error">{{ error }}</p>

  <input
    class="field-input tool-palette__search"
    :value="searchQuery"
    type="search"
    placeholder="Search tools by name or description…"
    aria-label="Search tools"
    @input="emit('update:searchQuery', ($event.target as HTMLInputElement).value)"
    @keydown="handleSearchKeydown"
  />

  <div class="tool-palette__status-row">
    <strong>{{ isSearching ? `${searchResults.length} results` : 'Quick Add' }}</strong>
    <button v-if="searchQuery" type="button" class="btn btn-ghost btn-xs" @click="clearSearch">Clear search</button>
  </div>

  <div class="tool-palette__listbox" role="listbox" :aria-label="isSearching ? 'Tool search results' : 'Quick Add tools'" :aria-activedescendant="activeDescendantId" tabindex="0" @keydown="handlePaletteKeydown">
    <p v-if="!hasCatalog" class="tool-palette__status">Refresh to load tools from MCP servers.</p>
    <p v-else-if="visibleTools.length === 0 && isSearching" class="tool-palette__status">No tools match “{{ searchQuery }}”.</p>
    <p v-else-if="visibleTools.length === 0" class="tool-palette__status">Quick Add is empty. Start typing to search across all connected servers.</p>
    <button
      v-for="tool in visibleTools"
      :id="optionId(tool.namespacedName)"
      :key="tool.namespacedName"
      type="button"
      class="tool-palette__option"
      :class="{ 'is-selected': tool.namespacedName === selectedToolName }"
      role="option"
      :aria-selected="tool.namespacedName === selectedToolName"
      @click="selectTool(tool.namespacedName)"
    >
      <div class="tool-palette__option-head">
        <strong>
          <template v-for="(part, index) in highlightText(tool.displayName, searchQuery)" :key="`${tool.namespacedName}-name-${index}`">
            <mark v-if="part.match">{{ part.text }}</mark>
            <span v-else>{{ part.text }}</span>
          </template>
        </strong>
        <span class="tool-palette__server">{{ tool.server }}</span>
      </div>
      <span class="tool-palette__option-desc">
        <template v-for="(part, index) in highlightText(tool.description || 'No description.', searchQuery)" :key="`${tool.namespacedName}-desc-${index}`">
          <mark v-if="part.match">{{ part.text }}</mark>
          <span v-else>{{ part.text }}</span>
        </template>
      </span>
      <span class="tool-palette__option-tags">
        <span v-if="isToolPinned(tool.namespacedName)" class="tool-palette__tag">Pinned</span>
        <span v-if="recentTools.includes(tool.namespacedName)" class="tool-palette__tag">Recent</span>
        <span v-if="tool.status === 'error'" class="tool-palette__tag tool-palette__tag--warning">Stale catalog</span>
      </span>
    </button>
  </div>

  <label>
    <span>Tool Name</span>
    <input class="field-input" :value="toolName" type="text" placeholder="server.ToolName" @input="updateToolName" />
  </label>
</template>
