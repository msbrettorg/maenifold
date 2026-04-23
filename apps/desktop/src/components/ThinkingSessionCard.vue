<script setup lang="ts">
import { computed, ref } from 'vue';
import type { Component } from 'vue';
import { RefreshCw, AlertTriangle, Brain, CheckCircle } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import type { ThinkingSession } from '../lib/workflow-types';
import ThinkingMarkdownContent from './ThinkingMarkdownContent.vue';
import ThinkingThoughtItem from './ThinkingThoughtItem.vue';

const props = defineProps<{
  node: ThinkingSession;
}>();

const isExpanded = ref(false);
const thoughtCountLabel = computed(() => `${props.node.thoughts.length} thought${props.node.thoughts.length === 1 ? '' : 's'}`);
const shortSessionId = computed(() => props.node.sessionId.length <= 18
  ? props.node.sessionId
  : `${props.node.sessionId.slice(0, 8)}…${props.node.sessionId.slice(-6)}`);
const statusMetaMap: Record<ThinkingSession['status'], { label: string; icon: Component; className: string }> = {
  active: { label: 'Active', icon: RefreshCw, className: 'status-active' },
  completed: { label: 'Completed', icon: CheckCircle, className: 'status-completed' },
  abandoned: { label: 'Abandoned', icon: AlertTriangle, className: 'status-abandoned' },
} as const;
const statusMeta = computed(() => statusMetaMap[props.node.status]);

function toggleExpanded(): void {
  isExpanded.value = !isExpanded.value;
}

function onBeforeEnter(element: Element): void {
  const el = element as HTMLElement;
  el.style.height = '0'; el.style.opacity = '0'; el.style.overflow = 'hidden';
}

function onEnter(element: Element, done: () => void): void {
  const el = element as HTMLElement;
  el.style.transition = 'height 220ms ease, opacity 220ms ease';
  requestAnimationFrame(() => {
    el.style.height = `${el.scrollHeight}px`; el.style.opacity = '1';
  });
  const handleDone = (event: TransitionEvent) => {
    if (event.propertyName !== 'height') return;
    el.removeEventListener('transitionend', handleDone); done();
  };
  el.addEventListener('transitionend', handleDone);
}

function onAfterEnter(element: Element): void {
  const el = element as HTMLElement;
  el.style.height = 'auto'; el.style.opacity = '1'; el.style.overflow = 'visible'; el.style.transition = '';
}

function onBeforeLeave(element: Element): void {
  const el = element as HTMLElement;
  el.style.height = `${el.scrollHeight}px`; el.style.opacity = '1'; el.style.overflow = 'hidden';
}

function onLeave(element: Element, done: () => void): void {
  const el = element as HTMLElement;
  el.style.transition = 'height 220ms ease, opacity 220ms ease';
  void el.offsetHeight;
  requestAnimationFrame(() => {
    el.style.height = '0'; el.style.opacity = '0';
  });
  const handleDone = (event: TransitionEvent) => {
    if (event.propertyName !== 'height') return;
    el.removeEventListener('transitionend', handleDone); done();
  };
  el.addEventListener('transitionend', handleDone);
}

function onAfterLeave(element: Element): void {
  const el = element as HTMLElement;
  el.style.transition = ''; el.style.height = ''; el.style.opacity = ''; el.style.overflow = '';
}
</script>

<template>
  <div class="thinking-session-card">
    <Button variant="ghost" class="card-header" :aria-expanded="isExpanded" @click="toggleExpanded">
      <div class="header-left">
        <span class="header-icon" aria-hidden="true"><Brain class="size-4" :stroke-width="1.5" /></span>
        <div class="header-copy">
          <span class="header-title">Thinking session</span>
          <span class="session-id" :title="node.sessionId">{{ shortSessionId }}</span>
        </div>
      </div>

      <div class="header-right">
        <span class="thought-count-badge">{{ thoughtCountLabel }}</span>
        <span class="status-badge" :class="statusMeta.className" :title="node.status">
          <span aria-hidden="true"><component :is="statusMeta.icon" class="size-4" :stroke-width="1.5" /></span>
          {{ statusMeta.label }}
        </span>
        <span class="expand-indicator" aria-hidden="true">{{ isExpanded ? '▲' : '▼' }}</span>
      </div>
    </Button>
    <Transition @before-enter="onBeforeEnter" @enter="onEnter" @after-enter="onAfterEnter" @before-leave="onBeforeLeave" @leave="onLeave" @after-leave="onAfterLeave">
      <div v-if="isExpanded" class="card-body">
        <ul v-if="node.thoughts.length" class="thought-list">
          <ThinkingThoughtItem v-for="thought in node.thoughts" :key="`${thought.number}-${thought.timestamp}`" :thought="thought" />
        </ul>
        <div v-else class="empty-state">No thoughts recorded for this session.</div>
        <section v-if="node.conclusion" class="conclusion-section">
          <div class="conclusion-label">Conclusion</div>
          <ThinkingMarkdownContent :content="node.conclusion" />
        </section>
      </div>
    </Transition>
  </div>
</template>

<!-- audit-ok: max-style-rules -->
<style scoped>
.thinking-session-card {
  --thinking-card-text: color-mix(in srgb, var(--color-text-primary) 84%, var(--color-node-prompt));
  --thinking-card-heading: color-mix(in srgb, var(--color-text-primary) 94%, var(--color-node-prompt));
  --thinking-card-muted: color-mix(in srgb, var(--color-text-primary) 56%, var(--color-text-secondary));
  --thinking-card-badge: color-mix(in srgb, var(--color-text-primary) 72%, var(--color-node-prompt));
  width: 280px;
  background: color-mix(in srgb, var(--color-bg-surface) 92%, var(--color-bg-app));
  border-radius: var(--radius);
  border-left: 3px solid var(--color-node-prompt);
  color: var(--thinking-card-text);
  box-shadow: var(--shadow-16);
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
.card-header:hover { background: color-mix(in srgb, var(--color-node-prompt) 8%, var(--color-bg-surface)); }
.header-left {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  min-width: 0;
}
.header-icon {
  width: 1.75rem;
  height: 1.75rem;
  border-radius: 4px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--color-node-prompt) 14%, var(--color-bg-surface));
  flex-shrink: 0;
}
.header-copy {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.header-title {
  font-size: var(--text-body-sm);
  font-weight: var(--weight-title);
  color: var(--thinking-card-heading);
}
.session-id {
  font-family: var(--font-code);
  font-size: var(--text-tiny);
  color: var(--thinking-card-muted);
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
.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  border-radius: 4px;
  padding: 0.2rem 0.5rem;
  font-size: var(--text-tiny);
  line-height: 1.1;
  white-space: nowrap;
}
.thought-count-badge {
  background: color-mix(in srgb, var(--color-bg-surface) 76%, var(--color-bg-app));
  color: color-mix(in srgb, var(--color-text-primary) 74%, var(--color-node-prompt));
}
.status-badge {
  border: 1px solid transparent;
  font-weight: var(--weight-title);
}
.status-active {
  background: color-mix(in srgb, var(--color-status-active) 14%, var(--color-bg-surface));
  border-color: color-mix(in srgb, var(--color-status-active) 28%, transparent);
  color: color-mix(in srgb, var(--color-text-primary) 74%, var(--color-status-active));
}
.status-completed {
  background: color-mix(in srgb, var(--color-status-completed) 14%, var(--color-bg-surface));
  border-color: color-mix(in srgb, var(--color-status-completed) 28%, transparent);
  color: color-mix(in srgb, var(--color-text-primary) 72%, var(--color-status-completed));
}
.status-abandoned {
  background: color-mix(in srgb, var(--color-status-stuck) 14%, var(--color-bg-surface));
  border-color: color-mix(in srgb, var(--color-status-stuck) 28%, transparent);
  color: color-mix(in srgb, var(--color-text-primary) 68%, var(--color-status-stuck));
}
.expand-indicator {
  color: var(--thinking-card-muted);
  font-size: var(--text-tiny);
  padding-top: 0.35rem;
}
.card-body { padding: 0 0.95rem 0.95rem; }
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
  background: linear-gradient(
    180deg,
    color-mix(in srgb, var(--color-node-prompt) 90%, transparent),
    color-mix(in srgb, var(--color-node-prompt) 22%, transparent)
  );
}
.conclusion-section {
  margin-top: 0.95rem;
  padding: 0.75rem;
  border: 1px solid color-mix(in srgb, var(--color-node-prompt) 18%, transparent);
  border-radius: var(--radius);
  background: color-mix(in srgb, var(--color-node-prompt) 6%, var(--color-bg-surface));
}
.conclusion-label {
  margin-bottom: 0.45rem;
  font-size: var(--text-tiny);
  font-weight: var(--weight-title);
  letter-spacing: 0.06em;
  color: var(--thinking-card-badge);
}
.empty-state {
  padding: 0.75rem 0;
  font-size: var(--text-caption);
  color: var(--thinking-card-muted);
}
</style>
