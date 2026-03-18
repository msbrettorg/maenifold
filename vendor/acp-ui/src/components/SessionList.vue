<script setup lang="ts">
import { computed } from 'vue';
import { useSessionStore } from '../stores/session';
import type { SavedSession } from '../lib/types';

const emit = defineEmits<{
  resume: [session: SavedSession];
  delete: [sessionId: string];
}>();

const sessionStore = useSessionStore();

// Only show sessions that can be resumed (agent supports loadSession)
const sessions = computed(() => 
  [...sessionStore.resumableSessions].sort((a, b) => b.lastUpdated - a.lastUpdated)
);

const liveSessionsByRemoteId = computed(() => {
  const liveSessions = new Map<
    string,
    {
      localId: string;
      isConnected: boolean;
      isConnecting: boolean;
    }
  >();

  for (const [localId, liveSession] of sessionStore.sessions.entries()) {
    const remoteSessionId = liveSession.session?.sessionId;
    if (!remoteSessionId) {
      continue;
    }

    liveSessions.set(remoteSessionId, {
      localId,
      isConnected: liveSession.isConnected,
      isConnecting: liveSession.isConnecting,
    });
  }

  return liveSessions;
});

function formatDate(timestamp: number): string {
  return new Date(timestamp).toLocaleString();
}

function handleResume(session: SavedSession) {
  const liveSession = liveSessionsByRemoteId.value.get(session.sessionId);

  if (liveSession) {
    sessionStore.setActiveSession(liveSession.localId);
    return;
  }

  emit('resume', session);
}

function isAgentSession(session: SavedSession): boolean {
  return session.id.startsWith('agent:');
}

function isCurrentConnectedSession(session: SavedSession): boolean {
  return liveSessionsByRemoteId.value.get(session.sessionId)?.isConnected ?? false;
}

function isCurrentConnectingSession(session: SavedSession): boolean {
  return liveSessionsByRemoteId.value.get(session.sessionId)?.isConnecting ?? false;
}

function getDeleteTitle(session: SavedSession): string {
  if (isCurrentConnectedSession(session)) {
    return 'Disconnect';
  }

  if (isCurrentConnectingSession(session)) {
    return 'Cancel';
  }

  return 'Delete session';
}

async function handleDelete(session: SavedSession, event: Event) {
  event.stopPropagation();

  const liveSession = liveSessionsByRemoteId.value.get(session.sessionId);

  if (liveSession?.isConnected) {
    await sessionStore.closeSession(liveSession.localId);
    return;
  }

  if (liveSession?.isConnecting) {
    if (sessionStore.activeSessionId !== liveSession.localId) {
      sessionStore.setActiveSession(liveSession.localId);
    }

    await sessionStore.cancelConnection();
    return;
  }

  if (confirm('Delete this session?')) {
    emit('delete', session.id);
  }
}
</script>

<template>
  <div class="session-list">
    <h3>Saved Sessions</h3>
    
    <div v-if="sessions.length === 0" class="empty-state">
      <p>No saved sessions yet.</p>
      <p class="hint">Create a new session to get started.</p>
    </div>
    
    <ul v-else>
      <li 
        v-for="session in sessions" 
        :key="session.id"
        class="session-item"
        @click="handleResume(session)"
      >
        <div class="session-info">
          <div class="session-title-row">
            <span class="session-title">{{ session.title }}</span>
            <span
              v-if="isCurrentConnectedSession(session)"
              class="connection-badge connected"
            >
              <span class="connection-dot">●</span> connected
            </span>
            <span
              v-else-if="isCurrentConnectingSession(session)"
              class="connection-badge connecting"
            >
              <span class="connection-dot">●</span> connecting
            </span>
          </div>
          <span class="session-agent">
            {{ session.agentName }}
            <span v-if="isAgentSession(session)" class="session-badge">from agent</span>
          </span>
          <span class="session-date">{{ formatDate(session.lastUpdated) }}</span>
        </div>
        <button 
          class="delete-btn"
          @click="(e) => handleDelete(session, e)"
          :title="getDeleteTitle(session)"
        >
          ×
        </button>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.session-list {
  padding: 1rem;
}

h3 {
  margin: 0 0 1rem 0;
  font-size: 1rem;
  color: var(--text-secondary, #666);
}

.empty-state {
  text-align: center;
  padding: 2rem;
  color: var(--text-muted, #999);
}

.empty-state .hint {
  font-size: 0.875rem;
  margin-top: 0.5rem;
}

ul {
  list-style: none;
  padding: 0;
  margin: 0;
}

.session-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem;
  border: 1px solid var(--border-color, #e0e0e0);
  border-radius: 6px;
  margin-bottom: 0.5rem;
  cursor: pointer;
  transition: background 0.15s;
}

.session-item:hover {
  background: var(--bg-hover, #f5f5f5);
}

.session-info {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  overflow: hidden;
}

.session-title {
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.session-title-row {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  min-width: 0;
}

.connection-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.0625rem 0.375rem;
  border-radius: 999px;
  font-size: 0.6875rem;
  font-weight: 500;
  white-space: nowrap;
}

.connection-badge.connected {
  background: rgba(40, 167, 69, 0.12);
  color: var(--bg-success, #28a745);
}

.connection-badge.connecting {
  background: rgba(209, 139, 0, 0.12);
  color: #d18b00;
}

.connection-dot {
  font-size: 0.5rem;
  line-height: 1;
}

.session-agent {
  font-size: 0.75rem;
  color: var(--text-accent, #0066cc);
}

.session-badge {
  display: inline-block;
  margin-left: 0.375rem;
  padding: 0.0625rem 0.375rem;
  border-radius: 999px;
  background: var(--bg-hover, #f5f5f5);
  color: var(--text-muted, #999);
  font-size: 0.6875rem;
  font-weight: 500;
  text-transform: lowercase;
}

.session-date {
  font-size: 0.75rem;
  color: var(--text-muted, #999);
}

.delete-btn {
  padding: 0.25rem 0.5rem;
  border: none;
  background: transparent;
  color: var(--text-muted, #999);
  font-size: 1.25rem;
  cursor: pointer;
  border-radius: 4px;
}

.delete-btn:hover {
  background: var(--bg-danger, #fee);
  color: var(--text-danger, #c00);
}
</style>
