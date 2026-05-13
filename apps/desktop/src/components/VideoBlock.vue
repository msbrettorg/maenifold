<script setup lang="ts">
defineProps<{
  src: string;
  mimeType: string;
}>();
</script>

<template>
  <video
    class="max-h-[480px] max-w-full rounded border border-border-subtle bg-bg-app"
    controls
    preload="metadata"
    playsinline
  >
    <source
      :src="src"
      :type="mimeType"
    />
    <a
      class="text-sm text-brand underline underline-offset-4"
      :href="src"
      download
    >
      Download video
    </a>
  </video>
</template>
