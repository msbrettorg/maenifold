<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useMcpClient } from '../composables/useMcpClient';
import { unwrapToolData } from '../composables/pipeline-mcp-transport';
import { useConfigStore } from '../stores/config';

type WallpaperLayer = 0 | 1;

interface WallpaperStatusResponse {
  images?: unknown;
  currentIndex?: unknown;
}

const configStore = useConfigStore();
const mcp = useMcpClient();
const rootEl = ref<HTMLElement | null>(null);
const wallpaperImages = ref<string[]>([]);
const wallpaperLayers = ref<[string | null, string | null]>([null, null]);
const visibleWallpaperLayer = ref<WallpaperLayer>(0);
const activeWallpaperIndex = ref(0);
const hasWallpaper = computed(() => wallpaperImages.value.length > 0 && wallpaperLayers.value.some(Boolean));

let rotationTimer: ReturnType<typeof setInterval> | null = null;
let visibilityObserver: IntersectionObserver | null = null;
let refreshToken = 0;
let hasObservedInitialActivation = false;

function stopWallpaperRotation(): void {
  if (rotationTimer !== null) {
    clearInterval(rotationTimer);
    rotationTimer = null;
  }
}

function resetWallpaper(): void {
  stopWallpaperRotation();
  wallpaperImages.value = [];
  wallpaperLayers.value = [null, null];
  visibleWallpaperLayer.value = 0;
  activeWallpaperIndex.value = 0;
}

function normalizeIndex(index: unknown, imageCount: number): number {
  if (imageCount === 0 || typeof index !== 'number' || !Number.isFinite(index)) {
    return 0;
  }
  return ((Math.trunc(index) % imageCount) + imageCount) % imageCount;
}

function rotateWallpaper(): void {
  if (wallpaperImages.value.length <= 1) return;

  const nextIndex = (activeWallpaperIndex.value + 1) % wallpaperImages.value.length;
  const hiddenLayer: WallpaperLayer = visibleWallpaperLayer.value === 0 ? 1 : 0;
  wallpaperLayers.value[hiddenLayer] = wallpaperImages.value[nextIndex] ?? null;
  activeWallpaperIndex.value = nextIndex;
  visibleWallpaperLayer.value = hiddenLayer;
}

function startWallpaperRotation(rotateMinutes: number): void {
  stopWallpaperRotation();
  if (wallpaperImages.value.length <= 1) return;

  const intervalMs = Math.max(1, Math.round(rotateMinutes || 15)) * 60 * 1000;
  rotationTimer = setInterval(rotateWallpaper, intervalMs);
}

async function loadWallpaperImages(paths: string[]): Promise<string[]> {
  const readWallpaperImage = window.electronAPI?.readWallpaperImage;
  if (!readWallpaperImage) return [];

  const results = await Promise.all(
    paths.map(async (imagePath) => {
      try {
        return await readWallpaperImage(imagePath);
      } catch {
        return null;
      }
    }),
  );

  return results.filter((image): image is string => (
    typeof image === 'string' && image.startsWith('data:image/png;base64,')
  ));
}

async function refreshWallpaper(): Promise<void> {
  const token = ++refreshToken;
  try {
    if (!configStore.config.settings) {
      await configStore.loadConfig();
    }
    const wallpaperSettings = configStore.config.settings?.easterEggs?.wallpaper;
    if (!wallpaperSettings?.enabled) {
      resetWallpaper();
      return;
    }

    if (!mcp.connected.value) {
      await mcp.connect();
    }

    const status = unwrapToolData<WallpaperStatusResponse>(await mcp.callTool('WallpaperStatus', {}));
    const imagePaths = Array.isArray(status.images)
      ? status.images.filter((image): image is string => typeof image === 'string' && image.length > 0)
      : [];
    const images = await loadWallpaperImages(imagePaths);
    if (token !== refreshToken) return;

    if (images.length === 0) {
      resetWallpaper();
      return;
    }

    const startIndex = normalizeIndex(status.currentIndex, images.length);
    wallpaperImages.value = images;
    activeWallpaperIndex.value = startIndex;
    visibleWallpaperLayer.value = 0;
    wallpaperLayers.value = [images[startIndex] ?? null, null];
    startWallpaperRotation(wallpaperSettings.rotateMinutes);
  } catch {
    if (token === refreshToken) {
      resetWallpaper();
    }
  }
}

function startActivationObserver(): void {
  if (!rootEl.value || typeof IntersectionObserver === 'undefined') return;

  visibilityObserver = new IntersectionObserver((entries) => {
    const isActive = entries.some((entry) => entry.isIntersecting);
    if (!isActive) return;

    if (!hasObservedInitialActivation) {
      hasObservedInitialActivation = true;
      return;
    }

    void refreshWallpaper();
  });
  visibilityObserver.observe(rootEl.value);
}

onMounted(async () => {
  await refreshWallpaper();
  startActivationObserver();
});

onUnmounted(() => {
  refreshToken += 1;
  stopWallpaperRotation();
  visibilityObserver?.disconnect();
  visibilityObserver = null;
});
</script>

<template>
  <div ref="rootEl" class="absolute inset-0 pointer-events-none">
    <template v-if="hasWallpaper">
      <div
        class="absolute inset-0 -z-10 pointer-events-none bg-cover bg-center transition-opacity duration-1000"
        :class="visibleWallpaperLayer === 0 ? 'opacity-100' : 'opacity-0'"
        :style="{ backgroundImage: wallpaperLayers[0] ? `url(${wallpaperLayers[0]})` : 'none' }"
      />
      <div
        class="absolute inset-0 -z-10 pointer-events-none bg-cover bg-center transition-opacity duration-1000"
        :class="visibleWallpaperLayer === 1 ? 'opacity-100' : 'opacity-0'"
        :style="{ backgroundImage: wallpaperLayers[1] ? `url(${wallpaperLayers[1]})` : 'none' }"
      />
      <div class="absolute inset-0 -z-5 pointer-events-none bg-bg-surface/80" />
    </template>
  </div>
</template>
