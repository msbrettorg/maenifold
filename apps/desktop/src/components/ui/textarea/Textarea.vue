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
  <textarea
    v-model="modelValue"
    data-slot="textarea"
    :class="cn('bg-[#1c1b1b] border-[rgba(66,71,83,0.15)] focus-visible:border-ring focus-visible:ring-ring/30 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive dark:aria-invalid:border-destructive/50 flex min-h-16 w-full resize-none rounded-[4px] border px-3 py-3 text-base transition-[color,box-shadow,background-color] outline-none placeholder:text-[var(--text-dim)] disabled:cursor-not-allowed disabled:opacity-50 focus-visible:ring-3 aria-invalid:ring-3 md:text-sm field-sizing-content', props.class)"
  />
</template>
