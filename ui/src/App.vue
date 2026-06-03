<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue';
import { canPickFolder, pickFolder, loadKvStore, type KVStore } from './lib/host';
import { useConfigStore } from './stores/config';
import { useEmbeddingsStore } from './stores/embeddings';
import { useSessionStore } from './stores/session';
import { useAppShellStore } from './stores/appShell';
import { initTelemetry } from './lib/telemetry';
import AgentSelector from './components/AgentSelector.vue';
import SessionList from './components/SessionList.vue';
import ChatView from './components/ChatView.vue';
import PermissionDialog from './components/PermissionDialog.vue';
import SettingsView from './components/SettingsView.vue';
import AuthMethodDialog from './components/AuthMethodDialog.vue';
import TrafficMonitor from './components/TrafficMonitor.vue';
import StartupProgress from './components/StartupProgress.vue';
import TabBar from './components/TabBar.vue';
import HomeView from './components/HomeView.vue';
import KnowledgeView from './components/KnowledgeView.vue';
import SchedulesView from './components/SchedulesView.vue';
import PipelinesView from './components/PipelinesView.vue';
import OnboardingWizard, {
  needsOnboarding,
  type OnboardingConfig,
  type OnboardingEmbedStatus,
} from './components/OnboardingWizard.vue';
import type { SavedSession } from './lib/types';

const configStore = useConfigStore();
const embeddingsStore = useEmbeddingsStore();
const sessionStore = useSessionStore();
const shell = useAppShellStore();

// Keep a live session visible: whenever a session becomes active or starts
// (re)connecting, surface the Agents tab so the chat / startup progress is
// never hidden behind the Home landing page.
watch(
  () => sessionStore.isConnected || sessionStore.isConnecting || sessionStore.isReconnecting,
  (active) => {
    if (active) shell.setActiveTab('agents');
  }
);

const selectedAgent = ref('');
const selectedCwd = ref('');
// On mobile / web there is no native folder picker (and the cwd refers to
// a path on the *agent's* machine, not the local device), so we expose a
// free-text field instead of the picker button.
const folderPickerAvailable = canPickFolder();
const showSidebar = ref(true);
const showTrafficMonitor = ref(false);
const showStartupDetails = ref(false);

// Embedded at build time by Vite. Used in the welcome hero pill so the
// version appears as soon as the bundle loads (no async getAppVersion()
// round-trip needed).
const APP_VERSION = '0.1.16';

// Reactive flag tracking whether the viewport is narrow enough to show the
// sidebar as a slide-in drawer (mobile / very narrow desktop windows). Used
// by the template to decide when the backdrop is interactive and by
// onMounted to default the drawer closed.
const isNarrowLayout = ref(false);
let narrowMql: MediaQueryList | null = null;
function syncNarrowLayout() {
  if (narrowMql) isNarrowLayout.value = narrowMql.matches;
}

// Foreground-reconnect plumbing. Mobile OSes freeze the WebView when the
// app is backgrounded and routers may drop the idle TCP connection while
// we're away. When the user returns we ask the session store to silently
// reattach to the last session if we have one.
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
function scheduleReconnect() {
  // Coalesce rapid visibility/online flips into a single attempt.
  if (reconnectTimer) return;
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    // Skip if the browser still thinks we're offline; we'll be re-triggered
    // by the `online` event when connectivity returns.
    if (typeof navigator !== 'undefined' && navigator.onLine === false) return;
    void sessionStore.tryReconnect();
  }, 250);
}
function handleVisibilityChange() {
  if (typeof document !== 'undefined' && !document.hidden) scheduleReconnect();
}
function handleOnline() {
  scheduleReconnect();
}

// Preferences store for persisting user selections
let prefsStore: KVStore | null = null;

const isConnected = computed(() => sessionStore.isConnected);
const isLoading = computed(() => sessionStore.isLoading);
const isConnecting = computed(() => sessionStore.isConnecting);
const isReconnecting = computed(() => sessionStore.isReconnecting);
const error = computed(() => sessionStore.error || configStore.error);
const hasAgents = computed(() => configStore.hasAgents);

// --- Onboarding wizard wiring -------------------------------------------
// The wizard renders its own full-screen scrim/dialog and self-manages
// visibility (localStorage dismissal + a window visibility event). App.vue
// just mounts it once, seeds its config/embedding snapshots from the existing
// stores, and decides the initial auto-show from `needsOnboarding`.
//
// `config` matches OnboardingConfig (agents map + settings.defaultAgent) and
// `status` matches OnboardingEmbedStatus (activeModelStatus). Both are passed
// live so the wizard seeds from real daemon state; the wizard fetches its own
// fresh data once visible.
const onboardingConfig = computed<OnboardingConfig>(
  () => configStore.config as OnboardingConfig
);
const onboardingEmbedStatus = computed<OnboardingEmbedStatus | null>(
  () => embeddingsStore.status as OnboardingEmbedStatus | null
);
// Gates the wizard mount until the initial setup snapshot has been evaluated,
// so a configured user never sees a wizard flash during store load.
const onboardingReady = ref(false);
// Snapshot of the auto-show decision, captured exactly once after initial
// load. Kept as a non-reactive snapshot (not a live computed) so progressing
// through the wizard — which flips store state to "configured" — does not
// retract `props.open` and close the wizard mid-flow. The wizard itself still
// honours its localStorage dismissal on top of this.
const onboardingInitialOpen = ref(false);

// Name of the agent the reconnect banner refers to. Falls back to the
// generic "agent" if the saved session has no name (shouldn't happen).
const reconnectingAgentName = computed(
  () => sessionStore.currentSession?.agentName ?? 'agent'
);

// True when there is a saved session we *could* reconnect to but the
// transport is currently down. Surfaces a manual "Reconnect" affordance on
// the error banner so users don't have to background+foreground the app to
// trigger the auto path.
const canManuallyReconnect = computed(
  () =>
    !isConnected.value &&
    !isReconnecting.value &&
    !isConnecting.value &&
    !!sessionStore.currentSession?.supportsLoadSession
);

async function handleManualReconnect() {
  await sessionStore.tryReconnect();
}

// Watch for permission requests from session store
const pendingPermission = computed(() => sessionStore.pendingPermission);

// Watch for auth method selection requests
const pendingAuthMethods = computed(() => sessionStore.pendingAuthMethods);
const pendingAuthAgentName = computed(() => sessionStore.pendingAuthAgentName);

onMounted(async () => {
  // Track viewport width so the sidebar can default-collapse into a drawer
  // on phones / narrow windows. We watch a MediaQueryList rather than
  // resize for correctness across orientation changes on iOS.
  if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
    narrowMql = window.matchMedia('(max-width: 800px)');
    syncNarrowLayout();
    narrowMql.addEventListener('change', syncNarrowLayout);
    if (isNarrowLayout.value) showSidebar.value = false;
  }

  // Load persisted preferences first
  prefsStore = await loadKvStore('preferences.json');
  
  // Initialize telemetry (check user preference)
  const telemetryEnabled = await prefsStore.get<boolean>('telemetryEnabled') ?? true;
  await initTelemetry(telemetryEnabled);
  
  // Initialize stores
  await configStore.loadConfig();
  await configStore.setupHotReload();
  await sessionStore.initStore();

  // Seed the embedding status once so the onboarding auto-show decision sees
  // real daemon state. A failure here (daemon down / vec0 missing) is itself a
  // signal that setup is incomplete, so we leave `status` null and let
  // `needsOnboarding` treat it as "needs setup".
  try {
    await embeddingsStore.fetchStatus();
  } catch {
    // Non-fatal: needsOnboarding(null) -> true, which is the correct default
    // for a fresh / unhealthy install.
  }

  // Decide the initial auto-show exactly once, then mount the wizard. The
  // wizard suppresses itself when its localStorage dismissal flag is set, so a
  // user who already finished setup is not re-prompted across relaunches.
  onboardingInitialOpen.value = needsOnboarding(
    onboardingConfig.value,
    onboardingEmbedStatus.value
  );
  onboardingReady.value = true;

  const savedCwd = await prefsStore.get<string>('lastCwd');
  if (savedCwd) {
    selectedCwd.value = savedCwd;
  }

  // Hook foreground-reconnect listeners. `pageshow` fires both on initial
  // navigation and when iOS restores a frozen WebView from the back/forward
  // cache, so it complements `visibilitychange` on Safari/iOS.
  if (typeof document !== 'undefined') {
    document.addEventListener('visibilitychange', handleVisibilityChange);
  }
  if (typeof window !== 'undefined') {
    window.addEventListener('pageshow', scheduleReconnect);
    window.addEventListener('online', handleOnline);
  }
});

async function handleAgentSelect(agentName: string) {
  selectedAgent.value = agentName;
}

async function handleSelectFolder() {
  const folder = await pickFolder('Select Working Directory');
  if (folder) {
    selectedCwd.value = folder;
    // Persist the selection
    if (prefsStore) {
      await prefsStore.set('lastCwd', folder);
      await prefsStore.save();
    }
  }
}

/** Persist a typed cwd as the user edits it (mobile / web field). */
async function handleCwdInput(event: Event) {
  const value = (event.target as HTMLInputElement).value;
  selectedCwd.value = value;
  if (prefsStore) {
    await prefsStore.set('lastCwd', value);
    await prefsStore.save();
  }
}

async function handleNewSession() {
  if (!selectedAgent.value) return;

  // ACP requires an absolute working directory; passing '.' is rejected by
  // most agents. On desktop the folder picker always returns an absolute
  // path, but on mobile the user types it, so validate up-front and surface
  // a helpful error rather than letting the agent reject `session/new`.
  const cwd = selectedCwd.value.trim();
  if (!cwd) {
    sessionStore.error = 'Please enter a working directory (absolute path on the agent\u2019s machine).';
    return;
  }
  const isAbsolute = cwd.startsWith('/') || /^[A-Za-z]:[\\/]/.test(cwd);
  if (!isAbsolute) {
    sessionStore.error = `Working directory must be an absolute path (got: ${cwd}).`;
    return;
  }

  try {
    shell.setActiveTab('agents');
    await sessionStore.createSession(selectedAgent.value, cwd);
  } catch (e) {
    console.error('Failed to create session:', e);
  }
}

async function handleResumeSession(session: SavedSession) {
  selectedAgent.value = session.agentName;
  try {
    shell.setActiveTab('agents');
    await sessionStore.resumeSession(session);
  } catch (e) {
    console.error('Failed to resume session:', e);
  }
}

async function handleDeleteSession(sessionId: string) {
  await sessionStore.deleteSession(sessionId);
}

async function handleDisconnect() {
  await sessionStore.disconnect();
}

async function handleCancelConnection() {
  await sessionStore.cancelConnection();
}

function handlePermissionSelect(optionId: string) {
  sessionStore.resolvePermission(optionId);
}

function handlePermissionCancel() {
  sessionStore.cancelPermission();
}

function handleAuthMethodSelect(methodId: string) {
  sessionStore.selectAuthMethod(methodId);
}

function handleAuthMethodCancel() {
  sessionStore.cancelAuthSelection();
}

function toggleSidebar() {
  showSidebar.value = !showSidebar.value;
}

/** Close the drawer when the user taps the backdrop on a narrow viewport. */
function handleBackdropClick() {
  if (isNarrowLayout.value) showSidebar.value = false;
}

onBeforeUnmount(() => {
  if (narrowMql) {
    narrowMql.removeEventListener('change', syncNarrowLayout);
    narrowMql = null;
  }
  if (typeof document !== 'undefined') {
    document.removeEventListener('visibilitychange', handleVisibilityChange);
  }
  if (typeof window !== 'undefined') {
    window.removeEventListener('pageshow', scheduleReconnect);
    window.removeEventListener('online', handleOnline);
  }
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
});

function clearError() {
  sessionStore.clearError();
  configStore.clearError();
}
</script>

<template>
  <div class="app-shell">
    <TabBar />

    <div class="tab-content">
    <!-- Agents tab: the full sidebar + main layout. Rendered with v-show so an
         active session / chat stays mounted while the user visits other tabs. -->
    <div
      v-show="shell.activeTab === 'agents'"
      class="app-container"
      :class="{ 'narrow-layout': isNarrowLayout }"
    >
    <!-- Sidebar (slides in as a drawer on narrow viewports) -->
    <aside
      v-show="showSidebar"
      class="sidebar"
      :class="{ 'is-drawer': isNarrowLayout }"
    >
      <div class="sidebar-header">
        <h1>Maenifold</h1>
        <div class="header-actions">
          <button 
            class="header-icon-btn" 
            :class="{ active: showTrafficMonitor }"
            @click="showTrafficMonitor = !showTrafficMonitor" 
            title="ACP Traffic Monitor"
          >📡</button>
          <button class="toggle-btn" @click="toggleSidebar">◀</button>
        </div>
      </div>
      
      <div class="sidebar-content">
        <!-- Agent Selection -->
        <div class="section">
          <AgentSelector 
            v-model:selected="selectedAgent"
            @select="handleAgentSelect"
          />
          
          <!-- Working Directory Picker -->
          <div class="cwd-picker">
            <label>Working Directory:</label>
            <!-- Desktop: read-only display + folder picker. -->
            <div v-if="folderPickerAvailable" class="cwd-row">
              <span class="cwd-path" :title="selectedCwd || 'Current directory'">
                {{ selectedCwd ? selectedCwd.split(/[\\/]/).pop() : '.' }}
              </span>
              <button 
                class="cwd-btn" 
                @click="handleSelectFolder"
                title="Select folder"
                :disabled="isConnecting || isConnected"
              >
                📁
              </button>
            </div>
            <!-- Mobile / web: free-text input. The cwd is interpreted by
                 the remote agent, so the path must exist on the agent's
                 machine. -->
            <input
              v-else
              class="cwd-input"
              type="text"
              :value="selectedCwd"
              @input="handleCwdInput"
              :disabled="isConnecting || isConnected"
              placeholder="/absolute/path/on/agent"
              autocapitalize="none"
              autocorrect="off"
              spellcheck="false"
            />
          </div>
          
          <button 
            v-if="hasAgents && !isConnected && !isConnecting"
            class="new-session-btn"
            :disabled="!selectedAgent || isLoading"
            @click="handleNewSession"
          >
            {{ isLoading ? 'Connecting...' : 'New Session' }}
          </button>
          
          <!-- Startup Progress -->
          <StartupProgress 
            v-if="isConnecting"
            :agent-name="selectedAgent"
            :phase="sessionStore.startupPhase"
            :logs="sessionStore.startupLogs"
            :elapsed-seconds="sessionStore.startupElapsed"
            :show-details="showStartupDetails"
            @cancel="handleCancelConnection"
            @toggle-details="showStartupDetails = !showStartupDetails"
          />
          
          <button 
            v-if="isConnected"
            class="disconnect-btn"
            @click="handleDisconnect"
          >
            Disconnect
          </button>
        </div>
        
        <!-- Session List -->
        <div class="section">
          <SessionList 
            @resume="handleResumeSession"
            @delete="handleDeleteSession"
          />
        </div>
      </div>
    </aside>
    
    <!-- Backdrop behind the drawer on narrow viewports. Only intercepts
         taps when the layout is narrow; on desktop it's display:none. -->
    <div
      v-show="isNarrowLayout && showSidebar"
      class="drawer-backdrop"
      @click="handleBackdropClick"
    />

    <!-- Mobile hamburger to open the drawer when collapsed. The desktop
         chevron toggle (`.sidebar-toggle-collapsed`) is hidden on narrow
         viewports via CSS so we don't show two affordances. -->
    <button
      v-show="isNarrowLayout && !showSidebar"
      class="mobile-hamburger"
      @click="showSidebar = true"
      aria-label="Open menu"
    >☰</button>

    <!-- Collapsed sidebar toggle -->
    <button 
      v-if="!showSidebar" 
      class="sidebar-toggle-collapsed"
      @click="toggleSidebar"
    >
      ▶
    </button>
    
    <!-- Main Content Area -->
    <div class="main-area">
      <main class="main-content">
        <!-- Reconnect banner takes priority over the error banner: while a
             reconnect is in progress we don't want a contradictory red
             "Connection lost" pill. -->
        <div v-if="isReconnecting" class="reconnect-banner">
          <span class="reconnect-spinner" aria-hidden="true"></span>
          <span class="reconnect-text">
            Reconnecting to <strong>{{ reconnectingAgentName }}</strong>…
          </span>
        </div>

        <!-- Error display (suppressed while reconnecting). -->
        <div v-else-if="error" class="error-banner">
          <span class="error-icon">⚠</span>
          <span class="error-text">{{ error }}</span>
          <button
            v-if="canManuallyReconnect"
            class="error-action"
            @click="handleManualReconnect"
            title="Reconnect"
          >Reconnect</button>
          <button class="error-close" @click="clearError" title="Dismiss">×</button>
        </div>
        
        <!-- Chat View when connected -->
        <ChatView v-if="isConnected" />
        
        <!-- Welcome screen when not connected -->
        <div v-else class="welcome-screen">
          <div class="welcome-hero">
            <!-- Inline Lucide brain glyph: same path as the app icon -->
            <svg
              class="welcome-brain"
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
            >
              <path d="M12 18V5"/>
              <path d="M15 13a4.17 4.17 0 0 1-3-4 4.17 4.17 0 0 1-3 4"/>
              <path d="M17.598 6.5A3 3 0 1 0 12 5a3 3 0 1 0-5.598 1.5"/>
              <path d="M17.997 5.125a4 4 0 0 1 2.526 5.77"/>
              <path d="M18 18a4 4 0 0 0 2-7.464"/>
              <path d="M19.967 17.483A4 4 0 1 1 12 18a4 4 0 1 1-7.967-.517"/>
              <path d="M6 18a4 4 0 0 1-2-7.464"/>
              <path d="M6.003 5.125a4 4 0 0 0-2.526 5.77"/>
            </svg>

            <div class="welcome-identity" aria-label="Application identity">
              <span class="welcome-name">Maenifold</span>
              <span class="welcome-version">v{{ APP_VERSION }}</span>
            </div>

            <h1 class="welcome-headline">What can I help you with?</h1>
            <p class="welcome-sub">
              {{ hasAgents
                  ? 'Pick an agent in the sidebar and start a new session.'
                  : 'Configure agents in ~/maenifold/maenifold.json to begin.' }}
            </p>

            <div class="welcome-actions">
              <button
                v-if="hasAgents"
                class="welcome-action primary"
                :disabled="!selectedAgent || isLoading || isConnecting"
                @click="handleNewSession"
              >
                <span class="welcome-action-icon">💬</span>
                <span class="welcome-action-label">
                  <strong>Start session</strong>
                  <small>{{ selectedAgent
                      ? `Open a new conversation`
                      : 'Choose an agent in the sidebar first' }}</small>
                </span>
              </button>
              <button
                class="welcome-action"
                @click="showTrafficMonitor = true"
              >
                <span class="welcome-action-icon">📡</span>
                <span class="welcome-action-label">
                  <strong>Traffic monitor</strong>
                  <small>Inspect ACP messages</small>
                </span>
              </button>
            </div>
          </div>
        </div>
      </main>
      
      <!-- Traffic Monitor Panel -->
      <div v-if="showTrafficMonitor" class="traffic-panel">
        <TrafficMonitor @close="showTrafficMonitor = false" />
      </div>
    </div>
    </div>
    <!-- /Agents tab -->

      <!-- Home tab -->
      <HomeView
        v-if="shell.activeTab === 'home'"
        :version="APP_VERSION"
        @new-session="handleNewSession"
        @resume-session="handleResumeSession"
      />
      <KnowledgeView v-else-if="shell.activeTab === 'knowledge'" />
      <SettingsView v-else-if="shell.activeTab === 'settings'" />
      <SchedulesView v-else-if="shell.activeTab === 'schedules'" />
      <PipelinesView v-else-if="shell.activeTab === 'pipelines'" />
    </div>
    <!-- /tab-content -->

    <!-- Permission Dialog -->
    <PermissionDialog 
      v-if="pendingPermission"
      :request="pendingPermission"
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

    <!-- First-run / Re-run onboarding wizard. Mounted at the app shell level so
         its full-screen scrim sits above every tab. It self-manages visibility
         (localStorage dismissal + visibility events); `Settings → Re-run setup`
         calls its module-level reopen() to bring it back for configured users. -->
    <OnboardingWizard
      v-if="onboardingReady"
      :open="onboardingInitialOpen"
      :config="onboardingConfig"
      :embedding-status="onboardingEmbedStatus"
    />

  </div>
</template>

<style>
:root {
  --bg-primary: #0066cc;
  --bg-primary-hover: #0052a3;
  --bg-sidebar: #f8f9fa;
  --bg-main: #ffffff;
  --bg-hover: #f0f0f0;
  --bg-user: #e3f2fd;
  --bg-assistant: #f5f5f5;
  --bg-code: #282c34;
  --bg-success: #28a745;
  --bg-danger: #dc3545;
  --bg-warning: #fff3cd;
  --text-primary: #333;
  --text-secondary: #666;
  --text-muted: #999;
  --text-accent: #0066cc;
  --text-code: #abb2bf;
  --border-color: #e0e0e0;
  --tabbar-height: 52px;
  
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
  font-size: 16px;
  line-height: 1.5;
  color: var(--text-primary);
  background-color: #ffffff;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-primary: #4da6ff;
    --bg-primary-hover: #3399ff;
    --bg-sidebar: #1e1e1e;
    --bg-main: #252525;
    --bg-hover: #333;
    --bg-user: #1a3a5c;
    --bg-assistant: #2d2d2d;
    --text-primary: #e0e0e0;
    --text-secondary: #b0b0b0;
    --text-muted: #8a8a8a;
    --text-accent: #4da6ff;
    --border-color: #404040;
    background-color: #252525;
  }
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html, body, #app {
  height: 100%;
}
</style>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

.tab-content {
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
  background:
    linear-gradient(90deg, color-mix(in srgb, var(--border-color) 72%, transparent) 1px, transparent 1px),
    linear-gradient(0deg, color-mix(in srgb, var(--border-color) 72%, transparent) 1px, transparent 1px),
    var(--bg-main);
  background-size: 28px 28px;
}

/* Each tab view fills the content area. */
.tab-content > * {
  flex: 1;
  min-width: 0;
}

.app-container {
  display: flex;
  height: 100%;
  width: 100%;
  overflow: hidden;
  gap: 0.75rem;
  padding: 0.75rem;
}

/* The agent sidebar is a floating pane that matches the Knowledge tree pane
   and the Settings rail: tinted surface, full border, rounded corners, with
   the notebook grid showing only in the gutter around it. */
.sidebar {
  width: 320px;
  min-width: 320px;
  background: color-mix(in srgb, var(--bg-main) 94%, var(--bg-sidebar));
  border: 1px solid var(--border-color);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem;
  border-bottom: 1px solid var(--border-color);
}

.sidebar-header h1 {
  font-size: 1.25rem;
  margin: 0;
}

.header-actions {
  display: flex;
  gap: 0.25rem;
}

.header-icon-btn,
.toggle-btn {
  padding: 0.25rem 0.5rem;
  border: 1px solid transparent;
  background: transparent;
  cursor: pointer;
  font-size: 1rem;
  line-height: 1;
  color: var(--text-secondary);
  border-radius: 4px;
  transition: background 0.12s, color 0.12s, border-color 0.12s;
}

.header-icon-btn:hover,
.toggle-btn:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
  border-color: var(--border-color);
}

.toggle-btn {
  /* The chevron glyph is a touch larger to read clearly at small sizes
     and keeps the same baseline as the gear / antenna emoji siblings. */
  font-size: 1.05rem;
}

.header-icon-btn.active {
  color: var(--text-accent);
  background: var(--bg-hover);
  border-color: var(--border-color);
  border-radius: 4px;
}

.sidebar-content {
  flex: 1;
  overflow-y: auto;
}

.section {
  padding: 1rem;
  border-bottom: 1px solid var(--border-color);
}

.new-session-btn,
.disconnect-btn {
  width: 100%;
  margin-top: 0.75rem;
  padding: 0.625rem 1rem;
  border: none;
  border-radius: 6px;
  font-size: 0.9rem;
  font-weight: 500;
  cursor: pointer;
}

.new-session-btn {
  background: var(--bg-primary);
  color: white;
}

.new-session-btn:hover:not(:disabled) {
  background: var(--bg-primary-hover);
}

.new-session-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.cwd-picker {
  margin-top: 0.75rem;
}

.cwd-picker label {
  display: block;
  font-size: 0.8rem;
  color: var(--text-secondary);
  margin-bottom: 0.25rem;
}

.cwd-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.cwd-path {
  flex: 1;
  padding: 0.375rem 0.5rem;
  background: var(--bg-main);
  border: 1px solid var(--border-color);
  border-radius: 4px;
  font-size: 0.8rem;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cwd-btn {
  padding: 0.375rem 0.5rem;
  border: 1px solid var(--border-color);
  border-radius: 4px;
  background: transparent;
  cursor: pointer;
  font-size: 1rem;
}

.cwd-btn:hover:not(:disabled) {
  background: var(--bg-hover);
}

.cwd-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Mobile-only free-text cwd input. */
.cwd-input {
  width: 100%;
  padding: 0.5rem 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 4px;
  background: var(--bg-main);
  color: var(--text-primary);
  font-size: 16px; /* 16px = no iOS auto-zoom */
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
}

.cwd-input:disabled {
  opacity: 0.5;
}

.disconnect-btn {
  background: var(--bg-danger);
  color: white;
}

.disconnect-btn:hover {
  background: #c82333;
}

.sidebar-toggle-collapsed {
  position: fixed;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  padding: 0.625rem 0.5rem;
  border: 1px solid var(--border-color);
  border-left: none;
  border-radius: 0 6px 6px 0;
  background: var(--bg-sidebar);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 1.05rem;
  line-height: 1;
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.18);
  transition: background 0.12s, color 0.12s, padding 0.12s;
}

.sidebar-toggle-collapsed:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
  padding-right: 0.75rem;
}

@media (prefers-color-scheme: dark) {
  .sidebar-toggle-collapsed {
    box-shadow: 2px 0 8px rgba(0, 0, 0, 0.55);
  }
}

/* The main chat region is a solid pane so messages read as a coherent
   surface instead of floating directly on the notebook grid. The grid is
   only visible in the gutter between this pane and the sidebar. */
.main-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-main);
  border: 1px solid var(--border-color);
  border-radius: 8px;
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: transparent;
}

.traffic-panel {
  flex-shrink: 0;
  border-top: 2px solid var(--border-color);
}

.error-banner {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: #fee;
  color: #c00;
  border-bottom: 1px solid #fcc;
}

.error-icon {
  flex-shrink: 0;
}

.error-text {
  flex: 1;
}

.error-close {
  flex-shrink: 0;
  padding: 0.25rem 0.5rem;
  border: none;
  background: transparent;
  color: #c00;
  font-size: 1.25rem;
  line-height: 1;
  cursor: pointer;
  opacity: 0.6;
  border-radius: 4px;
}

.error-close:hover {
  opacity: 1;
  background: rgba(204, 0, 0, 0.1);
}

/* Inline "Reconnect" affordance shown next to a stale error when we have a
   saved session we could reattach to. */
.error-action {
  flex-shrink: 0;
  padding: 0.25rem 0.6rem;
  margin-right: 0.25rem;
  border: 1px solid #c00;
  border-radius: 4px;
  background: transparent;
  color: #c00;
  font-size: 0.8rem;
  font-weight: 500;
  cursor: pointer;
}

.error-action:hover {
  background: rgba(204, 0, 0, 0.1);
}

/* Foreground-reconnect banner. Distinct visual style from the red error
   banner so users immediately read it as transient progress, not failure. */
.reconnect-banner {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.75rem 1rem;
  background: #e0f2fe;
  color: #0369a1;
  border-bottom: 1px solid #bae6fd;
}

.reconnect-text {
  flex: 1;
  font-size: 0.9rem;
}

.reconnect-text strong {
  font-weight: 600;
}

.reconnect-spinner {
  flex-shrink: 0;
  width: 14px;
  height: 14px;
  border: 2px solid currentColor;
  border-right-color: transparent;
  border-radius: 50%;
  animation: reconnect-spin 0.9s linear infinite;
}

@keyframes reconnect-spin {
  to { transform: rotate(360deg); }
}

@media (prefers-color-scheme: dark) {
  .reconnect-banner {
    background: #082f49;
    color: #7dd3fc;
    border-bottom-color: #0c4a6e;
  }
}

.welcome-screen {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 2rem;
  color: var(--text-secondary);
  background:
    radial-gradient(80% 60% at 50% 0%, color-mix(in srgb, var(--text-accent) 8%, transparent) 0%, transparent 70%),
    transparent;
}

.welcome-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
  max-width: 560px;
  gap: 1.25rem;
}

.welcome-brain {
  width: 88px;
  height: 88px;
  color: var(--text-accent);
  opacity: 0.85;
  filter: drop-shadow(0 6px 18px color-mix(in srgb, var(--text-accent) 28%, transparent));
}

.welcome-identity {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  margin-top: -0.25rem;
}

.welcome-name {
  font-size: 0.85rem;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--text-secondary);
}

.welcome-version {
  padding: 0.125rem 0.5rem;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  background: var(--bg-sidebar);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.65rem;
  font-weight: 500;
  color: var(--text-muted);
}

.welcome-headline {
  margin: 0;
  font-size: 1.65rem;
  line-height: 1.2;
  letter-spacing: -0.02em;
  font-weight: 600;
  color: var(--text-primary);
}

.welcome-sub {
  margin: 0;
  font-size: 0.9rem;
  color: var(--text-secondary);
  max-width: 440px;
}

.welcome-actions {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.625rem;
  width: 100%;
  margin-top: 0.5rem;
}

@media (max-width: 800px) {
  .welcome-actions {
    grid-template-columns: 1fr;
  }
  .welcome-brain {
    width: 64px;
    height: 64px;
  }
  .welcome-headline {
    font-size: 1.35rem;
  }
}

.welcome-action {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  padding: 0.75rem 0.875rem;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: var(--bg-main);
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
  transition: border-color 0.15s ease, background 0.15s ease, transform 0.05s ease;
  font: inherit;
}

.welcome-action:hover:not(:disabled) {
  border-color: var(--text-accent);
  background: var(--bg-hover);
}

.welcome-action:active:not(:disabled) {
  transform: translateY(1px);
}

.welcome-action:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.welcome-action.primary {
  border-color: color-mix(in srgb, var(--text-accent) 50%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 10%, var(--bg-main));
}

.welcome-action.primary:hover:not(:disabled) {
  background: color-mix(in srgb, var(--text-accent) 16%, var(--bg-main));
}

.welcome-action-icon {
  font-size: 1.25rem;
  line-height: 1;
  flex-shrink: 0;
}

.welcome-action-label {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  min-width: 0;
}

.welcome-action-label strong {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.welcome-action-label small {
  font-size: 0.7rem;
  font-weight: 400;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ---------- Mobile / narrow-viewport layout ---------- */

.drawer-backdrop {
  display: none;
}

.mobile-hamburger {
  display: none;
}

@media (max-width: 800px) {
  .app-container {
    /* Prevent the off-screen drawer from causing horizontal scroll. */
    overflow-x: hidden;
    /* Drawer mode: the sidebar overlays, so the content uses the full width
       with no gutter and the main pane is edge-to-edge. */
    gap: 0;
    padding: 0;
  }

  .main-area {
    border: none;
    border-radius: 0;
  }

  /* Banners sit at the very top of the main area, where the OS status bar
     / camera notch overlap on phones. Extend the banner colour through the
     safe-area inset and push the text below it so the status bar reads as
     a tinted continuation of the banner instead of clipping its content. */
  .reconnect-banner,
  .error-banner {
    padding-top: calc(0.75rem + env(safe-area-inset-top, 0px));
  }

  .sidebar.is-drawer {
    position: fixed;
    top: var(--tabbar-height);
    left: 0;
    bottom: 0;
    width: 85vw;
    max-width: 360px;
    z-index: 100;
    box-shadow: 2px 0 16px rgba(0, 0, 0, 0.3);
    /* Edge-anchored drawer: drop the floating-pane radius/border. */
    border: none;
    border-radius: 0;
    /* Honour iOS notch / Android status bar. */
    padding-top: env(safe-area-inset-top, 0px);
  }

  .drawer-backdrop {
    display: block;
    position: fixed;
    top: var(--tabbar-height);
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 90;
  }

  /* Hide the desktop chevron when the mobile hamburger is in play. */
  .sidebar-toggle-collapsed {
    display: none;
  }

  .mobile-hamburger {
    display: flex;
    align-items: center;
    justify-content: center;
    position: fixed;
    top: calc(var(--tabbar-height) + env(safe-area-inset-top, 0px) + 0.5rem);
    left: 0.5rem;
    z-index: 50;
    width: 44px;
    height: 44px;
    padding: 0;
    background: var(--bg-sidebar);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    font-size: 1.25rem;
    cursor: pointer;
    color: var(--text-primary);
  }

  /* Tap-target sizing for the icon buttons inside the sidebar header. */
  .header-icon-btn,
  .toggle-btn {
    min-width: 40px;
    min-height: 40px;
    font-size: 1rem;
  }

  /* Honour the iOS home indicator at the bottom of the main area. */
  .main-area {
    padding-bottom: env(safe-area-inset-bottom, 0px);
  }
}
</style>
