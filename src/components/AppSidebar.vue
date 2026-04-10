<script setup lang="ts">
import AgentSelector from './AgentSelector.vue';
import SessionList from './SessionList.vue';
import StartupProgress from './StartupProgress.vue';
import { useResizeHandle } from '../composables/useResizeHandle';
import { getSidebarSizingTokens } from '../lib/sidebarSizing';
import type { SavedSession } from '../lib/types';

const visible = defineModel<boolean>('visible', { required: true });
const selectedAgent = defineModel<string>('selectedAgent', { required: true });
const showTrafficMonitor = defineModel<boolean>('showTrafficMonitor', { required: true });
const showStartupDetails = defineModel<boolean>('showStartupDetails', { required: true });

defineProps<{
  selectedCwd: string;
  hasAgents: boolean;
  isConnecting: boolean;
  startupPhase: string;
  startupLogs: string[];
  startupElapsed: number;
}>();

const emit = defineEmits<{
  'open-settings': [];
  'agent-select': [agentName: string];
  'select-folder': [];
  'new-session': [];
  'resume-session': [session: SavedSession];
  'delete-session': [sessionId: string];
  'cancel-connection': [];
}>();

const sidebarSizing = getSidebarSizingTokens();
const resize = useResizeHandle({
  initial: sidebarSizing.width,
  min: sidebarSizing.minWidth,
  max: sidebarSizing.maxWidth,
});
</script>

<template>
  <!-- Sidebar panel -->
  <aside v-if="visible" class="sidebar" :style="{ width: resize.width.value + 'px', minWidth: resize.width.value + 'px' }">
    <div class="sidebar-header">
      <h1>maenifold</h1>
      <div class="header-actions">
        <button
          class="settings-btn"
          :class="{ active: showTrafficMonitor }"
          @click="showTrafficMonitor = !showTrafficMonitor"
          title="ACP Traffic Monitor"
        >📡</button>
        <button class="settings-btn" @click="emit('open-settings')" title="Settings">⚙</button>
        <button class="toggle-btn" @click="visible = false">◀</button>
      </div>
    </div>

    <div class="sidebar-content">
      <!-- Agent Selection -->
      <div class="section">
        <AgentSelector
          v-model:selected="selectedAgent"
          @select="(name: string) => emit('agent-select', name)"
        />

        <!-- Working Directory Picker -->
        <div class="cwd-picker">
          <label>Working Directory:</label>
          <div class="cwd-row">
            <span class="cwd-path" :title="selectedCwd || 'Current directory'">
              {{ selectedCwd ? selectedCwd.split(/[\\/]/).pop() : '.' }}
            </span>
            <button
              class="cwd-btn"
              @click="emit('select-folder')"
              title="Select folder"
            >
              📁
            </button>
          </div>
        </div>

        <button
          v-if="hasAgents"
          class="new-session-btn"
          :disabled="!selectedAgent || isConnecting"
          @click="emit('new-session')"
        >
          New Session
        </button>

        <!-- Startup Progress -->
        <StartupProgress
          v-if="isConnecting"
          :agent-name="selectedAgent"
          :phase="startupPhase"
          :logs="startupLogs"
          :elapsed-seconds="startupElapsed"
          :show-details="showStartupDetails"
          @cancel="emit('cancel-connection')"
          @toggle-details="showStartupDetails = !showStartupDetails"
        />
      </div>

      <!-- Session List -->
      <div class="section">
        <SessionList
          @resume="(session: SavedSession) => emit('resume-session', session)"
          @delete="(id: string) => emit('delete-session', id)"
        />
      </div>

    </div>
  </aside>

  <!-- Sidebar resize handle -->
  <div
    v-if="visible"
    class="app-resize-handle"
    @mousedown="resize.startResize($event, 'right')"
  />

  <!-- Collapsed sidebar toggle -->
  <button
    v-if="!visible"
    class="sidebar-toggle-collapsed"
    @click="visible = true"
  >
    ▶
  </button>
</template>

<style scoped>@import '../assets/app-sidebar.css';</style>
