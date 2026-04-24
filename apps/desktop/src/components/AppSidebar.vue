<script setup lang="ts">
import AgentSelector from './AgentSelector.vue';
import SessionList from './SessionList.vue';
import StartupProgress from './StartupProgress.vue';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useResizeHandle } from '../composables/useResizeHandle';
import { getSidebarSizingTokens } from '../lib/sidebarSizing';
import type { SavedSession } from '../lib/types';
import { Radio, PanelLeftClose, PanelLeftOpen, FolderOpen } from 'lucide-vue-next';

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
  <aside
    v-if="visible"
    class="flex shrink-0 flex-col bg-surface text-text-primary"
    role="complementary"
    aria-label="Agents panel"
    :style="{ width: resize.width.value + 'px', minWidth: resize.width.value + 'px' }"
  >
    <div class="flex items-center justify-between px-4 pt-3 pb-2">
      <h2 class="m-0 text-xs font-medium tracking-[0.02em] text-text-secondary">Agents</h2>
      <div class="flex gap-1">
        <Button
          variant="ghost"
          size="icon-sm"
          :class="showTrafficMonitor && 'bg-surface-high text-brand'"
          @click="showTrafficMonitor = !showTrafficMonitor"
          title="ACP Traffic Monitor"
          aria-label="Toggle ACP traffic monitor"
        ><Radio class="size-4" /></Button>
        <Button
          variant="ghost"
          size="icon-sm"
          @click="visible = false"
          title="Collapse sidebar"
          aria-label="Collapse sidebar"
        ><PanelLeftClose class="size-4" /></Button>
      </div>
    </div>

    <ScrollArea class="flex-1">
      <div class="flex flex-col gap-1">
      <!-- Agent Selection -->
      <div class="px-4 pt-1 pb-3">
        <AgentSelector
          v-model:selected="selectedAgent"
          @select="(name: string) => emit('agent-select', name)"
        />

        <!-- Working Directory Picker -->
        <div class="mt-3">
          <label class="mb-1.5 block text-xs font-normal tracking-normal text-text-secondary">
            Working directory
          </label>
          <div class="flex items-center gap-2">
            <span
              class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap rounded bg-surface-container px-2.5 py-2 text-xs text-text-primary"
              :title="selectedCwd || 'Current directory'"
            >
              {{ selectedCwd ? selectedCwd.split(/[\\/]/).pop() : '.' }}
            </span>
            <Button
              variant="ghost"
              size="icon-sm"
              class="bg-surface-container"
              @click="emit('select-folder')"
              title="Select folder"
              aria-label="Select folder"
            >
              <FolderOpen class="size-4" />
            </Button>
          </div>
        </div>

        <Button
          v-if="hasAgents"
          variant="default"
          class="mt-4 w-full"
          :disabled="!selectedAgent || isConnecting"
          @click="emit('new-session')"
        >
          New session
        </Button>

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
      <div class="px-4 pt-2 pb-4">
        <SessionList
          @resume="(session: SavedSession) => emit('resume-session', session)"
          @delete="(id: string) => emit('delete-session', id)"
        />
      </div>
      </div>
    </ScrollArea>
  </aside>

  <!-- Sidebar resize handle -->
  <div
    v-if="visible"
    class="relative z-10 -mx-[3px] w-[6px] shrink-0 cursor-col-resize transition-colors hover:bg-brand active:bg-brand"
    @mousedown="resize.startResize($event, 'right')"
  />

  <!-- Collapsed sidebar toggle -->
  <Button
    v-if="!visible"
    variant="ghost"
    size="icon"
    class="fixed top-1/2 left-0 z-50 -translate-y-1/2 rounded-l-none rounded-r bg-surface-container text-text-secondary hover:bg-surface-high hover:text-text-primary"
    @click="visible = true"
    title="Expand sidebar"
    aria-label="Expand sidebar"
  >
    <PanelLeftOpen class="size-4" />
  </Button>
</template>
