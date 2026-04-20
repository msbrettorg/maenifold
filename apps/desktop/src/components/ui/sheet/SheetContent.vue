<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { DialogContent, DialogOverlay, DialogPortal, useForwardPropsEmits } from 'reka-ui'
import type { DialogContentEmits, DialogContentProps } from 'reka-ui'
import { cn } from '@/lib/utils'
import { cva, type VariantProps } from 'class-variance-authority'
import SheetClose from './SheetClose.vue'

const sheetVariants = cva(
  'fixed z-50 gap-4 bg-background p-6 shadow-[0_8px_16px_rgba(0,0,0,0.28),0_0_2px_rgba(0,0,0,0.12)] transition ease-in-out data-[state=closed]:duration-300 data-[state=open]:duration-500 data-[state=open]:animate-in data-[state=closed]:animate-out',
  {
    variants: {
      side: {
        top: 'inset-x-0 top-0 border-b data-[state=closed]:slide-out-to-top data-[state=open]:slide-in-from-top',
        bottom: 'inset-x-0 bottom-0 border-t data-[state=closed]:slide-out-to-bottom data-[state=open]:slide-in-from-bottom',
        left: 'inset-y-0 left-0 h-full w-3/4 border-r data-[state=closed]:slide-out-to-left data-[state=open]:slide-in-from-left sm:max-w-sm',
        right: 'inset-y-0 right-0 h-full w-3/4 border-l data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right sm:max-w-sm',
      },
    },
    defaultVariants: {
      side: 'right',
    },
  },
)

type SheetContentVariants = VariantProps<typeof sheetVariants>

const props = defineProps<
  DialogContentProps & {
    class?: HTMLAttributes['class']
    side?: SheetContentVariants['side']
    showCloseButton?: boolean
  }
>()

const emits = defineEmits<DialogContentEmits>()

const forwarded = useForwardPropsEmits(
  () => {
    const { class: _, side: __, ...rest } = props
    return rest
  },
  emits,
)
</script>

<template>
  <DialogPortal>
    <DialogOverlay
      class="fixed inset-0 z-50 bg-black/80 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0"
    />
    <DialogContent
      :class="cn(sheetVariants({ side: props.side }), props.class)"
      v-bind="forwarded"
    >
      <slot />
      <SheetClose
        v-if="props.showCloseButton ?? true"
        class="absolute right-4 top-4 rounded-[4px] text-[var(--text-secondary)] opacity-100 transition-colors hover:bg-[var(--color-bg-surface)] hover:text-[var(--foreground)] focus:outline-none focus:ring-2 focus:ring-ring disabled:pointer-events-none"
      >
        <span class="sr-only">Close</span>
      </SheetClose>
    </DialogContent>
  </DialogPortal>
</template>
