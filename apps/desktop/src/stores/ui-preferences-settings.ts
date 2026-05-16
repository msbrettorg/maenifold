import { StorageSerializers, useLocalStorage } from '@vueuse/core';
import { defineStore } from 'pinia';
import { watch } from 'vue';
import { PREFS_STORE_PATH } from '../lib/constants';
import { storageKey } from '../lib/storage-key';
import { initTelemetry, setTelemetryEnabled as setTelemetryRuntimeEnabled } from '../lib/telemetry';

const STORAGE_KEY = 'uiPreferencesSettings';
const LEGACY_TELEMETRY_KEY = 'telemetryEnabled';
const LEGACY_RESTORE_CWD_KEY = 'restoreLastWorkingDirectory';

export type ThemePreference = 'dark';
export type FontScalePreference = 'compact' | 'default' | 'comfortable';

export interface UiPreferencesSettingsState {
  theme: ThemePreference;
  fontScale: FontScalePreference;
  denseMode: boolean;
  telemetryEnabled: boolean;
  restoreLastWorkingDirectory: boolean;
}

const TEXT_TOKENS = [
  '--text-2xs',
  '--text-xs',
  '--text-sm',
  '--text-base',
  '--text-md',
  '--text-lg',
] as const;

const FONT_SCALES: Record<FontScalePreference, string[]> = {
  compact: ['0.5625rem', '0.625rem', '0.6875rem', '0.75rem', '0.8125rem', '0.875rem'],
  default: ['0.625rem', '0.6875rem', '0.75rem', '0.8125rem', '0.875rem', '0.9375rem'],
  comfortable: ['0.6875rem', '0.75rem', '0.8125rem', '0.9375rem', '1rem', '1.0625rem'],
};

function createDefaultState(): UiPreferencesSettingsState {
  return {
    theme: 'dark',
    fontScale: 'default',
    denseMode: false,
    telemetryEnabled: false,
    restoreLastWorkingDirectory: true,
  };
}

function applyTheme(theme: ThemePreference) {
  if (typeof document === 'undefined') {
    return;
  }

  const root = document.documentElement;
  root.classList.toggle('dark', theme === 'dark');
}

function applyFontScale(fontScale: FontScalePreference) {
  if (typeof document === 'undefined') {
    return;
  }

  const rootStyle = document.documentElement.style;
  if (fontScale === 'default') {
    for (const token of TEXT_TOKENS) {
      rootStyle.removeProperty(token);
    }
    return;
  }

  const scale = FONT_SCALES[fontScale];
  for (let index = 0; index < TEXT_TOKENS.length; index += 1) {
    rootStyle.setProperty(TEXT_TOKENS[index], scale[index]);
  }
}

function applyDenseMode(enabled: boolean) {
  if (typeof document === 'undefined') {
    return;
  }

  const rootStyle = document.documentElement.style;
  if (!enabled) {
    rootStyle.removeProperty('--space-2');
    rootStyle.removeProperty('--space-3');
    rootStyle.removeProperty('--space-4');
    rootStyle.removeProperty('--spacing-2');
    rootStyle.removeProperty('--spacing-3');
    rootStyle.removeProperty('--spacing-4');
    return;
  }

  rootStyle.setProperty('--space-2', '6px');
  rootStyle.setProperty('--space-3', '10px');
  rootStyle.setProperty('--space-4', '14px');
  rootStyle.setProperty('--spacing-2', '6px');
  rootStyle.setProperty('--spacing-3', '10px');
  rootStyle.setProperty('--spacing-4', '14px');
}

export const useUiPreferencesSettingsStore = defineStore('uiPreferencesSettings', () => {
  const settings = useLocalStorage<UiPreferencesSettingsState>(
    storageKey(PREFS_STORE_PATH, STORAGE_KEY),
    createDefaultState(),
    {
      mergeDefaults: true,
      serializer: StorageSerializers.object,
    },
  );
  const legacyTelemetryEnabled = useLocalStorage<boolean | null>(
    storageKey(PREFS_STORE_PATH, LEGACY_TELEMETRY_KEY),
    null,
    { serializer: StorageSerializers.object },
  );
  const legacyRestoreCwd = useLocalStorage<boolean | null>(
    storageKey(PREFS_STORE_PATH, LEGACY_RESTORE_CWD_KEY),
    null,
    { serializer: StorageSerializers.object },
  );

  watch(
    settings,
    (next) => {
      legacyTelemetryEnabled.value = next.telemetryEnabled;
      legacyRestoreCwd.value = next.restoreLastWorkingDirectory;
    },
    { deep: true },
  );

  function applyRuntimeState() {
    applyTheme(settings.value.theme);
    applyFontScale(settings.value.fontScale);
    applyDenseMode(settings.value.denseMode);
  }

  function init() {
    if (typeof legacyTelemetryEnabled.value === 'boolean') {
      settings.value.telemetryEnabled = legacyTelemetryEnabled.value;
    }
    if (typeof legacyRestoreCwd.value === 'boolean') {
      settings.value.restoreLastWorkingDirectory = legacyRestoreCwd.value;
    }
    applyRuntimeState();
  }

  async function setTelemetryEnabled(enabled: boolean) {
    settings.value.telemetryEnabled = enabled;
    if (enabled) {
      await initTelemetry(true);
      return;
    }
    setTelemetryRuntimeEnabled(false);
  }

  function setTheme(theme: ThemePreference) {
    settings.value.theme = theme;
    applyTheme(theme);
  }

  function setFontScale(fontScale: FontScalePreference) {
    settings.value.fontScale = fontScale;
    applyFontScale(fontScale);
  }

  function setDenseMode(enabled: boolean) {
    settings.value.denseMode = enabled;
    applyDenseMode(enabled);
  }

  function setRestoreLastWorkingDirectory(enabled: boolean) {
    settings.value.restoreLastWorkingDirectory = enabled;
  }

  return {
    settings,
    init,
    setTheme,
    setFontScale,
    setDenseMode,
    setTelemetryEnabled,
    setRestoreLastWorkingDirectory,
  };
});
