<script setup lang="ts">
// Home landing tab. Modeled on the pre-rip UI's HomeDashboard: brain glyph,
// app identity + version pill, "What can I help you with?" headline, a live
// connection-status pill, a quick-tiles launcher grid, and a recent-activity
// list built from saved sessions. Pure presentation over the existing stores —
// no new backend wiring. Tab navigation goes through the appShell store; the
// session lifecycle (start / resume) is delegated to App.vue via emits so the
// shell can flip to the Agents tab once a session is live.
import { computed } from 'vue';
import { Brain, MessageSquare, MessageCircle, Clock } from '@lucide/vue';
import { useConfigStore } from '../stores/config';
import { useSessionStore } from '../stores/session';
import { useAppShellStore } from '../stores/appShell';
import type { SavedSession } from '../lib/types';

const props = defineProps<{ version: string }>();

const emit = defineEmits<{
  'new-session': [];
  'resume-session': [session: SavedSession];
}>();

const configStore = useConfigStore();
const sessionStore = useSessionStore();
const shell = useAppShellStore();

const hasAgents = computed(() => configStore.hasAgents);
const selectedAgentName = computed(() => configStore.defaultAgentName);

type Tone = 'connected' | 'connecting' | 'idle';
interface ConnStatus {
  label: string;
  description: string;
  tone: Tone;
}

const connectionStatus = computed<ConnStatus>(() => {
  if (sessionStore.isReconnecting) {
    return { label: 'Reconnecting', description: 'Reattaching to your last session…', tone: 'connecting' };
  }
  if (sessionStore.isConnecting) {
    return { label: 'Connecting', description: 'Starting an agent session…', tone: 'connecting' };
  }
  if (sessionStore.isConnected) {
    return { label: 'Connected', description: 'A session is live in the Agents tab.', tone: 'connected' };
  }
  return {
    label: 'Idle',
    description: hasAgents.value ? 'Ready when you are.' : 'Configure agents in ~/maenifold/maenifold.json.',
    tone: 'idle',
  };
});

interface QuickTile {
  key: 'agents';
  title: string;
  description: string;
  icon: typeof MessageSquare;
}

// QuickTile is preserved for when real, MCP-backed surfaces (Gates,
// Pipelines, Workflows) get views wired in. Right now there's nothing
// honest to put in the grid — the only real tile (Agents) duplicates
// the primary CTA above — so the grid renders an empty array and is
// hidden via v-if in the template.
const tiles: QuickTile[] = [];

const recentSessions = computed<SavedSession[]>(() =>
  [...sessionStore.resumableSessions].sort((a, b) => b.lastUpdated - a.lastUpdated).slice(0, 6)
);

const rtf = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });
function relativeTime(ts: number): string {
  if (!ts) return 'unknown';
  const diffSeconds = Math.round((ts - Date.now()) / 1000);
  const abs = Math.abs(diffSeconds);
  if (abs < 60) return rtf.format(diffSeconds, 'second');
  const m = Math.round(diffSeconds / 60);
  if (Math.abs(m) < 60) return rtf.format(m, 'minute');
  const h = Math.round(m / 60);
  if (Math.abs(h) < 24) return rtf.format(h, 'hour');
  const d = Math.round(h / 24);
  if (Math.abs(d) < 30) return rtf.format(d, 'day');
  const mo = Math.round(d / 30);
  if (Math.abs(mo) < 12) return rtf.format(mo, 'month');
  return rtf.format(Math.round(mo / 12), 'year');
}

function handleStartSession() {
  // App.vue validates prerequisites (agent + cwd) and switches to the Agents
  // tab. We always route there so any validation error is visible.
  shell.setActiveTab('agents');
  emit('new-session');
}

function handleResume(session: SavedSession) {
  emit('resume-session', session);
}
</script>

<template>
  <div class="home">
    <div class="home-inner">
      <!-- Identity -->
      <Brain class="home-brain" :size="80" :stroke-width="1" />
      <div class="home-identity">
        <span class="home-name">Maenifold</span>
        <span class="home-version">v{{ props.version }}</span>
      </div>
      <h1 class="home-headline">What can I help you with?</h1>

      <!-- Connection status pill -->
      <div class="home-status" :class="`tone-${connectionStatus.tone}`" role="status">
        <span class="home-status-dot" />
        <span class="home-status-label">{{ connectionStatus.label }}</span>
        <span class="home-status-desc">{{ connectionStatus.description }}</span>
      </div>

      <!-- Primary CTA -->
      <button
        class="home-cta"
        :disabled="!hasAgents || sessionStore.isConnecting"
        @click="handleStartSession"
      >
        <MessageCircle :size="18" :stroke-width="2" />
        <span>
          <strong>Start a session</strong>
          <small>{{ selectedAgentName ? `with ${selectedAgentName}` : 'Open a new conversation' }}</small>
        </span>
      </button>

      <!-- Quick tiles: rendered only when there are real, MCP-backed tabs
           to surface. See `tiles` in the script block. -->
      <div v-if="tiles.length > 0" class="home-tiles">
        <button
          v-for="tile in tiles"
          :key="tile.key"
          class="home-tile"
          @click="shell.setActiveTab(tile.key)"
        >
          <component :is="tile.icon" class="home-tile-icon" :size="22" :stroke-width="1.75" />
          <span class="home-tile-title">{{ tile.title }}</span>
          <span class="home-tile-desc">{{ tile.description }}</span>
        </button>
      </div>

      <!-- Recent activity -->
      <section class="home-recent">
        <h2 class="home-recent-title">
          <Clock :size="15" :stroke-width="1.75" />
          Recent sessions
        </h2>
        <p v-if="recentSessions.length === 0" class="home-recent-empty">
          Nothing here yet — start a session to get going.
        </p>
        <ul v-else class="home-recent-list">
          <li v-for="session in recentSessions" :key="session.id">
            <button class="home-recent-item" @click="handleResume(session)">
              <MessageSquare class="home-recent-item-icon" :size="16" :stroke-width="1.75" />
              <span class="home-recent-item-body">
                <span class="home-recent-item-title">{{ session.title || session.sessionId }}</span>
                <span class="home-recent-item-meta">{{ session.agentName }} · {{ relativeTime(session.lastUpdated) }}</span>
              </span>
            </button>
          </li>
        </ul>
      </section>
    </div>
  </div>
</template>

<style scoped>
.home {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  justify-content: center;
  padding: 3.5rem 2rem 3rem;
  background:
    radial-gradient(70% 50% at 50% 0%, color-mix(in srgb, var(--text-accent) 9%, transparent) 0%, transparent 70%),
    var(--bg-main);
}

.home-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
  max-width: 720px;
  gap: 1rem;
}

.home-brain {
  color: var(--text-accent);
  opacity: 0.9;
  filter: drop-shadow(0 8px 22px color-mix(in srgb, var(--text-accent) 28%, transparent));
}

.home-identity {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
}

.home-name {
  font-size: 0.85rem;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--text-secondary);
}

.home-version {
  padding: 0.125rem 0.5rem;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  background: var(--bg-sidebar);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.65rem;
  font-weight: 500;
  color: var(--text-muted);
}

.home-headline {
  margin: 0.25rem 0 0.5rem;
  font-size: 1.7rem;
  line-height: 1.2;
  letter-spacing: -0.02em;
  font-weight: 600;
  color: var(--text-primary);
  text-align: center;
}

/* ---- Connection status pill ---- */
.home-status {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.35rem 0.85rem;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--bg-sidebar);
  font-size: 0.78rem;
}

.home-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-muted);
  flex-shrink: 0;
}

.home-status-label {
  font-weight: 600;
  color: var(--text-primary);
}

.home-status-desc {
  color: var(--text-muted);
}

.home-status.tone-connected {
  border-color: color-mix(in srgb, var(--bg-success) 45%, var(--border-color));
  background: color-mix(in srgb, var(--bg-success) 12%, transparent);
}
.home-status.tone-connected .home-status-dot { background: var(--bg-success); }

.home-status.tone-connecting {
  border-color: color-mix(in srgb, var(--text-accent) 45%, var(--border-color));
  background: color-mix(in srgb, var(--text-accent) 12%, transparent);
}
.home-status.tone-connecting .home-status-dot {
  background: var(--text-accent);
  animation: home-pulse 1.1s ease-in-out infinite;
}

@keyframes home-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.35; }
}

/* ---- Primary CTA ---- */
.home-cta {
  display: inline-flex;
  align-items: center;
  gap: 0.6rem;
  margin-top: 0.5rem;
  padding: 0.7rem 1.25rem;
  border: 1px solid color-mix(in srgb, var(--text-accent) 55%, var(--border-color));
  border-radius: 10px;
  background: color-mix(in srgb, var(--text-accent) 14%, var(--bg-main));
  color: var(--text-primary);
  cursor: pointer;
  font: inherit;
  transition: background 0.15s ease, transform 0.05s ease, border-color 0.15s ease;
}

.home-cta:hover:not(:disabled) {
  background: color-mix(in srgb, var(--text-accent) 20%, var(--bg-main));
}
.home-cta:active:not(:disabled) { transform: translateY(1px); }
.home-cta:disabled { opacity: 0.5; cursor: not-allowed; }

.home-cta span {
  display: flex;
  flex-direction: column;
  text-align: left;
  line-height: 1.2;
}
.home-cta strong { font-size: 0.9rem; font-weight: 600; }
.home-cta small { font-size: 0.72rem; color: var(--text-muted); }

/* ---- Quick tiles ---- */
.home-tiles {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.7rem;
  width: 100%;
  margin-top: 1rem;
}

.home-tile {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.4rem;
  padding: 1rem 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: var(--bg-main);
  color: var(--text-primary);
  cursor: pointer;
  text-align: center;
  transition: border-color 0.15s ease, background 0.15s ease, transform 0.08s ease;
}

.home-tile:hover {
  border-color: var(--text-accent);
  background: var(--bg-hover);
  transform: translateY(-2px);
}

.home-tile-icon { color: var(--text-accent); }
.home-tile-title { font-size: 0.82rem; font-weight: 600; color: var(--text-primary); }
.home-tile-desc { font-size: 0.68rem; color: var(--text-muted); line-height: 1.3; }

/* ---- Recent activity ---- */
.home-recent {
  width: 100%;
  margin-top: 1.5rem;
  border: 1px solid var(--border-color);
  border-radius: 14px;
  background: var(--bg-main);
  padding: 0.85rem 0.85rem 0.65rem;
}

.home-recent-title {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  margin: 0 0 0.5rem;
  padding: 0 0.4rem;
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-primary);
}
.home-recent-title svg { color: var(--text-muted); }

.home-recent-empty {
  margin: 0;
  padding: 1rem 0.5rem;
  font-size: 0.85rem;
  color: var(--text-secondary);
}

.home-recent-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.home-recent-item {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  width: 100%;
  padding: 0.55rem 0.5rem;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
  font: inherit;
  transition: background 0.12s ease;
}
.home-recent-item:hover { background: var(--bg-hover); }

.home-recent-item-icon { color: var(--text-accent); flex-shrink: 0; }

.home-recent-item-body { display: flex; flex-direction: column; min-width: 0; }

.home-recent-item-title {
  font-size: 0.85rem;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.home-recent-item-meta {
  font-size: 0.72rem;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

@media (max-width: 720px) {
  .home-tiles { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .home-headline { font-size: 1.4rem; }
  .home-brain { width: 64px; height: 64px; }
}
</style>
