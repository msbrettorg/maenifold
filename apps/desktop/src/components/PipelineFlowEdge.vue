<script setup lang="ts">
import { computed } from 'vue';
import { BaseEdge, EdgeLabelRenderer, type EdgeProps } from '@vue-flow/core';

import { STATUS_COLORS } from '@/lib/pipeline-status-tokens';
import {
  edgeIndicator,
  edgeLabel,
  edgeLabelTitle,
  edgePath,
  edgePathStyle,
  edgeTooltip,
  type FlowEdgeData,
} from '@/lib/pipeline-composer-edge-utils';

const props = defineProps<EdgeProps<FlowEdgeData>>();

const pathMemo = computed(() => edgePath(props));
const indicator = computed(() => edgeIndicator(props));
const label = computed(() => edgeLabel(props.data));
const labelTitle = computed(() => edgeLabelTitle(props.data));
const pathStyle = computed(() => edgePathStyle(props));
const tooltip = computed(() => edgeTooltip(props.data));
const indicatorColor = computed(() =>
  indicator.value === 'mapped' ? STATUS_COLORS.active : STATUS_COLORS.stuck,
);
</script>

<template>
  <g>
    <BaseEdge
      :id="id"
      :path="pathMemo.d"
      :style="pathStyle"
      :marker-start="markerStart"
      :marker-end="markerEnd"
      :interaction-width="interactionWidth"
    />
    <title>{{ tooltip }}</title>
  </g>

  <EdgeLabelRenderer v-if="label">
    <div
      class="edge-condition-label"
      :style="{ transform: `translate(-50%, -50%) translate(${pathMemo.x}px, ${pathMemo.y}px)` }"
      :title="labelTitle ?? undefined"
    >
      {{ label }}
    </div>
  </EdgeLabelRenderer>

  <EdgeLabelRenderer v-if="indicator">
    <div
      :class="['edge-indicator', `edge-indicator--${indicator}`]"
      :style="{
        transform: `translate(-50%, -50%) translate(${pathMemo.x}px, ${pathMemo.y}px)`,
        '--edge-indicator-accent': indicatorColor,
      }"
      :title="tooltip"
    >
      {{ indicator === 'mapped' ? '⇄' : '⚠' }}
    </div>
  </EdgeLabelRenderer>
</template>

<style scoped>
.edge-indicator {
  position: absolute;
  display: grid;
  place-items: center;
  width: 1.4rem;
  height: 1.4rem;
  border-radius: 4px;
  background: color-mix(in srgb, var(--color-bg-app) 92%, var(--surface-high) 8%);
  color: var(--color-text-primary);
  font-size: var(--text-sm);
  font-weight: var(--weight-title);
  box-shadow: 0 1px 4px color-mix(in srgb, var(--color-bg-app) 15%, transparent);
  pointer-events: none;
}

.edge-indicator--mapped,
.edge-indicator--warning {
  background: color-mix(in srgb, var(--color-bg-app) 82%, var(--edge-indicator-accent) 18%);
}

.edge-indicator--warning {
  border-color: var(--edge-indicator-accent);
  color: color-mix(in srgb, var(--edge-indicator-accent) 72%, var(--color-text-primary));
}

.edge-condition-label {
  position: absolute;
  padding: 0.15rem 0.35rem;
  border-radius: 4px;
  background: var(--surface-high);
  color: var(--text);
  font-size: var(--text-xs);
  line-height: 1.2;
  pointer-events: none;
  max-width: 18ch;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
