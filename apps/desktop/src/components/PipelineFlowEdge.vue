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
    <!-- audit-ok: dynamic-binding (computed SVG edge styling from Vue Flow) -->
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
    <!-- audit-ok: dynamic-binding (computed transform for edge label positioning along SVG path) -->
    <div
      class="absolute py-[0.15rem] px-[0.35rem] rounded-[var(--radius-4)] bg-[var(--surface-high)] text-[var(--text)] text-[length:var(--text-xs)] leading-[1.2] pointer-events-none max-w-[18ch] whitespace-nowrap overflow-hidden text-ellipsis"
      :style="{ transform: `translate(-50%, -50%) translate(${pathMemo.x}px, ${pathMemo.y}px)` }"
      :title="labelTitle ?? undefined"
    >
      {{ label }}
    </div>
  </EdgeLabelRenderer>

  <EdgeLabelRenderer v-if="indicator">
    <!-- audit-ok: dynamic-binding (computed transform + CSS custom property + color-mix backgrounds for edge indicator) -->
    <div
      class="absolute grid place-items-center w-[1.4rem] h-[1.4rem] rounded-[var(--radius-4)] text-[length:var(--text-sm)] [font-weight:var(--weight-title)] shadow-[var(--shadow-8)] pointer-events-none"
      :style="{
        transform: `translate(-50%, -50%) translate(${pathMemo.x}px, ${pathMemo.y}px)`,
        '--edge-indicator-accent': indicatorColor,
        background: indicator === 'mapped' || indicator === 'warning'
          ? 'color-mix(in srgb, var(--color-bg-app) 82%, var(--edge-indicator-accent) 18%)'
          : 'color-mix(in srgb, var(--color-bg-app) 92%, var(--surface-high) 8%)',
        borderColor: indicator === 'warning' ? 'var(--edge-indicator-accent)' : undefined,
        color: indicator === 'warning'
          ? 'color-mix(in srgb, var(--edge-indicator-accent) 72%, var(--color-text-primary))'
          : 'var(--color-text-primary)',
      }"
      :title="tooltip"
    >
      {{ indicator === 'mapped' ? '⇄' : '⚠' }}
    </div>
  </EdgeLabelRenderer>
</template>
