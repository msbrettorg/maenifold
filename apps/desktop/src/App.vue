<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue';
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
import KnowledgeWorkspace from './components/KnowledgeWorkspace.vue';
import WorkflowWorkspace from './components/WorkflowWorkspace.vue';
import PipelineView from './components/PipelineView.vue';
import GateView from './components/GateView.vue';
import SchedulesTab from './components/SchedulesTab.vue';
import PermissionDecisionDialog from './components/PermissionDecisionDialog.vue';
import PermissionManagementDialog from './components/PermissionManagementDialog.vue';
import PermissionGhostNotifications from './components/PermissionGhostNotifications.vue';
import DaemonBanner from './components/DaemonBanner.vue';
import SettingsView from './components/SettingsView.vue';
import EmptyState from './components/EmptyState.vue';
import SessionConfigPanel from './components/SessionConfigPanel.vue';
import AuthMethodDialog from './components/AuthMethodDialog.vue';
import FolderPickerDialog from './components/FolderPickerDialog.vue';
import ConfirmDialog from './components/ConfirmDialog.vue';
import PromptDialog from './components/PromptDialog.vue';
import UserPromptHost from './components/UserPromptHost.vue';
import TrafficMonitor from './components/TrafficMonitor.vue';
import { useGlobalKeymap } from './composables/useGlobalKeymap';
import { Loader2, MessageSquare } from 'lucide-vue-next';
import { Toaster } from 'vue-sonner';
import { useDaemonHealth } from './composables/useDaemonHealth';
import type { AppShellTab } from './stores/app-shell';

const configStore = useConfigStore();
const sessionStore = useSessionStore();
const daemonHealth = useDaemonHealth();
const folderPickerStore = useFolderPickerStore();
const appShellStore = useAppShellStore();
useGlobalKeymap();
const permissionManagementOpen = ref(false);
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
// because it is the default landing tab and must preserve composer state across
// session lifecycle changes.
const visitedTabs = reactive(new Set<AppShellTab>(['home']));

watch(activeTab, (tab) => {
  visitedTabs.add(tab);
}, { immediate: true });

async function handlePromptSubmit(prompt: string): Promise<boolean> {
  const trimmedPrompt = prompt.trim();
  if (!trimmedPrompt) {
    return false;
  }

  if (!sessionStore.isConnected && !sessionStore.isConnecting) {
    await handleNewSession();
  }

  if (!sessionStore.isConnected) {
    return false;
  }

  return sessionStore.sendPrompt(trimmedPrompt);
}

const { isFirstRun, checkFirstRun, checkFirstRunWithConfig } = useOnboarding();

onMounted(async () => {
  daemonHealth.start();
  await checkFirstRunWithConfig();
  window.addEventListener('keydown', handlePermissionManagementShortcut);
});

onBeforeUnmount(() => {
  daemonHealth.stop();
  window.removeEventListener('keydown', handlePermissionManagementShortcut);
});

function handlePermissionManagementShortcut(event: KeyboardEvent): void {
  if (!event.metaKey || !event.shiftKey || event.key.toLowerCase() !== 'p') {
    return;
  }
  event.preventDefault();
  permissionManagementOpen.value = true;
}

function onOnboardingComplete(): void {
  // Re-sync local gate state from persisted storage. The wizard's composable
  // instance and this composable instance hold independent refs, so we re-read
  // to ensure the gate flips after the wizard persists completion.
  checkFirstRun();
}

// Source-based scoping: the Agents tab's ChatView only renders when the
// active session was created from that tab.
const activeSessionSource = computed(() => sessionStore.activeSession?.source ?? null);
const showAgentsChat = computed(
  () => isConnected.value && activeSessionSource.value === 'agents'
);
const agentsEmptyTitle = computed(() =>
  sessionStore.error ? 'Session failed to resume' : 'No active session'
);
const agentsEmptyDescription = computed(() =>
  sessionStore.error ?? 'Select an agent and start a new session'
);
</script>

<template>
  <div v-if="isFirstRun === null" class="grid h-screen place-items-center bg-bg-app">
    <Loader2 class="h-6 w-6 animate-spin text-text-secondary" />
  </div>
  <OnboardingWizard v-else-if="isFirstRun" @complete="onOnboardingComplete" />
  <div v-else class="relative box-border flex h-screen flex-col overflow-hidden">
    <DaemonBanner />
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
        <main class="flex min-h-0 w-full flex-1 flex-col overflow-hidden bg-bg-app">
        <HomeDashboard
          v-show="activeTab === 'home'"
          key="home"
          v-model:selected-agent="selectedAgent"
          :has-agents="hasAgents"
          :selected-cwd="selectedCwd"
          :is-connected="isConnected"
          :is-connecting="isConnecting"
          :recent-sessions="sessionStore.savedSessions"
          :submit-prompt="handlePromptSubmit"
          @agent-select="handleAgentSelect"
          @select-folder="handleSelectFolder"
          @new-session="handleNewSession"
          @open-tab="handleOpenTab"
          @resume-session="handleResumeSession"
        />

        <ChatView
          v-if="visitedTabs.has('agents')"
          v-show="!showHome && activeTab === 'agents' && showAgentsChat"
          :key="sessionStore.activeSessionId ?? 'active-session'"
        />

        <div v-if="visitedTabs.has('pipelines')" v-show="!showHome && activeTab === 'pipelines'" class="flex flex-1 flex-col overflow-hidden">
          <PipelineView />
        </div>

        <div v-if="visitedTabs.has('gate')" v-show="!showHome && activeTab === 'gate'" class="flex flex-1 flex-col overflow-hidden">
          <GateView :cwd="selectedCwd" />
        </div>

        <div v-if="visitedTabs.has('knowledge')" v-show="!showHome && activeTab === 'knowledge'" class="flex flex-1 flex-col overflow-hidden">
          <KnowledgeWorkspace />
        </div>

        <div v-if="visitedTabs.has('workflows')" v-show="!showHome && activeTab === 'workflows'" class="flex flex-1 flex-col overflow-hidden">
          <WorkflowWorkspace />
        </div>

        <div v-if="visitedTabs.has('schedules')" v-show="!showHome && activeTab === 'schedules'" class="flex flex-1 flex-col overflow-hidden">
          <SchedulesTab />
        </div>

        <div v-if="visitedTabs.has('settings')" v-show="!showHome && activeTab === 'settings'" class="flex flex-1 flex-col overflow-hidden">
          <SettingsView />
        </div>

        <div
          v-if="visitedTabs.has('agents')"
          v-show="!showHome && activeTab === 'agents' && !showAgentsChat"
          class="flex flex-1 flex-col overflow-hidden"
        >
          <SessionConfigPanel
            v-if="selectedAgent"
            :agent-name="selectedAgent"
            :cwd="selectedCwd"
            :is-read-only="false"
          />
          <EmptyState
            :icon="MessageSquare"
            :title="agentsEmptyTitle"
            :description="agentsEmptyDescription"
          />
        </div>
        </main>

        <!-- Traffic Monitor Panel -->
        <div v-if="showTrafficMonitor" class="shrink-0">
          <TrafficMonitor @close="showTrafficMonitor = false" />
        </div>
      </div>
    </div>

    <!-- Permission Dialog -->
    <PermissionDecisionDialog
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
    <UserPromptHost />
    <PermissionManagementDialog
      :open="permissionManagementOpen"
      @close="permissionManagementOpen = false"
    />
    <PermissionGhostNotifications />

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
