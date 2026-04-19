<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { ref, watch } from 'vue'
import { cn } from '@/lib/utils'

interface Props {
  class?: HTMLAttributes['class']
  defaultValue?: string | number
  modelValue?: string | number
}

const props = defineProps<Props>()

const emits = defineEmits<{
  (e: 'update:modelValue', payload: string | number): void
}>()

const modelValue = ref(props.modelValue ?? props.defaultValue ?? '')

watch(
  () => props.modelValue,
  (value) => {
    if (value !== undefined) {
      modelValue.value = value
    }
  },
)

watch(
  () => props.defaultValue,
  (value) => {
    if (props.modelValue === undefined && value !== undefined) {
      modelValue.value = value
    }
  },
)

watch(modelValue, (value) => {
  emits('update:modelValue', value)
})
</script>

<template>
  <input
    v-model="modelValue"
    data-slot="input"
    :class="cn(
      'bg-[#1c1b1b] border-[rgba(66,71,83,0.15)] focus-visible:border-ring focus-visible:ring-ring/30 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive dark:aria-invalid:border-destructive/50 h-9 w-full min-w-0 rounded-[4px] border px-3 py-1 text-base transition-[color,box-shadow,background-color] outline-none placeholder:text-[var(--text-dim)] file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 focus-visible:ring-3 aria-invalid:ring-3 md:text-sm',
      props.class,
    )"
  >
</template>
