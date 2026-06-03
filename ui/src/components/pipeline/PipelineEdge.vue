<script setup lang="ts">
// Custom Vue Flow edge for pipeline edges.
//
// Stroke style encodes the edge type: success = solid, always = long-dash
// (8 6), conditional = dotted (2 6). Conditional edges render a `?{path}="val"`
// label at the midpoint. Field-mapping edges render a ⇄ (ok) / ⚠ (warn) pill.
// Fan-out edges from one source are shelved apart (see buildEdgePath) so
// parallel edges do not overlap.
import { computed } from 'vue';
import { BaseEdge, EdgeLabelRenderer } from '@vue-flow/core';
import type { EdgeProps } from '@vue-flow/core';
import {
  buildEdgePath,
  edgeConditionLabel,
  edgeDashArray,
  edgeMappingIndicator,
  type FlowEdgeData,
} from './canvasBridge';

const props = defineProps<EdgeProps<FlowEdgeData>>();

const geometry = computed(() =>
  buildEdgePath({
    sourceX: props.sourceX,
    sourceY: props.sourceY,
    targetX: props.targetX,
    targetY: props.targetY,
    sourcePosition: props.sourcePosition,
    targetPosition: props.targetPosition,
    data: props.data,
  })
);

const dashArray = computed(() => edgeDashArray(props.data?.edgeType ?? 'success'));
const conditionLabel = computed(() => edgeConditionLabel(props.data));
const mappingIndicator = computed(() => edgeMappingIndicator(props.data));

const edgeStyle = computed(() => ({
  stroke: props.selected
    ? 'color-mix(in srgb, var(--text-accent) 70%, var(--text-muted))'
    : 'color-mix(in srgb, var(--text-muted) 85%, transparent)',
  strokeWidth: props.selected ? 2 : 1.5,
  ...(dashArray.value ? { strokeDasharray: dashArray.value } : {}),
}));

const labelTransform = computed(
  () =>
    `translate(-50%, -50%) translate(${geometry.value.labelX}px, ${geometry.value.labelY}px)`
);
</script>

<template>
  <BaseEdge :id="id" :path="geometry.d" :style="edgeStyle" :marker-end="markerEnd" />
  <EdgeLabelRenderer>
    <div
      v-if="conditionLabel || mappingIndicator"
      class="pedge-label"
      :style="{ transform: labelTransform }"
    >
      <span v-if="conditionLabel" class="pedge-cond">{{ conditionLabel }}</span>
      <span
        v-if="mappingIndicator"
        class="pedge-map"
        :class="mappingIndicator"
        :title="mappingIndicator === 'ok' ? 'Field mapping configured' : 'Field mapping incomplete'"
      >{{ mappingIndicator === 'ok' ? '⇄' : '⚠' }}</span>
    </div>
  </EdgeLabelRenderer>
</template>

<style scoped>
.pedge-label {
  position: absolute;
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  pointer-events: none;
  font-size: 0.66rem;
  font-weight: 600;
}

.pedge-cond {
  padding: 0.05rem 0.35rem;
  border: 1px solid var(--border-color);
  border-radius: 5px;
  background: color-mix(in srgb, var(--bg-main) 90%, var(--bg-sidebar));
  color: var(--text-secondary);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  white-space: nowrap;
}

.pedge-map {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  font-size: 0.7rem;
}

.pedge-map.ok {
  background: color-mix(in srgb, var(--node-accent-tool) 24%, transparent);
  color: color-mix(in srgb, var(--node-accent-tool) 85%, var(--text-primary));
}

.pedge-map.warn {
  background: color-mix(in srgb, var(--node-status-stuck) 26%, transparent);
  color: color-mix(in srgb, var(--node-status-stuck) 90%, var(--text-primary));
}
</style>
