<script setup lang="ts">
export interface SheetProps {
  open?: boolean
  defaultOpen?: boolean
}

import { DialogRoot, useForwardPropsEmits } from 'reka-ui'

const props = defineProps<SheetProps>()
const emits = defineEmits<{
  'update:open': [value: boolean]
}>()

const forwarded = useForwardPropsEmits(props, emits)
</script>

<template>
  <DialogRoot v-bind="forwarded">
    <slot />
  </DialogRoot>
</template>
