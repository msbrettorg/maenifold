<script setup lang="ts">
import type { AlertDialogContentEmits, AlertDialogContentProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { computed } from 'vue'
import {
  AlertDialogContent,
  AlertDialogOverlay,
  AlertDialogPortal,
  useForwardPropsEmits,
} from 'reka-ui'
import { cn } from '@/lib/utils'

defineOptions({
  inheritAttrs: false,
})

const props = withDefaults(
  defineProps<AlertDialogContentProps & {
    class?: HTMLAttributes['class']
    size?: 'default' | 'sm'
  }>(),
  {
    size: 'default',
  },
)
const emits = defineEmits<AlertDialogContentEmits>()

const delegatedProps = computed(() => {
  const { class: _class, size: _size, ...rest } = props
  return rest
})

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
  <AlertDialogPortal>
    <AlertDialogOverlay
      data-slot="alert-dialog-overlay"
      class="fixed inset-0 z-50 bg-background/80 duration-100 data-open:animate-in data-closed:animate-out data-closed:fade-out-0 data-open:fade-in-0"
    />
    <AlertDialogContent
      data-slot="alert-dialog-content"
      :data-size="size"
      v-bind="{ ...$attrs, ...forwarded }"
      :class="
        cn(
          'group/alert-dialog-content fixed top-1/2 left-1/2 z-50 grid w-full -translate-x-1/2 -translate-y-1/2 gap-6 rounded-[8px] border-0 bg-[var(--color-bg-elevated)] p-6 text-sm text-popover-foreground shadow-[0_32px_64px_rgba(0,0,0,0.32),0_0_8px_rgba(0,0,0,0.16)] outline-none duration-100 data-open:animate-in data-closed:animate-out data-closed:fade-out-0 data-open:fade-in-0 data-closed:zoom-out-95 data-open:zoom-in-95 data-[size=default]:max-w-xs data-[size=sm]:max-w-xs data-[size=default]:sm:max-w-md',
          props.class,
        )
      "
    >
      <slot />
    </AlertDialogContent>
  </AlertDialogPortal>
</template>
