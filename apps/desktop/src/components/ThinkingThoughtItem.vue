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
    <!-- audit-ok: dynamic-binding (color-mix locals consumed by descendants) -->
    <article
      class="thought-item relative grid grid-cols-[auto_1fr] gap-[0.65rem] items-start"
      :style="{
        '--thinking-thought-badge': 'color-mix(in srgb, var(--color-text-primary) 76%, var(--color-node-prompt))',
        '--thinking-thought-muted': 'color-mix(in srgb, var(--color-text-primary) 56%, var(--color-text-secondary))',
        '--thinking-thought-border': 'color-mix(in srgb, var(--color-border-subtle) 28%, var(--color-bg-surface))',
        '--thinking-thought-shadow': 'color-mix(in srgb, var(--color-bg-surface) 76%, var(--color-bg-app))',
        '--thinking-thought-text': 'color-mix(in srgb, var(--color-text-primary) 72%, var(--color-node-prompt))',
      }"
      role="article"
      :aria-label="`Thought ${thought.number} of ${thought.totalThoughts}`"
    >
      <div
        class="thought-pill relative z-[1] min-w-[2.5rem] px-[0.45rem] py-[0.26rem] rounded-[4px] bg-[color-mix(in_srgb,var(--color-node-prompt)_18%,var(--color-bg-surface))] border border-[color-mix(in_srgb,var(--color-node-prompt)_28%,transparent)] text-[var(--thinking-thought-badge)] font-[var(--font-code)] text-[length:var(--text-tiny)] text-center shadow-[0_0_0_4px_var(--thinking-thought-shadow)]"
      >
        {{ thought.number }}/{{ thought.totalThoughts }}
      </div>

      <div
        class="thought-entry bg-[color-mix(in_srgb,var(--color-bg-surface)_84%,var(--color-bg-app))] border border-[var(--thinking-thought-border)] rounded-[var(--radius)] px-[0.75rem] py-[0.65rem] min-w-0"
      >
        <div class="thought-meta flex items-center justify-between gap-[0.5rem] mb-[0.45rem]">
          <span
            v-if="thought.agent"
            class="agent-badge inline-flex items-center gap-[0.3rem] rounded-[4px] px-[0.5rem] py-[0.2rem] text-[length:var(--text-tiny)] leading-[1.1] whitespace-nowrap bg-[color-mix(in_srgb,var(--color-bg-surface)_72%,var(--color-bg-app))] text-[var(--thinking-thought-text)] tracking-[0.04em]"
          >{{ thought.agent }}</span>
          <time
            class="thought-timestamp text-[length:var(--text-tiny)] text-[var(--thinking-thought-muted)] whitespace-nowrap"
            :datetime="thought.timestamp"
            :title="thought.timestamp"
          >
            {{ formatRelativeTime(thought.timestamp) }}
          </time>
        </div>

        <KnowledgeNoteRenderer :content="thought.content" clamp />
      </div>
    </article>
  </li>
</template>
