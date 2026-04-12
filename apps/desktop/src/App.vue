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
  <div class="app-container">
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
    <div class="main-area">
      <main class="main-content">
        <Transition name="view-fade" mode="out-in">
          <!-- Chat View when connected and not showing workflows -->
          <ChatView
            v-if="isConnected && !showWorkflows && !showPipelines && !showGate && !showGraph"
            :key="sessionStore.activeSessionId ?? 'active-session'"
          />
          
          <!-- Workflow Authoring Page -->
          <div v-else-if="showWorkflows" key="workflows" class="workflow-panel">
            <div class="workflow-panel-header">
              <span>Workflows</span>
              <button class="workflow-close-btn" @click="closeWorkflowPanel" title="Close">×</button>
            </div>
            <div class="workflow-panel-body">
              <WorkflowAuthoringPage />
            </div>
          </div>

          <div v-else-if="showPipelines" key="pipelines" class="pipeline-panel">
            <div class="pipeline-panel-header">
              <span>Pipelines</span>
              <button class="pipeline-close-btn" @click="showPipelines = false" title="Close">×</button>
            </div>
            <PipelineView />
          </div>

          <div v-else-if="showGate" key="gate" class="gate-panel">
            <div class="gate-panel-header">
              <span>Gate</span>
              <button class="gate-close-btn" @click="showGate = false" title="Close">×</button>
            </div>
            <GateView :cwd="selectedCwd" />
          </div>

          <div v-else-if="showGraph" key="graph" class="graph-panel">
            <div class="graph-panel-header">
              <span>Graph</span>
              <button class="graph-close-btn" @click="showGraph = false" title="Close">×</button>
            </div>
            <GraphView />
          </div>
           
          <!-- Welcome screen when not connected -->
          <div v-else key="welcome" class="welcome-screen">
            <h2>Welcome to maenifold</h2>
            <p>Select an agent and create a new session to get started.</p>
            <p class="manifesto-quote">The space between the notes is where the music lives.</p>
            <p v-if="!hasAgents" class="hint">
              Configure agents in your config file to begin.
            </p>
          </div>
        </Transition>
      </main>
      
      <!-- Traffic Monitor Panel -->
      <div v-if="showTrafficMonitor" class="traffic-panel">
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

<style>@import './assets/design-tokens.css'; @import './assets/shared-components.css';</style>
<style scoped>@import './assets/app-layout.css'; @import './assets/app-panels.css';</style>
