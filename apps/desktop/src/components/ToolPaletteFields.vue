<script setup lang="ts">
import { cwdHasReasonablePathPrefix } from '../lib/pipeline-types';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types';
import type { ServerHealthStatus } from '../lib/types';
import AgentProbePreview from './AgentProbePreview.vue';
import SchemaForm from './SchemaForm.vue';
import ToolAuthSetupDialog from './ToolAuthSetupDialog.vue';
import { useToolPaletteFields } from '../composables/useToolPaletteFields';

const props = withDefaults(defineProps<{
  active?: boolean;
  toolName: string;
  agentName?: string;
  agentConfig?: PipelineNodeAgentConfig;
  cwd?: string;
  toolArgs: Record<string, unknown>;
  canSubmit?: boolean;
}>(), {
  active: false,
  agentName: '',
  agentConfig: () => ({}),
  cwd: '',
  canSubmit: false,
});

const emit = defineEmits<{
  'update:toolName': [value: string];
  'update:agentName': [value: string];
  'update:agentConfig': [value: PipelineNodeAgentConfig];
  'update:cwd': [value: string | undefined];
  'update:toolArgs': [value: Record<string, unknown>];
  validation: [state: { valid: boolean }];
  submitRequest: [];
}>();

function updateCwd(event: Event): void {
  const trimmed = (event.target as HTMLInputElement).value.trim();
  emit('update:cwd', trimmed.length > 0 ? trimmed : undefined);
}

function getCwdWarning(value: string): string | null {
  if (!value) {
    return null;
  }

  return cwdHasReasonablePathPrefix(value)
    ? null
    : 'Path should usually start with /, ~/, ./, ../, or a drive letter.';
}

const HEALTH_META: Record<ServerHealthStatus, { icon: string; label: string }> = {
  ready: { icon: '✓', label: 'Ready' },
  auth_required: { icon: '🔑', label: 'Setup Required' },
  stale: { icon: '⚠️', label: 'Stale' },
  unavailable: { icon: '✗', label: 'Unavailable' },
};

const {
  activeDescendantId,
  authDialogAction,
  authDialogBusy,
  authDialogError,
  authDialogTool,
  categorizedTools,
  clearSearch,
  clearSelection,
  closeAuthDialog,
  configStore,
  error,
  formatAgentOptionLabel,
  getAgentConfigOptionValue,
  getAgentModeValue,
  getAgentModelValue,
  getAgentThoughtLevelValue,
  getEditableAgentConfigOptions,
  handlePaletteKeydown,
  handleToolSelect,
  hasCatalog,
  highlightText,
  isProbeLoading,
  isSearching,
  isSessionConfigSelectGroup,
  loading,
  nodePaletteStore,
  onToolArgsChange,
  onToolArgsValidation,
  optionId,
  quickAddTools,
  readyProbe,
  refreshCatalog,
  requiredArgNames,
  runAuthSetup,
  searchQuery,
  searchResults,
  selectedAgentProbe,
  selectedDiscoveryTool,
  selectedToolAuthAction,
  selectedToolDescription,
  selectedToolHealth,
  selectedToolSchema,
  showCategories,
  showQuickAdd,
  submitSelectedTool,
  updateAgentConfigOption,
  updateAgentMode,
  updateAgentModel,
  updateAgentName,
  updateAgentThoughtLevel,
  updateToolName,
  visibleTools,
} = useToolPaletteFields(props, emit);
</script>

<template>
  <div class="tool-palette field-wide">
    <label>
      <span>Agent Override</span>
      <select class="field-input" data-testid="tool-agent-select" :value="agentName" @change="updateAgentName">
        <option value="">Use tool default</option>
        <option v-for="name in configStore.agentNames" :key="name" :value="name">
          {{ formatAgentOptionLabel(name) }}
        </option>
      </select>
    </label>

    <div v-if="agentName" class="tool-agent-panel">
      <AgentProbePreview compact :agent-name="agentName" :probe="selectedAgentProbe()" :loading="isProbeLoading(agentName)" />
      <template v-if="readyProbe()">
        <div v-if="readyProbe()?.modeOptions.length" class="agent-mode-badges">
          <button v-for="mode in readyProbe()?.modeOptions" :key="mode.id" type="button" class="mode-badge" :class="{ selected: getAgentModeValue(agentConfig, readyProbe()) === mode.id }" :data-testid="`tool-mode-badge-${mode.id}`" @click="updateAgentMode(mode.id)">{{ mode.name }}</button>
        </div>
        <div class="tool-agent-grid">
          <label v-if="readyProbe()?.modelOptions.length"><span>Model</span><select class="field-input" data-testid="tool-model-select" :value="getAgentModelValue(agentConfig, readyProbe())" @change="updateAgentModel"><option v-for="model in readyProbe()?.modelOptions" :key="model.modelId" :value="model.modelId">{{ model.name }}</option></select></label>
          <label v-if="readyProbe()?.thoughtLevelConfigOption"><span>{{ readyProbe()?.thoughtLevelConfigOption?.name }}</span><select class="field-input" data-testid="tool-thought-level-select" :value="getAgentThoughtLevelValue(agentConfig, readyProbe())" @change="updateAgentThoughtLevel"><template v-for="entry in readyProbe()?.thoughtLevelConfigOption?.options ?? []" :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"><optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name"><option v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">{{ groupOption.name }}</option></optgroup><option v-else :value="entry.value">{{ entry.name }}</option></template></select></label>
          <label v-for="option in getEditableAgentConfigOptions(readyProbe())" :key="option.id"><span>{{ option.name }}</span><select class="field-input" :data-testid="`tool-config-${option.id}`" :value="getAgentConfigOptionValue(agentConfig, option)" @change="updateAgentConfigOption(option, $event)"><template v-for="entry in option.options" :key="isSessionConfigSelectGroup(entry) ? entry.group : entry.value"><optgroup v-if="isSessionConfigSelectGroup(entry)" :label="entry.name"><option v-for="groupOption in entry.options" :key="groupOption.value" :value="groupOption.value">{{ groupOption.name }}</option></optgroup><option v-else :value="entry.value">{{ entry.name }}</option></template></select></label>
        </div>
      </template>
    </div>

    <label>
      <span>Working directory</span>
      <input class="field-input" :value="cwd" type="text" placeholder="/absolute/path" @input="updateCwd" />
      <small v-if="getCwdWarning(cwd)" class="tool-palette__hint tool-palette__hint--warning">
        {{ getCwdWarning(cwd) }}
      </small>
    </label>

    <div class="tool-palette__header"><div><span>Tool Palette</span><p>Search across servers, or reuse your recent and pinned tools from Quick Add.</p></div><button type="button" class="btn btn-ghost" :disabled="loading" @click="refreshCatalog">{{ loading ? 'Refreshing…' : 'Refresh' }}</button></div>
    <p v-if="error && !hasCatalog" class="tool-palette__status tool-palette__status--error">{{ error }}</p>
    <input v-model="searchQuery" class="field-input tool-palette__search" type="search" placeholder="Search tools by name or description…" aria-label="Search tools" />

    <div class="tool-palette__status-row"><strong>{{ isSearching ? `${searchResults.length} results` : 'Browse by intent' }}</strong><button v-if="searchQuery" type="button" class="btn btn-ghost btn-xs" @click="clearSearch">Clear search</button></div>

    <section v-if="showQuickAdd" class="tool-palette__quick-add" aria-label="Quick Add">
      <div class="tool-palette__quick-add-header"><div><strong>Quick Add</strong><p>Reuse recent and pinned tools without leaving intent-based browse mode.</p></div></div>
      <div class="tool-palette__quick-add-list">
        <button v-for="tool in quickAddTools" :id="`${optionId(tool.namespacedName)}-quick-add`" :key="`quick-add-${tool.key}`" type="button" class="tool-palette__option" :class="[`tool-palette__option--${tool.health}`, { 'is-selected': tool.kind === 'tool' && tool.toolName === toolName, 'tool-palette__option--server-state': tool.kind === 'server_state' }]" @click="handleToolSelect(tool)">
          <div class="tool-palette__option-head"><div class="tool-palette__option-title"><span class="tool-health-badge" :class="`health-${tool.health}`" :aria-label="HEALTH_META[tool.health].label">{{ HEALTH_META[tool.health].icon }}</span><strong>{{ tool.displayName }}</strong></div><span class="tool-palette__server">{{ tool.server }}</span></div>
          <span class="tool-palette__option-desc">{{ tool.description || 'No description.' }}</span>
          <span v-if="tool.kind === 'server_state' && tool.error" class="tool-palette__option-error">{{ tool.error }}</span>
          <span class="tool-palette__option-tags"><span class="tool-palette__tag" :class="`tool-palette__tag--${tool.health}`">{{ HEALTH_META[tool.health].label }}</span><span v-if="tool.kind === 'tool' && nodePaletteStore.isToolPinned(tool.toolName!)" class="tool-palette__tag">Pinned</span><span v-if="tool.kind === 'tool' && nodePaletteStore.recentTools.includes(tool.toolName!)" class="tool-palette__tag">Recent</span></span>
        </button>
      </div>
    </section>

    <div class="tool-palette__listbox" role="listbox" :aria-label="isSearching ? 'Tool search results' : 'Browse by intent tools'" :aria-activedescendant="activeDescendantId" tabindex="0" @keydown="handlePaletteKeydown">
      <p v-if="!hasCatalog" class="tool-palette__status">Refresh to load tools from MCP servers.</p>
      <p v-else-if="visibleTools.length === 0 && isSearching" class="tool-palette__status">No tools match “{{ searchQuery }}”.</p>
      <p v-else-if="visibleTools.length === 0" class="tool-palette__status">No tools are available to browse yet. Refresh to load the tool catalog.</p>
      <template v-else-if="showCategories">
        <div v-for="entry in categorizedTools" :key="entry.category.id" class="tool-palette__category" role="group" :aria-label="entry.category.label">
          <div class="tool-palette__category-header"><div><strong>{{ entry.category.label }}</strong><p>{{ entry.category.description }}</p></div><span class="tool-palette__tag">{{ entry.tools.length }}</span></div>
          <button v-for="tool in entry.tools" :id="optionId(tool.namespacedName)" :key="tool.key" type="button" class="tool-palette__option" :class="[`tool-palette__option--${tool.health}`, { 'is-selected': tool.kind === 'tool' && tool.toolName === toolName, 'tool-palette__option--server-state': tool.kind === 'server_state' }]" role="option" :aria-selected="tool.kind === 'tool' && tool.toolName === toolName" @click="handleToolSelect(tool)">
            <div class="tool-palette__option-head"><div class="tool-palette__option-title"><span class="tool-health-badge" :class="`health-${tool.health}`" :aria-label="HEALTH_META[tool.health].label">{{ HEALTH_META[tool.health].icon }}</span><strong>{{ tool.displayName }}</strong></div><span class="tool-palette__server">{{ tool.server }}</span></div>
            <span class="tool-palette__option-desc">{{ tool.description || 'No description.' }}</span>
            <span v-if="tool.kind === 'server_state' && tool.error" class="tool-palette__option-error">{{ tool.error }}</span>
            <span class="tool-palette__option-tags"><span class="tool-palette__tag" :class="`tool-palette__tag--${tool.health}`">{{ HEALTH_META[tool.health].label }}</span><span v-if="tool.kind === 'tool' && nodePaletteStore.isToolPinned(tool.toolName!)" class="tool-palette__tag">Pinned</span><span v-if="tool.kind === 'tool' && nodePaletteStore.recentTools.includes(tool.toolName!)" class="tool-palette__tag">Recent</span></span>
          </button>
        </div>
      </template>
      <button v-else v-for="tool in visibleTools" :id="optionId(tool.namespacedName)" :key="tool.key" type="button" class="tool-palette__option" :class="[`tool-palette__option--${tool.health}`, { 'is-selected': tool.kind === 'tool' && tool.toolName === toolName, 'tool-palette__option--server-state': tool.kind === 'server_state' }]" role="option" :aria-selected="tool.kind === 'tool' && tool.toolName === toolName" @click="handleToolSelect(tool)">
        <div class="tool-palette__option-head"><div class="tool-palette__option-title"><span class="tool-health-badge" :class="`health-${tool.health}`" :aria-label="HEALTH_META[tool.health].label">{{ HEALTH_META[tool.health].icon }}</span><strong><template v-for="(part, index) in highlightText(tool.displayName, searchQuery)" :key="`${tool.namespacedName}-name-${index}`"><mark v-if="part.match">{{ part.text }}</mark><span v-else>{{ part.text }}</span></template></strong></div><span class="tool-palette__server">{{ tool.server }}</span></div>
        <span class="tool-palette__option-desc"><template v-for="(part, index) in highlightText(tool.description || 'No description.', searchQuery)" :key="`${tool.namespacedName}-desc-${index}`"><mark v-if="part.match">{{ part.text }}</mark><span v-else>{{ part.text }}</span></template></span>
        <span v-if="tool.kind === 'server_state' && tool.error" class="tool-palette__option-error">{{ tool.error }}</span>
        <span class="tool-palette__option-tags"><span class="tool-palette__tag" :class="`tool-palette__tag--${tool.health}`">{{ HEALTH_META[tool.health].label }}</span><span v-if="tool.kind === 'tool' && nodePaletteStore.isToolPinned(tool.toolName!)" class="tool-palette__tag">Pinned</span><span v-if="tool.kind === 'tool' && nodePaletteStore.recentTools.includes(tool.toolName!)" class="tool-palette__tag">Recent</span></span>
      </button>
    </div>

    <label><span>Tool Name</span><input class="field-input" :value="toolName" type="text" placeholder="server.ToolName" @input="updateToolName" /></label>

    <section v-if="toolName" class="tool-palette__detail">
      <div class="tool-palette__detail-head"><div><strong>{{ selectedDiscoveryTool?.displayName ?? toolName.split('.').pop() }}</strong><p>{{ selectedToolDescription || 'No description.' }}</p></div><div class="tool-palette__detail-actions"><span v-if="selectedDiscoveryTool?.server" class="tool-palette__server">{{ selectedDiscoveryTool.server }}</span><span v-if="selectedToolHealth" class="tool-palette__tag" :class="`tool-palette__tag--${selectedToolHealth}`">{{ HEALTH_META[selectedToolHealth].label }}</span><button type="button" class="btn btn-ghost btn-xs" @click="nodePaletteStore.togglePinnedTool(toolName)">{{ nodePaletteStore.isToolPinned(toolName) ? 'Unpin' : 'Pin' }}</button><button type="button" class="btn btn-ghost btn-xs" @click="clearSelection">Clear</button></div></div>
      <p v-if="selectedDiscoveryTool?.error" class="tool-palette__status" :class="{ 'tool-palette__status--error': selectedToolHealth === 'unavailable' }">{{ selectedDiscoveryTool.error }}</p>
      <div v-if="selectedToolHealth === 'auth_required' && selectedToolAuthAction" class="tool-palette__setup-callout"><p>Complete auth setup before adding this tool.</p><button type="button" class="btn btn-primary" @click="submitSelectedTool">{{ selectedToolAuthAction.label }}</button></div>
      <template v-else>
        <div class="tool-palette__required"><span>Required args</span><div class="tool-palette__required-tags"><span v-for="requiredName in requiredArgNames" :key="requiredName" class="tool-palette__required-tag">{{ requiredName }}</span><span v-if="requiredArgNames.length === 0" class="tool-palette__required-empty">No required args</span></div></div>
        <SchemaForm :schema="selectedToolSchema" :data="toolArgs" @change="onToolArgsChange" @validation="onToolArgsValidation" />
        <div v-if="canSubmit" class="tool-palette__detail-footer"><button type="button" class="btn btn-primary" @click="submitSelectedTool">Add Tool</button></div>
      </template>
    </section>

    <ToolAuthSetupDialog v-if="authDialogAction && authDialogTool" :action="authDialogAction" :server="authDialogTool.server" :busy="authDialogBusy" :error="authDialogError" @close="closeAuthDialog" @confirm="runAuthSetup" />
  </div>
</template>

<style scoped>
.tool-palette,.tool-agent-panel,.tool-agent-grid,.tool-palette__detail,.tool-palette__setup-callout,.tool-palette__quick-add,.tool-palette__quick-add-list { display: grid; gap: 0.75rem; }
.tool-palette__hint { margin: 0; font-size: var(--text-xs); }
.tool-palette__hint--warning { color: var(--color-status-active); }
.tool-agent-panel,.tool-palette__detail,.tool-palette__setup-callout { padding: 0.8rem; border: 1px solid var(--border-default); border-radius: 0.85rem; background: color-mix(in srgb, var(--bg-panel) 72%, transparent); }
.tool-agent-grid { grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr)); }
.agent-mode-badges,.tool-palette__option-tags,.tool-palette__required-tags,.tool-palette__detail-actions,.tool-palette__option-title { display: flex; flex-wrap: wrap; gap: 0.35rem; align-items: center; }
.tool-palette__category { display: grid; gap: 0.45rem; }
.tool-palette__category + .tool-palette__category { padding-top: 0.55rem; border-top: 1px solid color-mix(in srgb, var(--border-default) 65%, transparent); }
.tool-palette__category-header,.tool-palette__quick-add-header { display: flex; justify-content: space-between; align-items: flex-start; gap: 0.75rem; }
.tool-palette__category-header p,.tool-palette__quick-add-header p { color: var(--text-secondary); font-size: var(--text-sm); margin: 0; }
.mode-badge,.tool-palette__tag,.tool-palette__required-tag,.tool-palette__server { display: inline-flex; align-items: center; padding: 0.15rem 0.45rem; border: 1px solid var(--border-default); border-radius: 999px; font-size: var(--text-xs); }
.mode-badge { background: var(--bg-main); color: var(--text-secondary); cursor: pointer; }
.mode-badge.selected,.tool-palette__option.is-selected { border-color: var(--border-focus); box-shadow: 0 0 0 1px var(--border-focus); }
.tool-palette__header,.tool-palette__status-row,.tool-palette__detail-head,.tool-palette__detail-footer { display: flex; justify-content: space-between; align-items: flex-start; gap: 0.75rem; }
.tool-palette__header p,.tool-palette__detail-head p,.tool-palette__status,.tool-palette__option-desc,.tool-palette__option-error,.tool-palette__required-empty,.tool-palette__setup-callout p { color: var(--text-secondary); font-size: var(--text-sm); margin: 0; }
.tool-palette__status--error,.tool-palette__tag--unavailable { color: var(--color-status-abandoned); }
.tool-palette__listbox { display: grid; gap: 0.45rem; max-height: min(18rem, 38vh); padding: 0.1rem; overflow-y: auto; border-radius: 0.8rem; outline: none; }
.tool-palette__option { display: grid; gap: 0.35rem; padding: 0.7rem 0.8rem; border: 1px solid var(--border-default); border-radius: 0.75rem; background: var(--bg-main); color: var(--text-primary); text-align: left; cursor: pointer; }
.tool-palette__option:hover { background: color-mix(in srgb, var(--text-primary) 4%, var(--bg-main)); }
.tool-palette__option:focus-visible,.tool-palette__listbox:focus-visible { outline: 2px solid var(--border-focus); outline-offset: 2px; }
.tool-palette__option--stale,.tool-palette__option--unavailable { opacity: 0.58; }
.tool-palette__option--server-state { border-style: dashed; }
.tool-palette__option-head { display: flex; justify-content: space-between; align-items: flex-start; gap: 0.5rem; }
.tool-palette__option-desc { display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
.tool-palette__option-error { color: var(--color-status-abandoned); }
.tool-palette__server { background: color-mix(in srgb, var(--text-primary) 8%, transparent); color: var(--text-secondary); }
.tool-palette__tag { background: color-mix(in srgb, var(--text-primary) 6%, transparent); color: var(--text-secondary); }
.tool-palette__tag--auth_required { color: var(--color-status-active); border-color: color-mix(in srgb, var(--color-status-active) 35%, var(--border-default)); }
.tool-palette__required { display: grid; gap: 0.45rem; }
.tool-palette__required-tag { background: color-mix(in srgb, var(--bg-main) 84%, transparent); }
.tool-health-badge { font-size: var(--text-sm); flex-shrink: 0; }
.health-ready { color: var(--text-primary); }
.health-auth_required { color: var(--color-status-active); }
.health-stale { opacity: 0.5; }
.health-unavailable { color: var(--color-status-abandoned); opacity: 0.5; }
mark { padding: 0; background: color-mix(in srgb, var(--color-status-active) 24%, transparent); color: inherit; }
</style>
