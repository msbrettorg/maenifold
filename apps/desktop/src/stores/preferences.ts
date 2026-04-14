import { defineStore } from 'pinia';
import { ref } from 'vue';
import { PREFS_STORE_PATH } from '../lib/constants';
import { readStorageValue, writeStorageValue } from '../lib/local-storage';

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
  const fontDensity = ref<FontDensity>('default');

  function setFontDensity(density: FontDensity) {
    fontDensity.value = density;
    writeStorageValue(PREFS_STORE_PATH, STORAGE_KEY, density);
    applyScale(density);
  }

  function init() {
    const saved = readStorageValue<FontDensity>(PREFS_STORE_PATH, STORAGE_KEY);
    if (saved && saved in DENSITY_SCALES) {
      fontDensity.value = saved;
      applyScale(saved);
    }
  }

  return {
    fontDensity,
    setFontDensity,
    init,
  };
});
