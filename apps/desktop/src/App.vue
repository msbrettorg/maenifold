<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { useConfigStore } from './stores/config';
import { useSessionStore } from './stores/session';
import { useFolderPickerStore } from './stores/folder-picker';
import { useAppShell } from './composables/useAppShell';
import AppSidebar from './components/AppSidebar.vue';
import AppTabBar from './components/AppTabBar.vue';
import ChatView from './components/ChatView.vue';
import HomeDashboard from './components/HomeDashboard.vue';
import KnowledgeExplorer from './components/KnowledgeExplorer.vue';
import WorkflowAuthoringPage from './components/WorkflowAuthoringPage.vue';
import PipelineView from './components/PipelineView.vue';
import GateView from './components/GateView.vue';
import PermissionDialog from './components/PermissionDialog.vue';
import SettingsView from './components/SettingsView.vue';
import EmptyState from './components/EmptyState.vue';
import AuthMethodDialog from './components/AuthMethodDialog.vue';
import FolderPickerDialog from './components/FolderPickerDialog.vue';
import ConfirmDialog from './components/ConfirmDialog.vue';
import PromptDialog from './components/PromptDialog.vue';
import TrafficMonitor from './components/TrafficMonitor.vue';
import ToastContainer from './components/ToastContainer.vue';
import type { AppShellTab } from './stores/app-shell';
import { MessageSquare } from 'lucide-vue-next';

const configStore = useConfigStore();
const sessionStore = useSessionStore();
const folderPickerStore = useFolderPickerStore();
const { isOpen: isFolderPickerOpen, initialPath: folderPickerInitialPath } = storeToRefs(folderPickerStore);

const {
  selectedAgent,
  selectedCwd,
  showSidebar,
  showTrafficMonitor,
  showStartupDetails,
  showHome,
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

function handleOpenTab(tab: AppShellTab): void {
  activeTab.value = tab;
}

async function handlePromptSubmit(prompt: string): Promise<void> {
  const trimmedPrompt = prompt.trim();
  if (!trimmedPrompt) {
    return;
  }

  if (!sessionStore.isConnected && !sessionStore.isConnecting) {
    await handleNewSession();
  }

  if (sessionStore.isConnected) {
    await sessionStore.sendPrompt(trimmedPrompt);
  }
}
</script>

<template>
  <div class="relative box-border flex h-screen flex-col overflow-hidden">
    <AppTabBar v-model="activeTab" />

    <div class="flex min-h-0 flex-1 overflow-hidden">
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
        @agent-select="handleAgentSelect"
        @select-folder="handleSelectFolder"
        @new-session="handleNewSession"
        @resume-session="handleResumeSession"
        @delete-session="handleDeleteSession"
        @cancel-connection="handleCancelConnection"
      />

      <!-- Main Content Area -->
      <div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        <main class="flex min-h-0 flex-1 flex-col overflow-hidden bg-bg-app">
        <HomeDashboard
          v-if="showHome"
          key="home"
          v-model:selected-agent="selectedAgent"
          :has-agents="hasAgents"
          :selected-cwd="selectedCwd"
          :is-connected="isConnected"
          :is-connecting="isConnecting"
          :recent-sessions="sessionStore.savedSessions"
          @agent-select="handleAgentSelect"
          @select-folder="handleSelectFolder"
          @new-session="handleNewSession"
          @prompt-submit="handlePromptSubmit"
          @open-tab="handleOpenTab"
          @resume-session="handleResumeSession"
        />

        <HomeDashboard
          v-else-if="activeTab === 'home'"
          key="home-with-active-session"
          v-model:selected-agent="selectedAgent"
          :has-agents="hasAgents"
          :selected-cwd="selectedCwd"
          :is-connected="isConnected"
          :is-connecting="isConnecting"
          :recent-sessions="sessionStore.savedSessions"
          @agent-select="handleAgentSelect"
          @select-folder="handleSelectFolder"
          @new-session="handleNewSession"
          @prompt-submit="handlePromptSubmit"
          @open-tab="handleOpenTab"
          @resume-session="handleResumeSession"
        />

        <ChatView
          v-else-if="activeTab === 'agents' && isConnected"
          :key="sessionStore.activeSessionId ?? 'active-session'"
        />

        <div v-else-if="activeTab === 'workflows'" class="flex flex-1 flex-col overflow-hidden">
          <div class="flex flex-1 overflow-auto p-4">
            <WorkflowAuthoringPage />
          </div>
        </div>

        <div v-else-if="activeTab === 'pipelines'" class="flex flex-1 flex-col overflow-hidden">
          <PipelineView />
        </div>

        <div v-else-if="activeTab === 'gate'" class="flex flex-1 flex-col overflow-hidden">
          <GateView :cwd="selectedCwd" />
        </div>

        <div v-else-if="activeTab === 'knowledge'" class="flex flex-1 flex-col overflow-hidden">
          <KnowledgeExplorer />
        </div>

        <div v-else-if="activeTab === 'settings'" class="flex flex-1 flex-col overflow-hidden">
          <SettingsView />
        </div>

        <EmptyState
          v-else-if="activeTab === 'agents'"
          :icon="MessageSquare"
          title="No active session"
          description="Select an agent and start a new session"
        />
        </main>

        <!-- Traffic Monitor Panel -->
        <div v-if="showTrafficMonitor" class="shrink-0">
          <TrafficMonitor @close="showTrafficMonitor = false" />
        </div>
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
    <FolderPickerDialog
      :open="isFolderPickerOpen"
      :initial-path="folderPickerInitialPath"
      @confirm="folderPickerStore.confirm"
      @cancel="folderPickerStore.cancel"
    />
    <ConfirmDialog />
    <PromptDialog />

    <ToastContainer />
  </div>
</template>
