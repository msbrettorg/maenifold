import { StorageSerializers, useLocalStorage } from '@vueuse/core';
import { defineStore } from 'pinia';
import { PREFS_STORE_PATH } from '../lib/constants';
import { storageKey } from '../lib/storage-key';

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

export const useAuthSettingsStore = defineStore('authSettings', () => {
  const settings = useLocalStorage<AuthSettingsState>(
    storageKey(PREFS_STORE_PATH, STORAGE_KEY),
    createDefaultState(),
    {
      mergeDefaults: true,
      serializer: StorageSerializers.object,
    },
  );

  function init() {}

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
