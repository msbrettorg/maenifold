<script setup lang="ts">
import { computed } from 'vue';
import { BaseEdge, EdgeText, type EdgeProps } from '@vue-flow/core';

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

const indicatorGlyph = computed(() =>
  indicator.value === 'mapped' ? '⇄' : indicator.value === 'warning' ? '⚠' : '',
);

const indicatorClass = computed(() => {
  if (indicator.value === 'warning') {
    return 'fill-amber-500/20 stroke-amber-500';
  }
  if (indicator.value === 'mapped') {
    return 'fill-emerald-500/20 stroke-emerald-500';
  }
  return 'fill-surface stroke-border-default';
});

const indicatorTextClass = computed(() =>
  indicator.value === 'warning' ? 'fill-amber-600' : 'fill-text-primary',
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

  <EdgeText
    v-if="label"
    :x="pathMemo.x"
    :y="pathMemo.y"
    :label="label"
    :label-show-bg="true"
    :label-bg-padding="[6, 3]"
    :label-bg-border-radius="4"
    :label-style="{ fontSize: '11px', fontFamily: 'inherit' }"
    class="fill-text-primary pointer-events-none [&>rect]:fill-surface [&>rect]:stroke-border-default"
  >
    <title>{{ labelTitle ?? tooltip }}</title>
  </EdgeText>

  <EdgeText
    v-if="indicator"
    :x="pathMemo.x"
    :y="pathMemo.y"
    :label="indicatorGlyph"
    :label-show-bg="true"
    :label-bg-padding="[6, 6]"
    :label-bg-border-radius="999"
    :class="[indicatorClass, indicatorTextClass, 'pointer-events-none']"
    :label-style="{ fontSize: '12px', fontWeight: '600', fontFamily: 'inherit' }"
  >
    <title>{{ tooltip }}</title>
  </EdgeText>
</template>
