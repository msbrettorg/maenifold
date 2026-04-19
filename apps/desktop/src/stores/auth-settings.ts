import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { PREFS_STORE_PATH } from '../lib/constants';
import { readStorageValue, writeStorageValue } from '../lib/local-storage';

const STORAGE_KEY = 'authSettings';

export type AuthProvider = 'github' | 'microsoft' | 'azure-cli' | 'custom';
export type TokenStatus = 'valid' | 'expiring' | 'expired' | 'revoked';

export interface AuthSettingsState {
  identity: {
    provider: AuthProvider;
    displayName: string;
    principal: string;
    tenant: string;
  };
  session: {
    isSignedIn: boolean;
    lastSignInAt: string | null;
    lastSignOutAt: string | null;
  };
  token: {
    label: string;
    status: TokenStatus;
    expiresAt: string;
    autoRefresh: boolean;
    rotateOnReconnect: boolean;
    metadata: string;
  };
  reconnectRequired: boolean;
}

function createDefaultState(): AuthSettingsState {
  return {
    identity: {
      provider: 'github',
      displayName: '',
      principal: '',
      tenant: '',
    },
    session: {
      isSignedIn: false,
      lastSignInAt: null,
      lastSignOutAt: null,
    },
    token: {
      label: '',
      status: 'valid',
      expiresAt: '',
      autoRefresh: true,
      rotateOnReconnect: true,
      metadata: '',
    },
    reconnectRequired: false,
  };
}

function mergeState(saved: unknown): AuthSettingsState {
  const defaults = createDefaultState();
  if (!saved || typeof saved !== 'object') {
    return defaults;
  }

  const value = saved as Partial<AuthSettingsState>;
  return {
    identity: {
      provider: value.identity?.provider ?? defaults.identity.provider,
      displayName: value.identity?.displayName ?? defaults.identity.displayName,
      principal: value.identity?.principal ?? defaults.identity.principal,
      tenant: value.identity?.tenant ?? defaults.identity.tenant,
    },
    session: {
      isSignedIn: value.session?.isSignedIn ?? defaults.session.isSignedIn,
      lastSignInAt: value.session?.lastSignInAt ?? defaults.session.lastSignInAt,
      lastSignOutAt: value.session?.lastSignOutAt ?? defaults.session.lastSignOutAt,
    },
    token: {
      label: value.token?.label ?? defaults.token.label,
      status: value.token?.status ?? defaults.token.status,
      expiresAt: value.token?.expiresAt ?? defaults.token.expiresAt,
      autoRefresh: value.token?.autoRefresh ?? defaults.token.autoRefresh,
      rotateOnReconnect: value.token?.rotateOnReconnect ?? defaults.token.rotateOnReconnect,
      metadata: value.token?.metadata ?? defaults.token.metadata,
    },
    reconnectRequired: value.reconnectRequired ?? defaults.reconnectRequired,
  };
}

export const useAuthSettingsStore = defineStore('authSettings', () => {
  const settings = ref<AuthSettingsState>(createDefaultState());
  const initialized = ref(false);

  watch(
    settings,
    (next) => {
      if (!initialized.value) {
        return;
      }
      writeStorageValue(PREFS_STORE_PATH, STORAGE_KEY, next);
    },
    { deep: true },
  );

  function init() {
    settings.value = mergeState(readStorageValue<AuthSettingsState>(PREFS_STORE_PATH, STORAGE_KEY));
    initialized.value = true;
  }

  function markCredentialChanged() {
    settings.value.reconnectRequired = true;
  }

  function markReconnected() {
    settings.value.reconnectRequired = false;
  }

  function setSignedIn(next: boolean) {
    settings.value.session.isSignedIn = next;
    if (next) {
      settings.value.session.lastSignInAt = new Date().toISOString();
    } else {
      settings.value.session.lastSignOutAt = new Date().toISOString();
    }
  }

  function signOut() {
    settings.value.session.isSignedIn = false;
    settings.value.session.lastSignOutAt = new Date().toISOString();
    settings.value.token.status = 'revoked';
    settings.value.token.metadata = '';
    settings.value.reconnectRequired = true;
  }

  return {
    settings,
    init,
    markCredentialChanged,
    markReconnected,
    setSignedIn,
    signOut,
  };
});
