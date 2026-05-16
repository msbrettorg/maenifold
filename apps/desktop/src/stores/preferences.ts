import { StorageSerializers, useLocalStorage } from '@vueuse/core';
import { defineStore } from 'pinia';
import { PREFS_STORE_PATH } from '../lib/constants';
import { storageKey } from '../lib/storage-key';

export type FontDensity = 'compact' | 'default' | 'comfortable';

const STORAGE_KEY = 'fontDensity';

const TEXT_TOKENS = [
  '--text-2xs',
  '--text-xs',
  '--text-sm',
  '--text-base',
  '--text-md',
  '--text-lg',
] as const;

const DENSITY_SCALES: Record<FontDensity, string[]> = {
  compact: [
    '0.5625rem',
    '0.625rem',
    '0.6875rem',
    '0.75rem',
    '0.8125rem',
    '0.875rem',
  ],
  default: [
    '0.625rem',
    '0.6875rem',
    '0.75rem',
    '0.8125rem',
    '0.875rem',
    '0.9375rem',
  ],
  comfortable: [
    '0.6875rem',
    '0.75rem',
    '0.8125rem',
    '0.9375rem',
    '1rem',
    '1.0625rem',
  ],
};

function applyScale(density: FontDensity): void {
  const root = document.documentElement.style;
  const scale = DENSITY_SCALES[density];

  if (density === 'default') {
    // Remove inline overrides so :root token values apply naturally
    for (const token of TEXT_TOKENS) {
      root.removeProperty(token);
    }
  } else {
    for (let i = 0; i < TEXT_TOKENS.length; i++) {
      root.setProperty(TEXT_TOKENS[i], scale[i]);
    }
  }
}

export const usePreferencesStore = defineStore('preferences', () => {
  const fontDensity = useLocalStorage<FontDensity>(
    storageKey(PREFS_STORE_PATH, STORAGE_KEY),
    'default',
    { serializer: StorageSerializers.object },
  );

  function setFontDensity(density: FontDensity) {
    fontDensity.value = density;
    applyScale(density);
  }

  function init() {
    if (typeof fontDensity.value === 'string' && fontDensity.value in DENSITY_SCALES) {
      applyScale(fontDensity.value);
      return;
    }
    fontDensity.value = 'default';
    applyScale('default');
  }

  return {
    fontDensity,
    setFontDensity,
    init,
  };
});
