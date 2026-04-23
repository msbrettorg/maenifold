<script setup lang="ts">
import { onMounted, reactive, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { useOnboarding } from './composables/useOnboarding';
import OnboardingWizard from './components/OnboardingWizard.vue';
import { useConfigStore } from './stores/config';
import { useSessionStore } from './stores/session';
import { useFolderPickerStore } from './stores/folder-picker';
import { useAppShellStore } from './stores/app-shell';
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
import { MessageSquare } from 'lucide-vue-next';
import { Toaster } from 'vue-sonner';
import type { AppShellTab } from './stores/app-shell';

const configStore = useConfigStore();
const sessionStore = useSessionStore();
const folderPickerStore = useFolderPickerStore();
const appShellStore = useAppShellStore();
const { isOpen: isFolderPickerOpen, initialPath: folderPickerInitialPath } = storeToRefs(folderPickerStore);
const { activeTab } = storeToRefs(appShellStore);

const {
  selectedAgent,
  selectedCwd,
  showSidebar,
  showTrafficMonitor,
  showStartupDetails,
  showHome,
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
  appShellStore.setActiveTab(tab);
}

// Lazy-mount tabs on first visit, then keep them alive via v-show so we avoid
// repeated mount/unmount churn (which previously caused GPU context thrash and
// crashes when WebGL-backed views were destroyed). Home is always pre-mounted
// because it is the default landing tab and is also rendered in the
// "showHome" overlay state regardless of activeTab.
const visitedTabs = reactive(new Set<AppShellTab>(['home']));

watch(activeTab, (tab) => {
  visitedTabs.add(tab);
}, { immediate: true });

async function handlePromptSubmit(prompt: string): Promise<void> {
  const trimmedPrompt = prompt.trim();
  if (!trimmedPrompt) {
    return;
  }

  if (!sessionStore.isConnected && !sessionStore.isConnecting) {
    await handleNewSession();
  }

  if (sessionStore.isConnected) {
    const success = await sessionStore.sendPrompt(trimmedPrompt);
    if (!success) {
      // Surface failure to caller so post-send cleanup (e.g., clearing the
      // composer input) is skipped and the user can retry without losing input.
      return;
    }
  }
}

const { isFirstRun, checkFirstRun } = useOnboarding();

onMounted(() => {
  checkFirstRun();
});

function onOnboardingComplete(): void {
  // Re-sync local gate state from persisted storage. The wizard's composable
  // instance and this composable instance hold independent refs, so we re-read
  // to ensure the gate flips after the wizard persists completion.
  checkFirstRun();
}
</script>

<template>
  <OnboardingWizard v-if="isFirstRun" @complete="onOnboardingComplete" />
  <div v-else class="relative box-border flex h-screen flex-col overflow-hidden">
    <AppTabBar :model-value="activeTab" @update:model-value="appShellStore.setActiveTab($event)" />

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
          v-show="showHome"
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
          v-show="!showHome && activeTab === 'home'"
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
          v-if="visitedTabs.has('agents')"
          v-show="!showHome && activeTab === 'agents' && isConnected"
          :key="sessionStore.activeSessionId ?? 'active-session'"
        />

        <div v-if="visitedTabs.has('workflows')" v-show="!showHome && activeTab === 'workflows'" class="flex flex-1 flex-col overflow-hidden">
          <div class="flex flex-1 overflow-auto p-4">
            <WorkflowAuthoringPage />
          </div>
        </div>

        <div v-if="visitedTabs.has('pipelines')" v-show="!showHome && activeTab === 'pipelines'" class="flex flex-1 flex-col overflow-hidden">
          <PipelineView />
        </div>

        <div v-if="visitedTabs.has('gate')" v-show="!showHome && activeTab === 'gate'" class="flex flex-1 flex-col overflow-hidden">
          <GateView :cwd="selectedCwd" />
        </div>

        <div v-if="visitedTabs.has('knowledge')" v-show="!showHome && activeTab === 'knowledge'" class="flex flex-1 flex-col overflow-hidden">
          <KnowledgeExplorer />
        </div>

        <div v-if="visitedTabs.has('settings')" v-show="!showHome && activeTab === 'settings'" class="flex flex-1 flex-col overflow-hidden">
          <SettingsView />
        </div>

        <EmptyState
          v-if="visitedTabs.has('agents')"
          v-show="!showHome && activeTab === 'agents' && !isConnected"
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

    <Toaster
      theme="dark"
      rich-colors
      :style="{
        '--normal-bg': 'var(--color-neutral-background-3)',
        '--normal-text': 'var(--color-neutral-foreground-1)',
        '--normal-border': 'var(--color-neutral-stroke-1)',
        '--border-radius': '4px',
      }"
      :toast-options="{ classes: { toast: 'rounded-[4px]' } }"
    />
  </div>
</template>
