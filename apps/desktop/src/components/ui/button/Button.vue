<script setup lang="ts">
import type { PrimitiveProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { computed } from 'vue'
import type { ButtonVariants } from '.'
import { Primitive } from 'reka-ui'
import { cn } from '@/lib/utils'
import { buttonVariants } from '.'

interface Props extends PrimitiveProps {
  variant?: ButtonVariants['variant']
  size?: ButtonVariants['size']
  class?: HTMLAttributes['class']
  disabled?: boolean
  loading?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  as: 'button',
  disabled: false,
  loading: false,
})

const isNativeButton = computed(() => props.as === 'button' && !props.asChild)
const isDisabled = computed(() => props.disabled || props.loading)
</script>

<template>
  <Primitive
    data-slot="button"
    :data-loading="loading ? 'true' : 'false'"
    :data-variant="variant"
    :data-size="size"
    :as="as"
    :as-child="asChild"
    :aria-busy="loading || undefined"
    :aria-disabled="isDisabled || undefined"
    :disabled="isNativeButton ? isDisabled : undefined"
    :class="cn(buttonVariants({ variant, size }), props.class)"
  >
    <slot />
  </Primitive>
</template>
