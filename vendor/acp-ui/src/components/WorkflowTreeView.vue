<script setup lang="ts">
import { onMounted, computed } from 'vue';
import { useWorkflowTree } from '../composables/useWorkflowTree';
import WorkflowNodeCard from './WorkflowNodeCard.vue';
import ThinkingSessionCard from './ThinkingSessionCard.vue';

// ─── Props ────────────────────────────────────────────────────────────────────
const props = defineProps<{
  uri: string;
}>();

// ─── Composable ───────────────────────────────────────────────────────────────
const { loadTree, tree, loading, error } = useWorkflowTree();

// ─── Load on mount ────────────────────────────────────────────────────────────
onMounted(() => {
  void loadTree(props.uri);
});

function retry(): void {
  void loadTree(props.uri);
}

// ─── Skeleton items for loading display ──────────────────────────────────────
const skeletonItems = [1, 2, 3] as const;

// ─── Determine if a workflow node is the root ─────────────────────────────────
const isRootLoaded = computed(() => tree.value !== null && tree.value.type === 'workflow');
</script>

<template>
  <div class="workflow-tree-view">

    <!-- ── Loading state ────────────────────────────────────────────────────── -->
    <div v-if="loading" class="tree-loading" aria-busy="true" aria-label="Loading workflow tree">
      <div class="loading-header">
        <div class="loading-pulse loading-badge" />
        <div class="loading-text-block">
          <div class="loading-pulse loading-line-wide" />
          <div class="loading-pulse loading-line-narrow" />
        </div>
      </div>
      <div class="loading-progress" />
      <div class="loading-steps">
        <div
          v-for="i in skeletonItems"
          :key="i"
          class="loading-step-row"
        >
          <div class="loading-pulse loading-step-icon" />
          <div class="loading-pulse loading-step-name" :style="{ width: `${55 + i * 13}%` }" />
        </div>
      </div>
      <div class="loading-connector-placeholder" />
      <div class="loading-children">
        <div
          v-for="i in skeletonItems"
          :key="i"
          class="skeleton-child"
        >
          <div class="loading-pulse skeleton-child-icon" />
          <div class="loading-pulse skeleton-child-label" :style="{ width: `${40 + i * 10}%` }" />
        </div>
      </div>
    </div>

    <!-- ── Error state ───────────────────────────────────────────────────────── -->
    <div v-else-if="error" class="tree-error" role="alert">
      <div class="error-icon">⚠️</div>
      <div class="error-body">
        <p class="error-title">Failed to load workflow tree</p>
        <p class="error-detail">{{ error }}</p>
      </div>
      <button type="button" class="retry-btn" @click="retry">
        ↺ Retry
      </button>
    </div>

    <!-- ── Tree ──────────────────────────────────────────────────────────────── -->
    <div v-else-if="isRootLoaded && tree" class="tree-root-scroll">
      <WorkflowTreeLevel :node="tree" :depth="0" />
    </div>

    <!-- ── Empty ─────────────────────────────────────────────────────────────── -->
    <div v-else class="tree-empty">
      No workflow data found.
    </div>
  </div>
</template>

<!-- ─── Recursive sub-component ─────────────────────────────────────────────── -->
<script lang="ts">
import { defineComponent, h, type PropType, type VNode } from 'vue';
import type { WorkflowTreeNode as WTN } from '../lib/workflow-types';

/**
 * WorkflowTreeLevel — recursively renders one level of the workflow tree.
 *
 * Layout per node:
 *   [WorkflowNodeCard or ThinkingSessionCard]
 *       │  (connector line)
 *   [children row with bracket connectors]
 *       recursion…
 */
export const WorkflowTreeLevel = defineComponent({
  name: 'WorkflowTreeLevel',

  props: {
    node: {
      type: Object as PropType<WTN>,
      required: true,
    },
    depth: {
      type: Number,
      default: 0,
    },
  },

  setup(props) {
    const CONN_WORKFLOW = '#22c55e';
    const CONN_THINKING = '#8b5cf6';

    function hasChildren(node: WTN): boolean {
      return node.children.length > 0;
    }

    function childColor(child: WTN): string {
      return child.type === 'thinking' ? CONN_THINKING : CONN_WORKFLOW;
    }

    /** Render the card for this node */
    function renderCard(node: WTN): VNode {
      if (node.type === 'workflow') {
        return h(WorkflowNodeCard, { node: node.data, key: node.data.uri });
      }
      return h(ThinkingSessionCard, { node: node.data, key: node.data.uri });
    }

    /**
     * Build SVG connector bracket for a list of children.
     *
     * For a single child: simple vertical drop
     *   ┃
     *   ●
     *
     * For multiple children: T-bracket
     *   ┃
     *   ┣━━━━┳━━━━┓
     *   ●    ●    ●
     */
    function renderConnector(children: WTN[]): VNode | null {
      if (children.length === 0) return null;

      const svgH = 40;          // connector SVG height in px
      const nodeW = 300;        // approximate card width
      const gap = 24;           // gap between cards
      const midX = nodeW / 2;   // centre of this (parent) node

      if (children.length === 1) {
        const col = childColor(children[0]);
        return h(
          'svg',
          {
            class: 'tree-connector single-connector',
            viewBox: `0 0 ${nodeW} ${svgH}`,
            width: nodeW,
            height: svgH,
            'aria-hidden': 'true',
          },
          [
            h('line', {
              x1: midX, y1: 0,
              x2: midX, y2: svgH,
              stroke: col,
              'stroke-width': 2,
              'stroke-dasharray': '4 3',
            }),
          ]
        );
      }

      // Multiple children — calculate positions
      const totalW = children.length * nodeW + (children.length - 1) * gap;
      const startX = midX - totalW / 2;

      const childMidXs = children.map((_, i) => startX + i * (nodeW + gap) + nodeW / 2);
      const bracketY = svgH * 0.65;

      const svgChildren: VNode[] = [
        // vertical drop from parent to bracket bar
        h('line', {
          x1: midX, y1: 0,
          x2: midX, y2: bracketY,
          stroke: '#4b5563',
          'stroke-width': 2,
          'stroke-dasharray': '4 3',
        }),
        // horizontal bracket bar
        h('line', {
          x1: childMidXs[0], y1: bracketY,
          x2: childMidXs[childMidXs.length - 1], y2: bracketY,
          stroke: '#4b5563',
          'stroke-width': 2,
        }),
        // vertical drops per child with colour
        ...children.flatMap((child, i) => {
          const cx = childMidXs[i];
          const col = childColor(child);
          return [
            h('line', {
              key: `drop-${i}`,
              x1: cx, y1: bracketY,
              x2: cx, y2: svgH,
              stroke: col,
              'stroke-width': 2,
              'stroke-dasharray': '4 3',
            }),
            // coloured dot at bottom
            h('circle', {
              key: `dot-${i}`,
              cx, cy: svgH - 3,
              r: 3,
              fill: col,
            }),
          ];
        }),
      ];

      return h(
        'svg',
        {
          class: 'tree-connector multi-connector',
          viewBox: `0 0 ${totalW} ${svgH}`,
          width: totalW,
          height: svgH,
          'aria-hidden': 'true',
          style: { alignSelf: 'flex-start' },
        },
        svgChildren,
      );
    }

    /** Render children row */
    function renderChildrenRow(children: WTN[], depth: number): VNode {
      return h(
        'div',
        { class: 'children-row' },
        children.map((child) =>
          h(WorkflowTreeLevel, {
            node: child,
            depth: depth + 1,
            key: child.type === 'workflow' ? child.data.uri : child.data.uri,
          })
        )
      );
    }

    return () => {
      const node = props.node;
      const depth = props.depth;
      const children = node.children;
      const showChildren = hasChildren(node);

      return h(
        'div',
        {
          class: [
            'tree-level',
            `depth-${depth}`,
            node.type === 'thinking' ? 'level-thinking' : 'level-workflow',
          ],
        },
        [
          renderCard(node),
          showChildren
            ? h('div', { class: 'connector-wrapper' }, [
                renderConnector(children),
              ])
            : null,
          showChildren ? renderChildrenRow(children, depth) : null,
        ].filter(Boolean) as VNode[]
      );
    };
  },
});
</script>

<style scoped>
/* ── Root container ─────────────────────────────────────────────────────────── */
.workflow-tree-view {
  background: #12131a;
  min-height: 200px;
  padding: 1.5rem;
  border-radius: 12px;
  font-family: -apple-system, BlinkMacSystemFont, 'Inter', 'Segoe UI', sans-serif;
}

/* ── Scroll wrapper for wide trees ─────────────────────────────────────────── */
.tree-root-scroll {
  overflow-x: auto;
  overflow-y: visible;
  padding-bottom: 1rem;
  /* nice scrollbar */
  scrollbar-width: thin;
  scrollbar-color: #2d3144 transparent;
}

.tree-root-scroll::-webkit-scrollbar {
  height: 6px;
}

.tree-root-scroll::-webkit-scrollbar-track {
  background: transparent;
}

.tree-root-scroll::-webkit-scrollbar-thumb {
  background: #2d3144;
  border-radius: 3px;
}

/* ── Tree level layout ──────────────────────────────────────────────────────── */

/* Each level is a column: card → connector → children row */
:global(.tree-level) {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0;
  flex-shrink: 0;
}

/* Children of a node spread out horizontally */
:global(.children-row) {
  display: flex;
  flex-direction: row;
  align-items: flex-start;
  gap: 24px;
  flex-shrink: 0;
}

/* Connector wrapper keeps SVG centred over parent card */
:global(.connector-wrapper) {
  display: flex;
  justify-content: center;
  align-items: flex-start;
  width: 100%;
  overflow: visible;
}

:global(.tree-connector) {
  display: block;
  overflow: visible;
  flex-shrink: 0;
}

/* ── Loading skeleton ──────────────────────────────────────────────────────── */
.tree-loading {
  width: 320px;
  background: #1e2028;
  border-radius: 8px;
  border-left: 3px solid #2d3144;
  padding: 0.875rem 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}

.loading-pulse {
  border-radius: 4px;
  background: linear-gradient(90deg, #252836 25%, #2d3144 50%, #252836 75%);
  background-size: 200% 100%;
  animation: shimmer 1.6s infinite;
}

@keyframes shimmer {
  0%   { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

.loading-header {
  display: flex;
  align-items: center;
  gap: 0.625rem;
}

.loading-badge {
  width: 28px;
  height: 28px;
  border-radius: 6px;
  flex-shrink: 0;
}

.loading-text-block {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  flex: 1;
}

.loading-line-wide  { height: 12px; width: 70%; }
.loading-line-narrow { height: 9px; width: 40%; }

.loading-progress {
  height: 4px;
  border-radius: 2px;
  background: linear-gradient(90deg, #252836 25%, #2d3144 50%, #252836 75%);
  background-size: 200% 100%;
  animation: shimmer 1.6s infinite;
}

.loading-steps {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.loading-step-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.loading-step-icon {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  flex-shrink: 0;
}

.loading-step-name {
  height: 10px;
}

.loading-connector-placeholder {
  height: 40px;
  display: flex;
  justify-content: center;
  align-items: center;
}

.loading-connector-placeholder::after {
  content: '';
  display: block;
  width: 2px;
  height: 100%;
  background: linear-gradient(180deg, #2d3144, transparent);
}

.loading-children {
  display: flex;
  gap: 0.5rem;
}

.skeleton-child {
  flex: 1;
  background: #1a1c24;
  border-radius: 8px;
  border-left: 3px solid #2d3144;
  padding: 0.6rem;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  min-width: 80px;
}

.skeleton-child-icon {
  width: 24px;
  height: 24px;
  border-radius: 50%;
}

.skeleton-child-label {
  height: 9px;
  border-radius: 3px;
}

/* ── Error state ────────────────────────────────────────────────────────────── */
.tree-error {
  display: flex;
  align-items: flex-start;
  gap: 0.875rem;
  background: rgba(239, 68, 68, 0.08);
  border: 1px solid rgba(239, 68, 68, 0.25);
  border-radius: 10px;
  padding: 1rem 1.25rem;
  color: #fca5a5;
  max-width: 480px;
}

.error-icon {
  font-size: 1.25rem;
  flex-shrink: 0;
  line-height: 1.3;
}

.error-body {
  flex: 1;
  min-width: 0;
}

.error-title {
  margin: 0 0 0.25rem;
  font-size: 0.875rem;
  font-weight: 600;
  color: #fca5a5;
}

.error-detail {
  margin: 0;
  font-size: 0.775rem;
  color: #f87171;
  font-family: 'SF Mono', 'Fira Code', monospace;
  overflow-wrap: anywhere;
}

.retry-btn {
  flex-shrink: 0;
  background: rgba(239, 68, 68, 0.14);
  border: 1px solid rgba(239, 68, 68, 0.3);
  color: #fca5a5;
  border-radius: 6px;
  padding: 0.375rem 0.75rem;
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.15s ease;
  align-self: center;
}

.retry-btn:hover {
  background: rgba(239, 68, 68, 0.22);
}

/* ── Empty state ────────────────────────────────────────────────────────────── */
.tree-empty {
  text-align: center;
  color: #4b5563;
  font-size: 0.875rem;
  padding: 2rem 0;
}
</style>
