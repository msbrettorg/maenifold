<script setup lang="ts">
import { computed, ref } from 'vue';

interface ToolMenuItem {
  name: string;
  displayName: string;
  description?: string | null;
}

interface ToolMenuGroup {
  server: string;
  tools: ToolMenuItem[];
}

const props = defineProps<{
  x: number;
  y: number;
  mode: 'design' | 'server';
  target: 'canvas' | 'node';
  toolGroups: ToolMenuGroup[];
  canAskAgent: boolean;
  canThink: boolean;
  canRunWorkflow: boolean;
  pipelineStatus?: string | null;
  pipelineRunning?: boolean;
  /** Canvas-level: pipeline has completed/abandoned/stuck nodes. */
  hasRewindableNodes?: boolean;
  /** Canvas-level: pipeline has completed nodes with non-empty output. */
  hasCompletedNodesWithOutput?: boolean;
  /** Node-level: status of the right-clicked node (server mode). */
  nodeStatus?: string | null;
  /** Node-level: whether the right-clicked node has output (server mode). */
  nodeHasOutput?: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'ask-agent'): void;
  (e: 'think-problem'): void;
  (e: 'run-workflow'): void;
  (e: 'run-tool', toolName: string): void;
  (e: 'edit'): void;
  (e: 'duplicate'): void;
  (e: 'delete'): void;
  (e: 'disconnect-edges'): void;
  (e: 'restart'): void;
  (e: 'rewind'): void;
  (e: 'fork'): void;
  (e: 'abandon'): void;
  (e: 'run-pipeline'): void;
  (e: 'rewind-all'): void;
  (e: 'graft'): void;
  (e: 'promote-to-memory'): void;
}>();

const isNonTerminal = computed(() => {
  const s = props.pipelineStatus;
  return s === 'active' || s === 'paused' || s === 'stuck';
});

const hasServerCanvasActions = computed(() => {
  if (props.mode !== 'server' || props.target !== 'canvas') return false;
  return isNonTerminal.value || Boolean(props.hasRewindableNodes) || Boolean(props.hasCompletedNodesWithOutput);
});

// Node-level action visibility (server mode, target === 'node').
// When nodeStatus is unknown/null, fall back to showing all items.
const showRestart = computed(() => {
  const s = props.nodeStatus;
  if (!s) return true;
  return s === 'completed' || s === 'abandoned' || s === 'stuck';
});

const showRewind = computed(() => {
  const s = props.nodeStatus;
  if (!s) return true;
  return s === 'completed' || s === 'abandoned' || s === 'stuck';
});

const showFork = computed(() => {
  const s = props.nodeStatus;
  if (!s) return true;
  return s === 'completed' && Boolean(props.nodeHasOutput);
});

const showAbandon = computed(() => {
  const s = props.nodeStatus;
  if (!s) return true;
  return s === 'pending' || s === 'ready' || s === 'active' || s === 'retrying' || s === 'stuck';
});

const toolMenuOpen = ref(false);
const expandedServers = ref(new Set<string>());

const menuStyle = computed(() => ({
  left: `${props.x}px`,
  top: `${props.y}px`,
}));

function toggleServer(server: string): void {
  const next = new Set(expandedServers.value);
  if (next.has(server)) {
    next.delete(server);
  } else {
    next.add(server);
  }
  expandedServers.value = next;
}

function isServerExpanded(server: string): boolean {
  return expandedServers.value.has(server);
}

function handleAction(action: () => void): void {
  action();
  emit('close');
}

function handleTool(toolName: string): void {
  emit('run-tool', toolName);
  emit('close');
}
</script>

<template>
  <div
    v-if="mode === 'design' || target === 'node' || hasServerCanvasActions"
    class="composer-context-menu"
    :style="menuStyle"
    @contextmenu.prevent
  >
    <template v-if="mode === 'design' && target === 'canvas'">
      <button
        type="button"
        class="composer-context-menu__item"
        :disabled="!canAskAgent"
        @click="handleAction(() => emit('ask-agent'))"
      >
        Ask an agent…
      </button>
      <div
        class="composer-context-menu__submenu"
        @mouseenter="toolMenuOpen = true"
        @mouseleave="toolMenuOpen = false"
      >
        <button
          type="button"
          class="composer-context-menu__item composer-context-menu__item--submenu"
          :disabled="toolGroups.length === 0"
          @click.stop="toolMenuOpen = !toolMenuOpen"
        >
          <span>Run a tool…</span>
          <span aria-hidden="true">›</span>
        </button>
        <div v-if="toolMenuOpen && toolGroups.length > 0" class="composer-context-menu__submenu-panel">
          <section
            v-for="group in toolGroups"
            :key="group.server"
            class="composer-context-menu__tool-group"
          >
            <button
              type="button"
              class="composer-context-menu__server-toggle"
              :aria-expanded="isServerExpanded(group.server)"
              @click.stop="toggleServer(group.server)"
            >
              <span
                class="composer-context-menu__chevron"
                :class="{ 'composer-context-menu__chevron--open': isServerExpanded(group.server) }"
                aria-hidden="true"
              >▸</span>
              <span class="composer-context-menu__server-name">{{ group.server }}</span>
              <span class="composer-context-menu__tool-count">{{ group.tools.length }}</span>
            </button>
            <div v-if="isServerExpanded(group.server)" class="composer-context-menu__tool-list">
              <button
                v-for="tool in group.tools"
                :key="tool.name"
                type="button"
                class="composer-context-menu__tool-item"
                :title="tool.description ?? tool.name"
                @click="handleTool(tool.name)"
              >
                {{ tool.displayName }}
              </button>
            </div>
          </section>
        </div>
      </div>
      <button
        type="button"
        class="composer-context-menu__item"
        :disabled="!canThink"
        @click="handleAction(() => emit('think-problem'))"
      >
        Think through a problem…
      </button>
      <button
        type="button"
        class="composer-context-menu__item"
        :disabled="!canRunWorkflow"
        @click="handleAction(() => emit('run-workflow'))"
      >
        Run a workflow…
      </button>
    </template>

    <template v-else-if="mode === 'design'">
      <button type="button" class="composer-context-menu__item" @click="handleAction(() => emit('edit'))">
        Edit
      </button>
      <button type="button" class="composer-context-menu__item" @click="handleAction(() => emit('duplicate'))">
        Duplicate
      </button>
      <button type="button" class="composer-context-menu__item composer-context-menu__item--danger" @click="handleAction(() => emit('delete'))">
        Delete
      </button>
      <button type="button" class="composer-context-menu__item" @click="handleAction(() => emit('disconnect-edges'))">
        Disconnect all edges
      </button>
    </template>

    <template v-else-if="mode === 'server' && target === 'node'">
      <button v-if="showRestart" type="button" class="composer-context-menu__item" @click="handleAction(() => emit('restart'))">
        Restart
      </button>
      <button v-if="showRewind" type="button" class="composer-context-menu__item" @click="handleAction(() => emit('rewind'))">
        Rewind
      </button>
      <button v-if="showFork" type="button" class="composer-context-menu__item" @click="handleAction(() => emit('fork'))">
        Fork
      </button>
      <button v-if="showAbandon" type="button" class="composer-context-menu__item composer-context-menu__item--danger" @click="handleAction(() => emit('abandon'))">
        Abandon
      </button>
    </template>

    <template v-else-if="mode === 'server' && target === 'canvas'">
      <button
        v-if="pipelineStatus === 'active' && !pipelineRunning"
        type="button"
        class="composer-context-menu__item"
        @click="handleAction(() => emit('run-pipeline'))"
      >
        ▶ Run Pipeline
      </button>
      <button
        v-if="hasRewindableNodes"
        type="button"
        class="composer-context-menu__item"
        @click="handleAction(() => emit('rewind-all'))"
      >
        ↩ Rewind
      </button>
      <button
        v-if="isNonTerminal"
        type="button"
        class="composer-context-menu__item"
        @click="handleAction(() => emit('graft'))"
      >
        ⑂ Graft
      </button>
      <button
        v-if="hasCompletedNodesWithOutput"
        type="button"
        class="composer-context-menu__item"
        @click="handleAction(() => emit('promote-to-memory'))"
      >
        ✦ Promote to Memory
      </button>
    </template>
  </div>
</template>

<style scoped>
.composer-context-menu {
  position: fixed;
  z-index: 60;
  min-width: 220px;
  padding: 0.4rem;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 12px;
  background: #11161f;
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.38);
}

.composer-context-menu__item,
.composer-context-menu__tool-item {
  display: flex;
  width: 100%;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.65rem 0.8rem;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: inherit;
  text-align: left;
  font: inherit;
  cursor: pointer;
}

.composer-context-menu__item:hover:not(:disabled),
.composer-context-menu__tool-item:hover {
  background: rgba(255, 255, 255, 0.08);
}

.composer-context-menu__item:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.composer-context-menu__item--danger {
  color: #ff9b9b;
}

.composer-context-menu__submenu {
  position: relative;
}

.composer-context-menu__submenu-panel {
  position: absolute;
  top: 0;
  left: calc(100% + 0.4rem);
  min-width: 240px;
  max-height: min(420px, 70vh);
  overflow: auto;
  padding: 0.4rem;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 12px;
  background: #11161f;
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.38);
}

.composer-context-menu__tool-group + .composer-context-menu__tool-group {
  margin-top: 0.25rem;
  padding-top: 0.25rem;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.composer-context-menu__server-toggle {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.8rem;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: rgba(255, 255, 255, 0.82);
  text-align: left;
  font: inherit;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
}

.composer-context-menu__server-toggle:hover {
  background: rgba(255, 255, 255, 0.08);
}

.composer-context-menu__chevron {
  display: inline-block;
  font-size: 0.7rem;
  transition: transform 0.15s ease;
}

.composer-context-menu__chevron--open {
  transform: rotate(90deg);
}

.composer-context-menu__server-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.composer-context-menu__tool-count {
  flex-shrink: 0;
  padding: 0.1rem 0.45rem;
  border-radius: 9999px;
  background: rgba(255, 255, 255, 0.1);
  color: rgba(255, 255, 255, 0.56);
  font-size: 0.7rem;
  font-weight: 500;
  line-height: 1.3;
}

.composer-context-menu__tool-list {
  padding-left: 0.6rem;
}

.composer-context-menu__tool-group-title {
  padding: 0.2rem 0.8rem 0.35rem;
  color: rgba(255, 255, 255, 0.62);
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}
</style>
