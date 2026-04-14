<script setup lang="ts">
import type { AlertDialogCancelProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import type { ButtonVariants } from '@/components/ui/button'
import { computed } from 'vue'
import { AlertDialogCancel } from 'reka-ui'
import { cn } from '@/lib/utils'
import { buttonVariants } from '@/components/ui/button'

const props = withDefaults(
  defineProps<AlertDialogCancelProps & {
    class?: HTMLAttributes['class']
    variant?: ButtonVariants['variant']
    size?: ButtonVariants['size']
  }>(),
  {
    variant: 'outline',
    size: 'default',
  },
)

const delegatedProps = computed(() => {
  const { class: _class, variant: _variant, size: _size, ...rest } = props
  return rest
})
</script>

<template>
  <AlertDialogCancel
    data-slot="alert-dialog-cancel"
    v-bind="delegatedProps"
    :class="cn(
      '',
      buttonVariants({ variant, size }),
      props.class,
    )"
  >
    <slot />
  </AlertDialogCancel>
</template>
