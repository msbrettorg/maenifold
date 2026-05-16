<script setup lang="ts">
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { cwdHasReasonablePathPrefix } from '../lib/pipeline-types-node';
import type { PipelineNodeAgentConfig } from '../lib/pipeline-types-node';
import {
  AGENT_DEFAULT_OPTION,
  HEALTH_META,
  TOOL_PALETTE_CLASSES,
  healthBadgeToneClass,
  optionStateClass,
  syntheticChangeEvent,
  tagToneClass,
  type ToolPaletteOption,
  type ToolPaletteToolOptionProps,
} from '../lib/tool-palette-helpers';
import ToolAuthSetupDialog from './ToolAuthSetupDialog.vue';
import ToolPaletteToolList from './ToolPaletteToolList.vue';
import ToolPaletteAgentPanel from './ToolPaletteAgentPanel.vue';
import ToolPaletteSelectedTool from './ToolPaletteSelectedTool.vue';
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

const { selectField: selectFieldClass, panel: panelClass, tagBase: tagBaseClass, optionBase: optionBaseClass, flexRow: flexRowClass, splitRow: splitRowClass } = TOOL_PALETTE_CLASSES;

function updateCwd(value: string | number): void {
  const trimmed = String(value).trim();
  emit('update:cwd', trimmed.length > 0 ? trimmed : undefined);
}

function forwardToolName(value: string | number): void {
  emit('update:toolName', String(value));
}

function getCwdWarning(value: string): string | null {
  if (!value) return null;
  return cwdHasReasonablePathPrefix(value)
    ? null
    : 'Path should usually start with /, ~/, ./, ../, or a drive letter.';
}

function optionProps(tool: ToolPaletteOption, opts: { idSuffix?: string; isOption?: boolean; highlight?: boolean } = {}): ToolPaletteToolOptionProps {
  const idValue = opts.idSuffix ? `${optionId(tool.namespacedName)}${opts.idSuffix}` : optionId(tool.namespacedName);
  const isSelected = tool.kind === 'tool' && tool.toolName === props.toolName;
  const base: ToolPaletteToolOptionProps = {
    tool,
    optionId: idValue,
    optionClasses: [optionBaseClass, ...optionStateClass(tool.health, isSelected, tool.kind === 'server_state')],
    selected: isSelected,
    isPinned: tool.kind === 'tool' && nodePaletteStore.isToolPinned(tool.toolName!),
    isRecent: tool.kind === 'tool' && nodePaletteStore.recentTools.includes(tool.toolName!),
    healthIcon: HEALTH_META[tool.health].icon,
    healthLabel: HEALTH_META[tool.health].label,
    healthBadgeToneClass: healthBadgeToneClass(tool.health),
    healthTagToneClass: tagToneClass(tool.health),
    isOption: opts.isOption ?? false,
  };
  if (opts.highlight) {
    base.nameParts = highlightText(tool.displayName, searchQuery.value);
    base.descriptionParts = highlightText(tool.description || 'No description.', searchQuery.value);
  }
  return base;
}

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
  visibleTools,
} = useToolPaletteFields(props, emit);
</script>

<template>
  <div class="grid gap-3 w-full">
    <Label>
      <span>Agent Override</span>
      <Select :model-value="agentName || AGENT_DEFAULT_OPTION" @update:model-value="(val) => updateAgentName(syntheticChangeEvent(val))">
        <SelectTrigger :class="selectFieldClass" data-testid="tool-agent-select">
          <SelectValue placeholder="Use tool default" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem :value="AGENT_DEFAULT_OPTION">Use tool default</SelectItem>
          <SelectItem v-for="name in configStore.agentNames" :key="name" :value="name">
            {{ formatAgentOptionLabel(name) }}
          </SelectItem>
        </SelectContent>
      </Select>
    </Label>

    <ToolPaletteAgentPanel
      v-if="agentName"
      :agent-name="agentName"
      :agent-config="agentConfig"
      :probe="selectedAgentProbe()"
      :loading="isProbeLoading(agentName)"
      :ready="readyProbe()"
      :select-field-class="selectFieldClass"
      :panel-class="panelClass"
      :flex-row-class="flexRowClass"
      :get-agent-mode-value="getAgentModeValue"
      :get-agent-model-value="getAgentModelValue"
      :get-agent-thought-level-value="getAgentThoughtLevelValue"
      :get-editable-agent-config-options="getEditableAgentConfigOptions"
      :get-agent-config-option-value="getAgentConfigOptionValue"
      :is-session-config-select-group="isSessionConfigSelectGroup"
      :synthetic-change-event="syntheticChangeEvent"
      @update:mode="updateAgentMode"
      @update:model="updateAgentModel"
      @update:thought-level="updateAgentThoughtLevel"
      @update:config-option="updateAgentConfigOption"
    />

    <Label>
      <span>Working directory</span>
      <Input
        :model-value="cwd"
        type="text"
        placeholder="/absolute/path"
        @update:model-value="updateCwd"
      />
      <small v-if="getCwdWarning(cwd)" class="m-0 text-xs text-status-active">
        {{ getCwdWarning(cwd) }}
      </small>
    </Label>

    <div :class="splitRowClass">
      <div>
        <span>Tool Palette</span>
        <p class="m-0 text-text-secondary text-sm">Search across servers, or reuse your recent and pinned tools from Quick Add.</p>
      </div>
      <Button variant="ghost" :disabled="loading" @click="refreshCatalog">{{ loading ? 'Refreshing…' : 'Refresh' }}</Button>
    </div>
    <p v-if="error && !hasCatalog" class="m-0 text-status-abandoned text-sm">{{ error }}</p>
    <Input v-model="searchQuery" class="w-full" type="search" placeholder="Search tools by name or description…" aria-label="Search tools" />

    <div :class="splitRowClass">
      <strong>{{ isSearching ? `${searchResults.length} results` : 'Browse by intent' }}</strong>
      <Button v-if="searchQuery" variant="ghost" size="xs" @click="clearSearch">Clear search</Button>
    </div>

    <ToolPaletteToolList
      :has-catalog="hasCatalog"
      :is-searching="isSearching"
      :search-query="searchQuery"
      :show-categories="showCategories"
      :show-quick-add="showQuickAdd"
      :visible-tools="visibleTools"
      :quick-add-tools="quickAddTools"
      :categorized-tools="categorizedTools"
      :active-descendant-id="activeDescendantId"
      :split-row-class="splitRowClass"
      :tag-base-class="tagBaseClass"
      :option-props="optionProps"
      @select="handleToolSelect"
      @palette-keydown="handlePaletteKeydown"
    />

    <Label>
      <span>Tool Name</span>
      <Input :model-value="toolName" type="text" placeholder="server.ToolName" @update:model-value="forwardToolName" />
    </Label>

    <ToolPaletteSelectedTool
      v-if="toolName"
      :tool-name="toolName"
      :selected-discovery-tool="selectedDiscoveryTool"
      :selected-tool-description="selectedToolDescription"
      :selected-tool-health="selectedToolHealth"
      :selected-tool-auth-action="selectedToolAuthAction"
      :selected-tool-schema="selectedToolSchema"
      :tool-args="toolArgs"
      :required-arg-names="requiredArgNames"
      :can-submit="canSubmit"
      :panel-class="panelClass"
      :split-row-class="splitRowClass"
      :flex-row-class="flexRowClass"
      :tag-base-class="tagBaseClass"
      :health-meta="HEALTH_META"
      :tag-tone-class="tagToneClass"
      :pin-adapter="nodePaletteStore"
      @clear-selection="clearSelection"
      @submit="submitSelectedTool"
      @args-change="onToolArgsChange"
      @args-validation="onToolArgsValidation"
    />

    <ToolAuthSetupDialog
      v-if="authDialogAction && authDialogTool"
      :action="authDialogAction"
      :server="authDialogTool.server"
      :busy="authDialogBusy"
      :error="authDialogError"
      @close="closeAuthDialog"
      @confirm="runAuthSetup"
    />
  </div>
</template>
