<script setup lang="ts">
import Plyr from 'plyr';
import 'plyr/dist/plyr.css';
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';

const props = defineProps<{
  src: string;
  mimeType?: string;
  label?: string;
}>();

const audioRef = ref<HTMLAudioElement | null>(null);
const ariaLabel = computed(() => props.label ?? 'Audio playback');
let player: InstanceType<typeof Plyr> | null = null;

const controls = ['play', 'progress', 'current-time', 'mute', 'volume'] as const;

onMounted(() => {
  if (!audioRef.value) return;
  player = new Plyr(audioRef.value, { controls: [...controls] });
});

onBeforeUnmount(() => {
  player?.destroy();
  player = null;
});
</script>

<template>
  <div
    data-testid="audio-plyr-container"
    class="w-[min(100%,28rem)] max-w-full"
  >
    <audio
      ref="audioRef"
      class="w-full"
      preload="metadata"
      :aria-label="ariaLabel"
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
        Download audio
      </a>
    </audio>
  </div>
</template>
