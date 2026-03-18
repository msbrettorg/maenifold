<script setup lang="ts">
import { computed, ref } from 'vue';
import { marked } from 'marked';
import type { ThinkingSession } from '../lib/workflow-types';

const props = defineProps<{
  node: ThinkingSession;
}>();

const isExpanded = ref(false);

const thoughtCountLabel = computed(() => {
  const count = props.node.thoughts.length;
  return `${count} thought${count === 1 ? '' : 's'}`;
});

const shortSessionId = computed(() => {
  const id = props.node.sessionId;
  if (id.length <= 18) return id;
  return `${id.slice(0, 8)}…${id.slice(-6)}`;
});

const statusMeta = computed(() => {
  switch (props.node.status) {
    case 'completed':
      return { label: 'Completed', icon: '✅', className: 'status-completed' };
    case 'abandoned':
      return { label: 'Abandoned', icon: '⚠️', className: 'status-abandoned' };
    case 'active':
    default:
      return { label: 'Active', icon: '🔄', className: 'status-active' };
  }
});

function toggleExpanded(): void {
  isExpanded.value = !isExpanded.value;
}

function formatRelativeTime(iso: string): string {
  const timestamp = new Date(iso).getTime();
  if (Number.isNaN(timestamp)) return iso;

  const diff = Date.now() - timestamp;
  if (diff < 0) return 'just now';

  const seconds = Math.floor(diff / 1000);
  if (seconds < 60) return `${seconds}s ago`;

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;

  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;

  return new Date(timestamp).toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
  });
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function renderMarkdown(content: string | undefined): string {
  if (!content) return '';

  const withWikiPills = content.replace(/\[\[([^\]]+)\]\]/g, (_match, label: string) => {
    return `<span class="wiki-pill">${escapeHtml(label)}</span>`;
  });

  const result = marked.parse(withWikiPills, { async: false });
  return typeof result === 'string' ? result : '';
}

function onBeforeEnter(element: Element): void {
  const el = element as HTMLElement;
  el.style.height = '0';
  el.style.opacity = '0';
  el.style.overflow = 'hidden';
}

function onEnter(element: Element, done: () => void): void {
  const el = element as HTMLElement;
  el.style.transition = 'height 220ms ease, opacity 220ms ease';

  requestAnimationFrame(() => {
    el.style.height = `${el.scrollHeight}px`;
    el.style.opacity = '1';
  });

  const handleDone = (event: TransitionEvent) => {
    if (event.propertyName !== 'height') return;
    el.removeEventListener('transitionend', handleDone);
    done();
  };

  el.addEventListener('transitionend', handleDone);
}

function onAfterEnter(element: Element): void {
  const el = element as HTMLElement;
  el.style.height = 'auto';
  el.style.opacity = '1';
  el.style.overflow = 'visible';
  el.style.transition = '';
}

function onBeforeLeave(element: Element): void {
  const el = element as HTMLElement;
  el.style.height = `${el.scrollHeight}px`;
  el.style.opacity = '1';
  el.style.overflow = 'hidden';
}

function onLeave(element: Element, done: () => void): void {
  const el = element as HTMLElement;
  el.style.transition = 'height 220ms ease, opacity 220ms ease';
  void el.offsetHeight;

  requestAnimationFrame(() => {
    el.style.height = '0';
    el.style.opacity = '0';
  });

  const handleDone = (event: TransitionEvent) => {
    if (event.propertyName !== 'height') return;
    el.removeEventListener('transitionend', handleDone);
    done();
  };

  el.addEventListener('transitionend', handleDone);
}

function onAfterLeave(element: Element): void {
  const el = element as HTMLElement;
  el.style.transition = '';
  el.style.height = '';
  el.style.opacity = '';
  el.style.overflow = '';
}
</script>

<template>
  <div class="thinking-session-card">
    <button
      type="button"
      class="card-header"
      :aria-expanded="isExpanded"
      @click="toggleExpanded"
    >
      <div class="header-left">
        <span class="header-icon" aria-hidden="true">🧠</span>
        <div class="header-copy">
          <span class="header-title">Thinking session</span>
          <span class="session-id" :title="node.sessionId">{{ shortSessionId }}</span>
        </div>
      </div>

      <div class="header-right">
        <span class="thought-count-badge">{{ thoughtCountLabel }}</span>
        <span class="status-badge" :class="statusMeta.className" :title="node.status">
          <span aria-hidden="true">{{ statusMeta.icon }}</span>
          {{ statusMeta.label }}
        </span>
        <span class="expand-indicator" aria-hidden="true">{{ isExpanded ? '▲' : '▼' }}</span>
      </div>
    </button>

    <Transition
      @before-enter="onBeforeEnter"
      @enter="onEnter"
      @after-enter="onAfterEnter"
      @before-leave="onBeforeLeave"
      @leave="onLeave"
      @after-leave="onAfterLeave"
    >
      <div v-if="isExpanded" class="card-body">
        <ul v-if="node.thoughts.length" class="thought-list">
          <li
            v-for="thought in node.thoughts"
            :key="`${thought.number}-${thought.timestamp}`"
            class="thought-item"
          >
            <div class="thought-pill">
              {{ thought.number }}/{{ thought.totalThoughts }}
            </div>

            <div class="thought-entry">
              <div class="thought-meta">
                <span v-if="thought.agent" class="agent-badge">{{ thought.agent }}</span>
                <time class="thought-timestamp" :datetime="thought.timestamp" :title="thought.timestamp">
                  {{ formatRelativeTime(thought.timestamp) }}
                </time>
              </div>

              <div
                class="thought-content"
                v-html="renderMarkdown(thought.content)"
              />
            </div>
          </li>
        </ul>

        <div v-else class="empty-state">
          No thoughts recorded for this session.
        </div>

        <section v-if="node.conclusion" class="conclusion-section">
          <div class="conclusion-label">Conclusion</div>
          <div
            class="conclusion-content"
            v-html="renderMarkdown(node.conclusion)"
          />
        </section>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.thinking-session-card {
  width: 280px;
  background: #1a1c24;
  border-radius: 10px;
  border-left: 3px solid #8b5cf6;
  color: #d9def1;
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.28);
  overflow: hidden;
}

.card-header {
  width: 100%;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.875rem 0.95rem;
  background: transparent;
  border: 0;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.card-header:hover {
  background: rgba(139, 92, 246, 0.08);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  min-width: 0;
}

.header-icon {
  width: 1.75rem;
  height: 1.75rem;
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: rgba(139, 92, 246, 0.14);
  flex-shrink: 0;
}

.header-copy {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.header-title {
  font-size: 0.88rem;
  font-weight: 600;
  color: #eef2ff;
}

.session-id {
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 0.69rem;
  color: #8d95b3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.header-right {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 0.4rem;
  min-width: 0;
}

.thought-count-badge,
.status-badge,
.agent-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  border-radius: 999px;
  padding: 0.2rem 0.5rem;
  font-size: 0.68rem;
  line-height: 1.1;
  white-space: nowrap;
}

.thought-count-badge {
  background: #242838;
  color: #c8d0ea;
}

.status-badge {
  border: 1px solid transparent;
  font-weight: 600;
}

.status-active {
  background: rgba(59, 130, 246, 0.14);
  border-color: rgba(59, 130, 246, 0.28);
  color: #93c5fd;
}

.status-completed {
  background: rgba(34, 197, 94, 0.14);
  border-color: rgba(34, 197, 94, 0.28);
  color: #86efac;
}

.status-abandoned {
  background: rgba(245, 158, 11, 0.14);
  border-color: rgba(245, 158, 11, 0.28);
  color: #fcd34d;
}

.expand-indicator {
  color: #8d95b3;
  font-size: 0.58rem;
  padding-top: 0.35rem;
}

.card-body {
  padding: 0 0.95rem 0.95rem;
}

.thought-list {
  list-style: none;
  margin: 0;
  padding: 0 0 0 0.35rem;
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
}

.thought-list::before {
  content: '';
  position: absolute;
  left: 0.95rem;
  top: 0.4rem;
  bottom: 0.4rem;
  width: 2px;
  background: linear-gradient(180deg, rgba(139, 92, 246, 0.9), rgba(139, 92, 246, 0.22));
}

.thought-item {
  position: relative;
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.65rem;
  align-items: start;
}

.thought-pill {
  position: relative;
  z-index: 1;
  min-width: 2.5rem;
  padding: 0.26rem 0.45rem;
  border-radius: 999px;
  background: #2d2345;
  border: 1px solid rgba(167, 139, 250, 0.28);
  color: #ddd6fe;
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 0.66rem;
  text-align: center;
  box-shadow: 0 0 0 4px #1a1c24;
}

.thought-entry {
  background: #20232d;
  border: 1px solid #2b3040;
  border-radius: 8px;
  padding: 0.65rem 0.75rem;
  min-width: 0;
}

.thought-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  margin-bottom: 0.45rem;
}

.agent-badge {
  background: #262d3d;
  color: #b9c3e4;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.thought-timestamp {
  font-size: 0.68rem;
  color: #8d95b3;
  white-space: nowrap;
}

.thought-content,
.conclusion-content {
  font-size: 0.76rem;
  line-height: 1.5;
  color: #d9def1;
  overflow: hidden;
  overflow-wrap: anywhere;
}

.thought-content {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
}

.thought-content :deep(p),
.conclusion-content :deep(p) {
  margin: 0;
}

.thought-content :deep(p + p),
.conclusion-content :deep(p + p) {
  margin-top: 0.35rem;
}

.thought-content :deep(ul),
.thought-content :deep(ol),
.conclusion-content :deep(ul),
.conclusion-content :deep(ol) {
  margin: 0.2rem 0 0;
  padding-left: 1rem;
}

.thought-content :deep(code),
.conclusion-content :deep(code) {
  background: #111827;
  color: #c4b5fd;
  border-radius: 4px;
  padding: 0.1rem 0.3rem;
  font-size: 0.72rem;
  font-family: 'SF Mono', 'Fira Code', monospace;
}

.thought-content :deep(pre),
.conclusion-content :deep(pre) {
  background: #111827;
  border-radius: 6px;
  padding: 0.55rem;
  overflow-x: auto;
}

.thought-content :deep(pre code),
.conclusion-content :deep(pre code) {
  background: transparent;
  padding: 0;
}

.thought-content :deep(.wiki-pill),
.conclusion-content :deep(.wiki-pill) {
  display: inline-flex;
  align-items: center;
  padding: 0.08rem 0.42rem;
  margin: 0 0.08rem;
  border-radius: 999px;
  background: #2d3748;
  color: #90cdf4;
  font-size: 0.7rem;
  font-weight: 600;
  white-space: nowrap;
}

.conclusion-section {
  margin-top: 0.95rem;
  padding: 0.75rem;
  border: 1px solid rgba(139, 92, 246, 0.18);
  border-radius: 8px;
  background: rgba(139, 92, 246, 0.05);
}

.conclusion-label {
  margin-bottom: 0.45rem;
  font-size: 0.68rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: #c4b5fd;
}

.empty-state {
  padding: 0.75rem 0;
  font-size: 0.76rem;
  color: #8d95b3;
}
</style>
