<script setup lang="ts">
import { DialogTrigger } from 'reka-ui'
</script>

<template>
  <DialogTrigger v-bind="$attrs">
    <slot />
  </DialogTrigger>
</template>
