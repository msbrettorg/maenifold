<script setup lang="ts">
import { computed, ref } from 'vue';
import type { Component } from 'vue';
import { RefreshCw, AlertTriangle, Brain, CheckCircle } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import type { ThinkingSession } from '../lib/workflow-types';
import KnowledgeNoteRenderer from './KnowledgeNoteRenderer.vue';
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
  active: {
    label: 'Active',
    icon: RefreshCw,
    className: 'bg-[color-mix(in_srgb,var(--color-status-active)_14%,var(--color-bg-surface))] border-[color-mix(in_srgb,var(--color-status-active)_28%,transparent)] text-[color-mix(in_srgb,var(--color-text-primary)_74%,var(--color-status-active))]',
  },
  completed: {
    label: 'Completed',
    icon: CheckCircle,
    className: 'bg-[color-mix(in_srgb,var(--color-status-completed)_14%,var(--color-bg-surface))] border-[color-mix(in_srgb,var(--color-status-completed)_28%,transparent)] text-[color-mix(in_srgb,var(--color-text-primary)_72%,var(--color-status-completed))]',
  },
  abandoned: {
    label: 'Abandoned',
    icon: AlertTriangle,
    className: 'bg-[color-mix(in_srgb,var(--color-status-stuck)_14%,var(--color-bg-surface))] border-[color-mix(in_srgb,var(--color-status-stuck)_28%,transparent)] text-[color-mix(in_srgb,var(--color-text-primary)_68%,var(--color-status-stuck))]',
  },
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
  <!-- audit-ok: dynamic-binding (color-mix locals consumed by descendants) -->
  <div
    class="thinking-session-card w-[280px] bg-[color-mix(in_srgb,var(--color-bg-surface)_92%,var(--color-bg-app))] rounded-[var(--radius)] border-l-[3px] border-l-[var(--color-node-prompt)] text-[var(--thinking-card-text)] shadow-[var(--shadow-16)] overflow-hidden"
    :style="{
      '--thinking-card-text': 'color-mix(in srgb, var(--color-text-primary) 84%, var(--color-node-prompt))',
      '--thinking-card-heading': 'color-mix(in srgb, var(--color-text-primary) 94%, var(--color-node-prompt))',
      '--thinking-card-muted': 'color-mix(in srgb, var(--color-text-primary) 56%, var(--color-text-secondary))',
      '--thinking-card-badge': 'color-mix(in srgb, var(--color-text-primary) 72%, var(--color-node-prompt))',
    }"
  >
    <Button
      variant="ghost"
      class="card-header w-full flex items-start justify-between gap-[0.75rem] px-[0.95rem] py-[0.875rem] bg-transparent border-0 text-inherit text-left cursor-pointer h-auto rounded-none hover:bg-[color-mix(in_srgb,var(--color-node-prompt)_8%,var(--color-bg-surface))]"
      :aria-expanded="isExpanded"
      @click="toggleExpanded"
    >
      <div class="header-left flex items-center gap-[0.625rem] min-w-0">
        <span
          class="header-icon w-[1.75rem] h-[1.75rem] rounded-[4px] inline-flex items-center justify-center bg-[color-mix(in_srgb,var(--color-node-prompt)_14%,var(--color-bg-surface))] flex-shrink-0"
          aria-hidden="true"
        ><Brain class="size-4" :stroke-width="1.5" /></span>
        <div class="header-copy flex flex-col min-w-0">
          <span class="header-title text-[length:var(--text-body-sm)] font-[var(--weight-title)] text-[var(--thinking-card-heading)]">Thinking session</span>
          <span
            class="session-id font-[var(--font-code)] text-[length:var(--text-tiny)] text-[var(--thinking-card-muted)] whitespace-nowrap overflow-hidden text-ellipsis"
            :title="node.sessionId"
          >{{ shortSessionId }}</span>
        </div>
      </div>

      <div class="header-right flex items-center justify-end flex-wrap gap-[0.4rem] min-w-0">
        <span
          class="thought-count-badge inline-flex items-center gap-[0.3rem] rounded-[4px] px-[0.5rem] py-[0.2rem] text-[length:var(--text-tiny)] leading-[1.1] whitespace-nowrap bg-[color-mix(in_srgb,var(--color-bg-surface)_76%,var(--color-bg-app))] text-[color-mix(in_srgb,var(--color-text-primary)_74%,var(--color-node-prompt))]"
        >{{ thoughtCountLabel }}</span>
        <span
          class="status-badge inline-flex items-center gap-[0.3rem] rounded-[4px] px-[0.5rem] py-[0.2rem] text-[length:var(--text-tiny)] leading-[1.1] whitespace-nowrap border border-transparent font-[var(--weight-title)]"
          :class="statusMeta.className"
          :title="node.status"
        >
          <span aria-hidden="true"><component :is="statusMeta.icon" class="size-4" :stroke-width="1.5" /></span>
          {{ statusMeta.label }}
        </span>
        <span
          class="expand-indicator text-[var(--thinking-card-muted)] text-[length:var(--text-tiny)] pt-[0.35rem]"
          aria-hidden="true"
        >{{ isExpanded ? '▲' : '▼' }}</span>
      </div>
    </Button>
    <Transition @before-enter="onBeforeEnter" @enter="onEnter" @after-enter="onAfterEnter" @before-leave="onBeforeLeave" @leave="onLeave" @after-leave="onAfterLeave">
      <div v-if="isExpanded" class="card-body px-[0.95rem] pb-[0.95rem]">
        <ul
          v-if="node.thoughts.length"
          class="thought-list relative list-none m-0 pl-[0.35rem] flex flex-col gap-[0.8rem]"
        >
          <ThinkingThoughtItem v-for="thought in node.thoughts" :key="`${thought.number}-${thought.timestamp}`" :thought="thought" />
        </ul>
        <div
          v-else
          class="empty-state py-[0.75rem] text-[length:var(--text-caption)] text-[var(--thinking-card-muted)]"
        >No thoughts recorded for this session.</div>
        <section
          v-if="node.conclusion"
          class="conclusion-section mt-[0.95rem] p-[0.75rem] border border-[color-mix(in_srgb,var(--color-node-prompt)_18%,transparent)] rounded-[var(--radius)] bg-[color-mix(in_srgb,var(--color-node-prompt)_6%,var(--color-bg-surface))]"
        >
          <div
            class="conclusion-label mb-[0.45rem] text-[length:var(--text-tiny)] font-[var(--weight-title)] tracking-[0.06em] text-[var(--thinking-card-badge)]"
          >Conclusion</div>
          <KnowledgeNoteRenderer :content="node.conclusion" />
        </section>
      </div>
    </Transition>
  </div>
</template>

<!-- audit-ok: pseudo-element-gradient (timeline ::before with linear-gradient + color-mix unrepresentable in Tailwind) -->
<style scoped>
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
</style>

