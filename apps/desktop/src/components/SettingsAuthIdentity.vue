<script setup lang="ts">
import { computed } from 'vue';
import { storeToRefs } from 'pinia';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useAuthSettingsStore, type AuthProvider, type TokenStatus } from '../stores/auth-settings';

const authSettingsStore = useAuthSettingsStore();
const { settings } = storeToRefs(authSettingsStore);

const providerOptions: Array<{ value: AuthProvider; label: string }> = [
  { value: 'github', label: 'GitHub' },
  { value: 'microsoft', label: 'Microsoft account' },
  { value: 'azure-cli', label: 'Azure CLI' },
  { value: 'custom', label: 'Custom provider' },
];

const tokenStatusOptions: Array<{ value: TokenStatus; label: string }> = [
  { value: 'valid', label: 'Valid' },
  { value: 'expiring', label: 'Expiring soon' },
  { value: 'expired', label: 'Expired' },
  { value: 'revoked', label: 'Revoked' },
];

const loginBadgeVariant = computed(() => (settings.value.session.isSignedIn ? 'secondary' : 'outline'));

const loginBadgeText = computed(() => (settings.value.session.isSignedIn ? 'Signed in' : 'Signed out'));

const lastAuthEventText = computed(() => {
  const timestamp = settings.value.session.isSignedIn
    ? settings.value.session.lastSignInAt
    : settings.value.session.lastSignOutAt;

  if (!timestamp) {
    return 'No auth activity recorded yet.';
  }

  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) {
    return 'No auth activity recorded yet.';
  }

  return settings.value.session.isSignedIn
    ? `Signed in ${date.toLocaleString()}.`
    : `Signed out ${date.toLocaleString()}.`;
});

function updateProvider(value: string) {
  settings.value.identity.provider = value as AuthProvider;
  authSettingsStore.markCredentialChanged();
}

function updateDisplayName(value: string) {
  settings.value.identity.displayName = value;
}

function updatePrincipal(value: string) {
  settings.value.identity.principal = value;
  authSettingsStore.markCredentialChanged();
}

function updateTenant(value: string) {
  settings.value.identity.tenant = value;
}

function updateSignedIn(value: boolean) {
  authSettingsStore.setSignedIn(value);
}

function updateTokenLabel(value: string) {
  settings.value.token.label = value;
  authSettingsStore.markCredentialChanged();
}

function updateTokenStatus(value: string) {
  settings.value.token.status = value as TokenStatus;
  authSettingsStore.markCredentialChanged();
}

function updateTokenExpiresAt(value: string) {
  settings.value.token.expiresAt = value;
  authSettingsStore.markCredentialChanged();
}

function updateTokenMetadata(value: string) {
  settings.value.token.metadata = value;
}

function updateAutoRefresh(value: boolean) {
  settings.value.token.autoRefresh = value;
}

function updateRotateOnReconnect(value: boolean) {
  settings.value.token.rotateOnReconnect = value;
}

function handleSignOut() {
  authSettingsStore.signOut();
}
</script>

<template>
  <section class="flex flex-col gap-4" role="form" aria-label="Authentication and identity settings">
    <Card class="border-border-subtle bg-bg-sidebar shadow-none">
      <CardHeader class="gap-2">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <div>
            <CardTitle class="text-base text-text-primary">Authentication and identity</CardTitle>
            <CardDescription class="text-text-secondary">
              Manage persisted identity metadata and token behavior.
            </CardDescription>
          </div>
          <Badge :variant="loginBadgeVariant">{{ loginBadgeText }}</Badge>
        </div>
        <p class="text-[var(--text-caption)] text-text-secondary">{{ lastAuthEventText }}</p>
        <p
          v-if="settings.reconnectRequired"
          class="rounded-md border border-border-subtle bg-bg-surface px-3 py-2 text-[var(--text-caption)] text-text-secondary"
        >
          Credential changes are saved now and apply after the next session or reconnect.
        </p>
      </CardHeader>
    </Card>

    <Tabs default-value="identity" class="w-full">
      <TabsList variant="line" class="w-full justify-start gap-2">
        <TabsTrigger value="identity">Identity</TabsTrigger>
        <TabsTrigger value="token">Token</TabsTrigger>
        <TabsTrigger value="session">Session</TabsTrigger>
      </TabsList>

      <TabsContent value="identity" class="mt-3">
        <Card class="border-border-subtle bg-bg-sidebar shadow-none">
          <CardContent class="grid gap-4 p-4 sm:grid-cols-2">
            <div class="grid gap-2">
              <Label for="auth-provider">Provider</Label>
              <Select :model-value="settings.identity.provider" @update:model-value="updateProvider(String($event))">
                <SelectTrigger id="auth-provider" class="w-full border-border-subtle bg-bg-surface text-text-primary">
                  <SelectValue placeholder="Select provider" />
                </SelectTrigger>
                <SelectContent class="border-border-subtle bg-bg-surface text-text-primary">
                  <SelectItem v-for="provider in providerOptions" :key="provider.value" :value="provider.value">
                    {{ provider.label }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div class="grid gap-2">
              <Label for="auth-display-name">Display name</Label>
              <Input
                id="auth-display-name"
                class="border-border-subtle bg-bg-surface text-text-primary placeholder:text-text-secondary"
                :model-value="settings.identity.displayName"
                placeholder="Agent operator"
                @update:model-value="updateDisplayName(String($event))"
              />
            </div>

            <div class="grid gap-2 sm:col-span-2">
              <Label for="auth-principal">Login principal</Label>
              <Input
                id="auth-principal"
                class="border-border-subtle bg-bg-surface text-text-primary placeholder:text-text-secondary"
                :model-value="settings.identity.principal"
                placeholder="name@contoso.com"
                @update:model-value="updatePrincipal(String($event))"
              />
            </div>

            <div class="grid gap-2 sm:col-span-2">
              <Label for="auth-tenant">Tenant / org</Label>
              <Input
                id="auth-tenant"
                class="border-border-subtle bg-bg-surface text-text-primary placeholder:text-text-secondary"
                :model-value="settings.identity.tenant"
                placeholder="contoso"
                @update:model-value="updateTenant(String($event))"
              />
            </div>
          </CardContent>
        </Card>
      </TabsContent>

      <TabsContent value="token" class="mt-3">
        <Card class="border-border-subtle bg-bg-sidebar shadow-none">
          <CardContent class="grid gap-4 p-4 sm:grid-cols-2">
            <div class="grid gap-2 sm:col-span-2">
              <Label for="auth-token-label">Token label</Label>
              <Input
                id="auth-token-label"
                class="border-border-subtle bg-bg-surface text-text-primary placeholder:text-text-secondary"
                :model-value="settings.token.label"
                placeholder="Primary workstation token"
                @update:model-value="updateTokenLabel(String($event))"
              />
            </div>

            <div class="grid gap-2">
              <Label for="auth-token-status">Token status</Label>
              <Select :model-value="settings.token.status" @update:model-value="updateTokenStatus(String($event))">
                <SelectTrigger id="auth-token-status" class="w-full border-border-subtle bg-bg-surface text-text-primary">
                  <SelectValue placeholder="Select status" />
                </SelectTrigger>
                <SelectContent class="border-border-subtle bg-bg-surface text-text-primary">
                  <SelectItem v-for="status in tokenStatusOptions" :key="status.value" :value="status.value">
                    {{ status.label }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div class="grid gap-2">
              <Label for="auth-token-expiry">Expires at</Label>
              <Input
                id="auth-token-expiry"
                type="datetime-local"
                class="border-border-subtle bg-bg-surface text-text-primary"
                :model-value="settings.token.expiresAt"
                @update:model-value="updateTokenExpiresAt(String($event))"
              />
            </div>

            <div class="grid gap-2 sm:col-span-2">
              <Label for="auth-token-metadata">Token metadata</Label>
              <Input
                id="auth-token-metadata"
                class="border-border-subtle bg-bg-surface text-text-primary placeholder:text-text-secondary"
                :model-value="settings.token.metadata"
                placeholder="Scope, audience, notes"
                @update:model-value="updateTokenMetadata(String($event))"
              />
            </div>

            <div class="flex items-center justify-between gap-2 rounded-md border border-border-subtle bg-bg-surface px-3 py-2 sm:col-span-2">
              <Label for="auth-auto-refresh">Auto refresh metadata</Label>
              <Switch
                id="auth-auto-refresh"
                :model-value="settings.token.autoRefresh"
                @update:model-value="updateAutoRefresh(Boolean($event))"
              />
            </div>

            <div class="flex items-center justify-between gap-2 rounded-md border border-border-subtle bg-bg-surface px-3 py-2 sm:col-span-2">
              <Label for="auth-rotate-reconnect">Rotate credential on reconnect</Label>
              <Switch
                id="auth-rotate-reconnect"
                :model-value="settings.token.rotateOnReconnect"
                @update:model-value="updateRotateOnReconnect(Boolean($event))"
              />
            </div>
          </CardContent>
        </Card>
      </TabsContent>

      <TabsContent value="session" class="mt-3">
        <Card class="border-border-subtle bg-bg-sidebar shadow-none">
          <CardContent class="flex flex-col gap-4 p-4">
            <div class="flex items-center justify-between gap-2 rounded-md border border-border-subtle bg-bg-surface px-3 py-2">
              <Label for="auth-signed-in-toggle">Signed in state</Label>
              <Switch
                id="auth-signed-in-toggle"
                :model-value="settings.session.isSignedIn"
                @update:model-value="updateSignedIn(Boolean($event))"
              />
            </div>

            <div class="flex flex-wrap gap-2">
              <Button variant="destructive" @click="handleSignOut">Sign out</Button>
              <Button
                variant="outline"
                :disabled="!settings.reconnectRequired"
                @click="authSettingsStore.markReconnected"
              >
                Mark reconnected
              </Button>
            </div>
          </CardContent>
        </Card>
      </TabsContent>
    </Tabs>
  </section>
</template>
