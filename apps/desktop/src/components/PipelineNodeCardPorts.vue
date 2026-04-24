<script setup lang="ts">
import { Handle, Position } from '@vue-flow/core';
import { computed, inject, ref } from 'vue';

const props = defineProps<{
  direction?: 'LR' | 'TB';
}>();

const injectedDirection = inject('pipelineLayoutDirection', ref<'LR' | 'TB'>('LR'));
const direction = computed<'LR' | 'TB'>(() => props.direction ?? injectedDirection.value);

const baseHandleClasses =
  '!w-3 !h-3 !border-2 !border-bg-app !bg-primary !rounded-full transition-opacity';
</script>

<template>
  <div class="contents">
    <Handle
      id="in-top"
      type="target"
      :position="Position.Top"
      :class="[baseHandleClasses, '!-top-1.5', direction !== 'TB' ? 'opacity-0 pointer-events-none' : 'opacity-100']"
    />
    <Handle
      id="out-bottom"
      type="source"
      :position="Position.Bottom"
      :class="[baseHandleClasses, '!-bottom-1.5', direction !== 'TB' ? 'opacity-0 pointer-events-none' : 'opacity-100']"
    />
    <Handle
      id="in-left"
      type="target"
      :position="Position.Left"
      :class="[baseHandleClasses, '!-left-1.5', direction !== 'LR' ? 'opacity-0 pointer-events-none' : 'opacity-100']"
    />
    <Handle
      id="out-right"
      type="source"
      :position="Position.Right"
      :class="[baseHandleClasses, '!-right-1.5', direction !== 'LR' ? 'opacity-0 pointer-events-none' : 'opacity-100']"
    />
  </div>
</template>
