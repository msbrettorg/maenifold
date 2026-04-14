<script setup lang="ts">
import { useConfigStore } from './stores/config';
import { useSessionStore } from './stores/session';
import { useAppShell } from './composables/useAppShell';
import AppSidebar from './components/AppSidebar.vue';
import AppTabBar from './components/AppTabBar.vue';
import ChatView from './components/ChatView.vue';
import WorkflowAuthoringPage from './components/WorkflowAuthoringPage.vue';
import PipelineView from './components/PipelineView.vue';
import GateView from './components/GateView.vue';
import GraphView from './components/GraphView.vue';
import PermissionDialog from './components/PermissionDialog.vue';
import SettingsView from './components/SettingsView.vue';
import AuthMethodDialog from './components/AuthMethodDialog.vue';
import TrafficMonitor from './components/TrafficMonitor.vue';
import ToastContainer from './components/ToastContainer.vue';

const configStore = useConfigStore();
const sessionStore = useSessionStore();

const {
  selectedAgent,
  selectedCwd,
  showSidebar,
  showSettings,
  showTrafficMonitor,
    showStartupDetails,
    showWorkflows,
    showPipelines,
    showGate,
    showGraph,
    activeTab,
    isConnected,
    isConnecting,
  hasAgents,
  pendingPermission,
  pendingPermissionCount,
  pendingAuthMethods,
  pendingAuthAgentName,
  handleAgentSelect,
  handleSelectFolder,
  handleNewSession,
  handleResumeSession,
  handleDeleteSession,
  handleCancelConnection,
  handlePermissionSelect,
  handlePermissionCancel,
  handleAuthMethodSelect,
  handleAuthMethodCancel,
} = useAppShell({ configStore, sessionStore });

function closeWorkflowPanel() {
  showWorkflows.value = false
}
</script>

<template>
  <div class="relative box-border flex h-screen overflow-hidden pt-10">
    <AppTabBar v-model="activeTab" />

    <AppSidebar
      v-if="activeTab === 'agents'"
      v-model:visible="showSidebar"
      v-model:selected-agent="selectedAgent"
      v-model:show-traffic-monitor="showTrafficMonitor"
      v-model:show-startup-details="showStartupDetails"
      :selected-cwd="selectedCwd"
      :has-agents="hasAgents"
      :is-connecting="isConnecting"
      :startup-phase="sessionStore.startupPhase"
      :startup-logs="sessionStore.startupLogs"
      :startup-elapsed="sessionStore.startupElapsed"
      @open-settings="showSettings = true"
      @agent-select="handleAgentSelect"
      @select-folder="handleSelectFolder"
      @new-session="handleNewSession"
      @resume-session="handleResumeSession"
      @delete-session="handleDeleteSession"
      @cancel-connection="handleCancelConnection"
    />

    <!-- Main Content Area -->
    <div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      <main class="flex min-h-0 flex-1 flex-col overflow-hidden bg-[var(--background)]">
        <Transition name="view-fade" mode="out-in">
          <!-- Chat View when connected and not showing workflows -->
          <ChatView
            v-if="isConnected && !showWorkflows && !showPipelines && !showGate && !showGraph"
            :key="sessionStore.activeSessionId ?? 'active-session'"
          />
          
          <!-- Workflow Authoring Page -->
          <div v-else-if="showWorkflows" key="workflows" class="flex flex-1 flex-col overflow-hidden">
            <div
              class="flex shrink-0 items-center justify-between border-b border-[var(--border)] px-4 py-3 text-[var(--text-body-sm)] font-[var(--weight-display)] text-[var(--muted-foreground)]"
            >
              <span>Workflows</span>
              <button
                class="rounded px-2 py-1 text-[var(--text-title)] leading-none text-[var(--muted-foreground)] transition-colors hover:bg-[var(--muted)] hover:text-[var(--foreground)]"
                @click="closeWorkflowPanel"
                title="Close"
              >
                ×
              </button>
            </div>
            <div class="workflow-panel-body flex flex-1 overflow-auto p-4">
              <WorkflowAuthoringPage />
            </div>
          </div>

          <div v-else-if="showPipelines" key="pipelines" class="flex flex-1 flex-col overflow-hidden">
            <div
              class="flex shrink-0 items-center justify-between border-b border-[var(--border)] px-4 py-3 text-[var(--text-body-sm)] font-[var(--weight-display)] text-[var(--muted-foreground)]"
            >
              <span>Pipelines</span>
              <button
                class="rounded px-2 py-1 text-[var(--text-title)] leading-none text-[var(--muted-foreground)] transition-colors hover:bg-[var(--muted)] hover:text-[var(--foreground)]"
                @click="showPipelines = false"
                title="Close"
              >
                ×
              </button>
            </div>
            <PipelineView />
          </div>

          <div v-else-if="showGate" key="gate" class="flex flex-1 flex-col overflow-hidden">
            <div
              class="flex shrink-0 items-center justify-between border-b border-[var(--border)] px-4 py-3 text-[var(--text-body-sm)] font-[var(--weight-display)] text-[var(--muted-foreground)]"
            >
              <span>Gate</span>
              <button
                class="rounded px-2 py-1 text-[var(--text-title)] leading-none text-[var(--muted-foreground)] transition-colors hover:bg-[var(--muted)] hover:text-[var(--foreground)]"
                @click="showGate = false"
                title="Close"
              >
                ×
              </button>
            </div>
            <GateView :cwd="selectedCwd" />
          </div>

          <div v-else-if="showGraph" key="graph" class="flex flex-1 flex-col overflow-hidden">
            <div
              class="flex shrink-0 items-center justify-between border-b border-[var(--border)] px-4 py-3 text-[var(--text-body-sm)] font-[var(--weight-display)] text-[var(--muted-foreground)]"
            >
              <span>Graph</span>
              <button
                class="rounded px-2 py-1 text-[var(--text-title)] leading-none text-[var(--muted-foreground)] transition-colors hover:bg-[var(--muted)] hover:text-[var(--foreground)]"
                @click="showGraph = false"
                title="Close"
              >
                ×
              </button>
            </div>
            <GraphView />
          </div>
            
          <!-- Welcome screen when not connected -->
          <div
            v-else
            key="welcome"
            class="flex flex-1 flex-col items-center justify-center p-8 text-center text-[var(--muted-foreground)]"
          >
            <h2 class="mb-2 font-[var(--font-display)] tracking-[0.04em] text-[var(--foreground)]">
              Welcome to maenifold
            </h2>
            <p>Select an agent and create a new session to get started.</p>
            <p class="mt-2 text-[var(--text-caption)] italic text-[var(--muted-foreground)]">
              The space between the notes is where the music lives.
            </p>
            <p v-if="!hasAgents" class="mt-4 text-[var(--text-caption)] text-[var(--muted-foreground)]">
              Configure agents in your config file to begin.
            </p>
          </div>
        </Transition>
      </main>
      
      <!-- Traffic Monitor Panel -->
      <div v-if="showTrafficMonitor" class="shrink-0 border-t-2 border-[var(--border)]">
        <TrafficMonitor @close="showTrafficMonitor = false" />
      </div>
    </div>
    
    <!-- Permission Dialog -->
    <PermissionDialog 
      v-if="pendingPermission"
      :request="pendingPermission"
      :total-pending="pendingPermissionCount"
      @select="handlePermissionSelect"
      @cancel="handlePermissionCancel"
    />

    <!-- Auth Method Dialog -->
    <AuthMethodDialog 
      v-if="pendingAuthMethods.length > 0"
      :auth-methods="pendingAuthMethods"
      :agent-name="pendingAuthAgentName"
      @select="handleAuthMethodSelect"
      @cancel="handleAuthMethodCancel"
    />

    <!-- Settings -->
    <SettingsView 
      v-if="showSettings"
      @close="showSettings = false"
    />

    <ToastContainer />
  </div>
</template>

<style scoped>
.view-fade-enter-active,
.view-fade-leave-active {
  transition: opacity 200ms var(--easing-standard);
}

.view-fade-enter-from,
.view-fade-leave-to {
  opacity: 0;
}

.workflow-panel-body {
  scrollbar-width: thin;
  scrollbar-color: var(--border) transparent;
}

.workflow-panel-body::-webkit-scrollbar {
  width: 8px;
}

.workflow-panel-body::-webkit-scrollbar-track {
  background: transparent;
}

.workflow-panel-body::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 4px;
}

.workflow-panel-body::-webkit-scrollbar-thumb:hover {
  background: var(--muted-foreground);
}
</style>
