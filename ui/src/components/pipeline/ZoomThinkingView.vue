<script setup lang="ts">
// SemanticZoom content renderer: the sequential-thinking trace as an accordion
// of thoughts. Each entry is collapsible; revisions are flagged with the thought
// they revise.
import { ref } from 'vue';
import { ChevronRight, GitBranch, RotateCcw } from '@lucide/vue';
import type { ThinkingTraceEntry } from '../../lib/pipeline-types';

const props = defineProps<{ trace: ThinkingTraceEntry[] }>();

// Open the first thought by default; track open state per index.
const open = ref<Set<number>>(new Set(props.trace.length > 0 ? [0] : []));

function toggle(index: number): void {
  const next = new Set(open.value);
  if (next.has(index)) next.delete(index);
  else next.add(index);
  open.value = next;
}

function isOpen(index: number): boolean {
  return open.value.has(index);
}

function summary(text: string): string {
  const oneLine = text.replace(/\s+/g, ' ').trim();
  return oneLine.length > 80 ? `${oneLine.slice(0, 80)}…` : oneLine;
}
</script>

<template>
  <div class="zthink">
    <article
      v-for="(entry, index) in props.trace"
      :key="index"
      class="zthink-item"
      :class="{ open: isOpen(index), revision: entry.isRevision }"
    >
      <button type="button" class="zthink-head" @click="toggle(index)">
        <ChevronRight class="zthink-chevron" :size="14" :stroke-width="2.2" />
        <span class="zthink-num">
          {{ entry.thoughtNumber }}<span v-if="entry.totalThoughts" class="zthink-total">/{{ entry.totalThoughts }}</span>
        </span>
        <span class="zthink-summary">{{ summary(entry.text) }}</span>
        <span v-if="entry.isRevision" class="zthink-tag revision-tag">
          <RotateCcw :size="11" :stroke-width="2" />
          <template v-if="entry.revisesThought">revises #{{ entry.revisesThought }}</template>
          <template v-else>revision</template>
        </span>
        <span v-if="entry.branchId" class="zthink-tag branch-tag">
          <GitBranch :size="11" :stroke-width="2" />{{ entry.branchId }}
        </span>
      </button>
      <div v-if="isOpen(index)" class="zthink-body">
        <pre class="zthink-text">{{ entry.text }}</pre>
        <p v-if="entry.agent" class="zthink-agent">agent: {{ entry.agent }}</p>
      </div>
    </article>
  </div>
</template>

<style scoped>
.zthink {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.zthink-item {
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-main) 94%, var(--bg-sidebar));
  overflow: hidden;
}

.zthink-item.revision {
  border-color: color-mix(in srgb, var(--node-accent-thinking) 45%, var(--border-color));
}

.zthink-head {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.5rem 0.6rem;
  border: none;
  background: transparent;
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
}

.zthink-head:hover {
  background: var(--bg-hover);
}

.zthink-chevron {
  flex-shrink: 0;
  color: var(--text-muted);
  transition: transform 0.12s ease;
}

.zthink-item.open .zthink-chevron {
  transform: rotate(90deg);
}

.zthink-num {
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
  font-weight: 700;
  font-size: 0.78rem;
  color: var(--node-accent-thinking);
}

.zthink-total {
  color: var(--text-muted);
  font-weight: 500;
}

.zthink-summary {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.8rem;
  color: var(--text-secondary);
}

.zthink-tag {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  flex-shrink: 0;
  padding: 0.08rem 0.35rem;
  border-radius: 999px;
  font-size: 0.66rem;
  font-weight: 700;
}

.revision-tag {
  color: color-mix(in srgb, var(--node-accent-thinking) 80%, var(--text-primary));
  background: color-mix(in srgb, var(--node-accent-thinking) 20%, transparent);
}

.branch-tag {
  color: var(--text-secondary);
  background: var(--bg-hover);
}

.zthink-body {
  padding: 0 0.6rem 0.6rem 1.7rem;
}

.zthink-text {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-size: 0.79rem;
  line-height: 1.5;
  color: var(--text-primary);
}

.zthink-agent {
  margin: 0.4rem 0 0;
  font-size: 0.7rem;
  color: var(--text-muted);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}
</style>
