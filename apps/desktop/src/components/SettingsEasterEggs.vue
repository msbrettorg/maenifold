<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { useMcpClient } from '../composables/useMcpClient';
import { unwrapToolData } from '../composables/pipeline-mcp-transport';
import type { AgentsConfig, MaenifoldSettings, WallpaperAzureSettings, WallpaperSettings } from '../lib/types';
import { useConfigStore } from '../stores/config';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import SettingsEasterEggsAzure from './SettingsEasterEggsAzure.vue';
import SettingsEasterEggsContinuity from './SettingsEasterEggsContinuity.vue';

type WallpaperMode = 'endophasia' | 'zeitgeist' | 'self-actualization';
type Fidelity = 'low' | 'medium' | 'high';

const DEFAULT_PROMPT = 'Render the living shape of my maenifold knowledge graph as an ambient desktop wallpaper.';

const DEFAULT_WALLPAPER: WallpaperSettings = {
  enabled: false,
  mode: 'endophasia',
  prompt: DEFAULT_PROMPT,
  azure: {
    endpointEnvVar: 'AZURE_OPENAI_ENDPOINT',
    apiKeyEnvVar: 'AZURE_OPENAI_API_KEY',
    imageModel: 'gpt-image-2',
    smallModel: 'gpt-4.1-mini',
    reasoningModel: 'gpt-5.4',
  },
  size: '1536x1024',
  quality: 'high',
  continuity: false,
  inputFidelity: 'medium',
  maxRetries: 3,
  communityCount: 5,
  rotateMinutes: 15,
};

const modeOptions: { value: WallpaperMode; label: string; description: string }[] = [
  { value: 'endophasia', label: 'Endophasia', description: 'The inner monologue — renders recent activity as imagery' },
  { value: 'zeitgeist', label: 'Zeitgeist', description: 'The spirit of the time — one image per concept community' },
  {
    value: 'self-actualization',
    label: 'Self-actualization',
    description: 'The self-portrait — maenifold reads its own knowledge graph',
  },
];

const configStore = useConfigStore();
const mcp = useMcpClient();
const form = reactive<WallpaperSettings>(cloneWallpaper(DEFAULT_WALLPAPER));
const isLoading = ref(true);
const isSubmitting = ref(false);
const feedback = ref<{ type: 'success' | 'error'; message: string } | null>(null);

const showCommunityCount = computed(() => form.mode === 'zeitgeist' || form.mode === 'self-actualization');

function cloneWallpaper(settings: WallpaperSettings): WallpaperSettings {
  return JSON.parse(JSON.stringify(settings)) as WallpaperSettings;
}

function cloneSettings(settings: MaenifoldSettings): MaenifoldSettings {
  return JSON.parse(JSON.stringify(settings)) as MaenifoldSettings;
}

function normalizeWallpaper(settings?: WallpaperSettings): WallpaperSettings {
  return {
    ...cloneWallpaper(DEFAULT_WALLPAPER),
    ...(settings ?? {}),
    azure: {
      ...DEFAULT_WALLPAPER.azure,
      ...(settings?.azure ?? {}),
    },
  };
}

function applyWallpaper(settings?: WallpaperSettings) {
  Object.assign(form, normalizeWallpaper(settings));
}

function updateNumber(field: 'rotateMinutes' | 'communityCount', value: string | number, min: number, max: number) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return;
  form[field] = Math.min(max, Math.max(min, Math.round(parsed)));
}

function updateAzureField(field: keyof Pick<WallpaperAzureSettings, 'endpointEnvVar' | 'apiKeyEnvVar' | 'imageModel'>, value: string) {
  form.azure[field] = value;
}

async function loadSettings() {
  isLoading.value = true;
  feedback.value = null;

  try {
    await configStore.loadConfig();
    if (configStore.error) {
      throw new Error(configStore.error);
    }

    applyWallpaper(configStore.config.settings?.easterEggs?.wallpaper);
  } catch (error) {
    feedback.value = {
      type: 'error',
      message: error instanceof Error ? error.message : String(error),
    };
  } finally {
    isLoading.value = false;
  }
}

async function saveWallpaper(successMessage: string) {
  isSubmitting.value = true;
  feedback.value = null;

  try {
    if (!mcp.connected.value) {
      await mcp.connect();
    }

    const currentSettings = configStore.config.settings;
    if (!currentSettings) {
      throw new Error('Maenifold settings are not loaded.');
    }

    const nextSettings = cloneSettings(currentSettings);
    nextSettings.easterEggs = {
      ...(nextSettings.easterEggs ?? {}),
      wallpaper: cloneWallpaper(form),
    };

    const nextConfig = unwrapToolData<AgentsConfig>(
      await mcp.callTool('UpdateSettings', { settings: nextSettings }),
    );
    configStore.updateFromEvent(nextConfig);
    applyWallpaper(nextConfig.settings?.easterEggs?.wallpaper ?? nextSettings.easterEggs.wallpaper);
    feedback.value = { type: 'success', message: successMessage };
  } catch (error) {
    feedback.value = {
      type: 'error',
      message: error instanceof Error ? error.message : String(error),
    };
  } finally {
    isSubmitting.value = false;
  }
}

function resetPrompt() {
  form.prompt = DEFAULT_PROMPT;
}

onMounted(loadSettings);
</script>

<template>
  <section class="flex flex-col gap-4" role="form" aria-label="Easter Eggs settings">
    <Card>
      <CardHeader>
        <CardTitle>Easter Eggs</CardTitle>
        <CardDescription>Experimental features and visual surprises.</CardDescription>
      </CardHeader>
      <CardContent class="flex flex-col gap-4">
        <div v-if="isLoading" class="rounded-md border border-border bg-muted/40 p-4 text-sm text-muted-foreground">
          Loading Easter Eggs settings…
        </div>

        <template v-else>
          <div
            v-if="feedback"
            class="rounded-md border px-4 py-3 text-sm"
            :class="feedback.type === 'success'
              ? 'border-success/40 bg-success/10 text-success'
              : 'border-destructive/40 bg-destructive/10 text-destructive'"
            role="status"
          >
            {{ feedback.message }}
          </div>

          <div class="flex items-center justify-between gap-4 rounded-md border bg-muted/40 p-4 max-sm:flex-col max-sm:items-stretch">
            <div class="flex flex-col gap-1">
              <Label for="easter-eggs-wallpaper-switch" class="text-sm font-semibold">Wallpaper slideshow</Label>
              <p class="m-0 text-sm text-muted-foreground">Generate AI wallpaper from your knowledge graph.</p>
            </div>
            <Switch
              id="easter-eggs-wallpaper-switch"
              :model-value="form.enabled"
              :disabled="isSubmitting"
              @update:model-value="(value: boolean) => { form.enabled = value; }"
            />
          </div>

          <div class="flex flex-col gap-3 rounded-md border bg-muted/40 p-4">
            <div class="flex flex-col gap-1">
              <Label for="easter-eggs-mode" class="text-sm font-semibold">Mode</Label>
              <p class="m-0 text-sm text-muted-foreground">Choose how the graph is interpreted for wallpaper generation.</p>
            </div>
            <Select
              :model-value="form.mode"
              @update:model-value="(value) => { form.mode = value as WallpaperMode; }"
            >
              <SelectTrigger id="easter-eggs-mode" :disabled="isSubmitting" class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="option in modeOptions" :key="option.value" :value="option.value">
                  <div class="flex flex-col gap-1">
                    <span>{{ option.label }}</span>
                    <span class="text-xs text-muted-foreground">{{ option.description }}</span>
                  </div>
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div class="flex flex-col gap-3 rounded-md border bg-muted/40 p-4">
            <div class="flex items-start justify-between gap-3 max-sm:flex-col max-sm:items-stretch">
              <div class="flex flex-col gap-1">
                <Label for="easter-eggs-prompt" class="text-sm font-semibold">Prompt</Label>
                <p class="m-0 text-sm text-muted-foreground">Guide the image model while the graph supplies context.</p>
              </div>
              <Button type="button" variant="outline" size="sm" :disabled="isSubmitting" @click="resetPrompt">
                Reset to default
              </Button>
            </div>
            <Textarea
              id="easter-eggs-prompt"
              :model-value="form.prompt"
              :disabled="isSubmitting"
              class="min-h-28"
              @update:model-value="(value) => { form.prompt = String(value); }"
            />
          </div>

          <SettingsEasterEggsAzure
            :azure="form.azure"
            :disabled="isSubmitting"
            @update-field="updateAzureField"
          />

          <div class="flex flex-col gap-3 rounded-md border bg-muted/40 p-4">
            <h3 class="text-base font-semibold">Slideshow</h3>
            <div class="grid gap-3 sm:grid-cols-2">
              <div class="flex flex-col gap-2">
                <Label for="easter-eggs-rotate" class="text-sm font-semibold">Rotate every (minutes)</Label>
                <Input
                  id="easter-eggs-rotate"
                  type="number"
                  min="1"
                  max="60"
                  :model-value="form.rotateMinutes"
                  :disabled="isSubmitting"
                  @update:model-value="(value) => updateNumber('rotateMinutes', value, 1, 60)"
                />
              </div>
              <div v-if="showCommunityCount" class="flex flex-col gap-2">
                <Label for="easter-eggs-communities" class="text-sm font-semibold">Concept communities</Label>
                <Input
                  id="easter-eggs-communities"
                  type="number"
                  min="1"
                  max="20"
                  :model-value="form.communityCount"
                  :disabled="isSubmitting"
                  @update:model-value="(value) => updateNumber('communityCount', value, 1, 20)"
                />
              </div>
            </div>
          </div>

          <SettingsEasterEggsContinuity
            :continuity="form.continuity"
            :input-fidelity="form.inputFidelity"
            :disabled="isSubmitting"
            @update-continuity="(value) => { form.continuity = value; }"
            @update-fidelity="(value) => { form.inputFidelity = value as Fidelity; }"
          />

          <div class="flex flex-col gap-2 sm:flex-row">
            <Button type="button" :disabled="isSubmitting" @click="saveWallpaper('Easter Eggs settings saved.')">
              {{ isSubmitting ? 'Saving…' : 'Save settings' }}
            </Button>
            <Button
              type="button"
              variant="outline"
              disabled
              title="Available when wallpaper service is running."
            >
              Generate now
            </Button>
          </div>
        </template>
      </CardContent>
    </Card>
  </section>
</template>
