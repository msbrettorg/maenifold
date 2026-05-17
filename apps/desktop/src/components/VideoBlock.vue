<script setup lang="ts">
import Plyr from 'plyr';
import 'plyr/dist/plyr.css';
import { onBeforeUnmount, onMounted, ref } from 'vue';

defineProps<{
  src: string;
  mimeType: string;
}>();

const videoRef = ref<HTMLVideoElement | null>(null);
let player: InstanceType<typeof Plyr> | null = null;

const controls = ['play', 'progress', 'current-time', 'mute', 'volume', 'fullscreen'] as const;

onMounted(() => {
  if (!videoRef.value) return;
  player = new Plyr(videoRef.value, { controls: [...controls] });
});

onBeforeUnmount(() => {
  player?.destroy();
  player = null;
});
</script>

<template>
  <div
    data-testid="video-plyr-container"
    class="max-w-full"
  >
    <video
      ref="videoRef"
      class="max-h-[480px] max-w-full rounded border border-border-subtle bg-bg-app"
      preload="metadata"
      playsinline
      :src="src"
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
  </div>
</template>
