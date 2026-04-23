<script setup lang="ts">
import type { Thought } from '../lib/workflow-types';
import KnowledgeNoteRenderer from './KnowledgeNoteRenderer.vue';

defineProps<{
  thought: Thought;
}>();

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
</script>

<template>
  <li>
    <article class="thought-item" role="article" :aria-label="`Thought ${thought.number} of ${thought.totalThoughts}`">
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

        <KnowledgeNoteRenderer :content="thought.content" clamp />
      </div>
    </article>
  </li>
</template>

<style scoped>
.thought-item {
  --thinking-thought-badge: color-mix(in srgb, var(--color-text-primary) 76%, var(--color-node-prompt));
  --thinking-thought-muted: color-mix(in srgb, var(--color-text-primary) 56%, var(--color-text-secondary));
  --thinking-thought-border: color-mix(in srgb, var(--color-border-subtle) 28%, var(--color-bg-surface));
  --thinking-thought-shadow: color-mix(in srgb, var(--color-bg-surface) 76%, var(--color-bg-app));
  --thinking-thought-text: color-mix(in srgb, var(--color-text-primary) 72%, var(--color-node-prompt));
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
  border-radius: 4px;
  background: color-mix(in srgb, var(--color-node-prompt) 18%, var(--color-bg-surface));
  border: 1px solid color-mix(in srgb, var(--color-node-prompt) 28%, transparent);
  color: var(--thinking-thought-badge);
  font-family: var(--font-code);
  font-size: var(--text-tiny);
  text-align: center;
  box-shadow: 0 0 0 4px var(--thinking-thought-shadow);
}

.thought-entry {
  background: color-mix(in srgb, var(--color-bg-surface) 84%, var(--color-bg-app));
  border: 1px solid var(--thinking-thought-border);
  border-radius: var(--radius);
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
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  border-radius: 4px;
  padding: 0.2rem 0.5rem;
  font-size: var(--text-tiny);
  line-height: 1.1;
  white-space: nowrap;
  background: color-mix(in srgb, var(--color-bg-surface) 72%, var(--color-bg-app));
  color: var(--thinking-thought-text);
  letter-spacing: 0.04em;
}

.thought-timestamp {
  font-size: var(--text-tiny);
  color: var(--thinking-thought-muted);
  white-space: nowrap;
}
</style>
