<script setup lang="ts">
import { Button } from '@/components/ui/button';
import SchemaForm from './SchemaForm.vue';
import type { ServerHealthStatus } from '../lib/types';
import type { Component } from 'vue';

interface DiscoveryToolLike {
  displayName?: string;
  server?: string;
  error?: string | null;
}

interface AuthAction {
  label: string;
}

interface PinAdapter {
  isToolPinned: (name: string) => boolean;
  togglePinnedTool: (name: string) => void;
}

const props = defineProps<{
  toolName: string;
  selectedDiscoveryTool: DiscoveryToolLike | null | undefined;
  selectedToolDescription: string | null;
  selectedToolHealth: ServerHealthStatus | null | undefined;
  selectedToolAuthAction: AuthAction | null | undefined;
  selectedToolSchema: unknown;
  toolArgs: Record<string, unknown>;
  requiredArgNames: string[];
  canSubmit: boolean;
  panelClass: string;
  splitRowClass: string;
  flexRowClass: string;
  tagBaseClass: string;
  healthMeta: Record<ServerHealthStatus, { icon: Component; label: string }>;
  tagToneClass: (h: ServerHealthStatus) => string;
  pinAdapter: PinAdapter;
}>();

const emit = defineEmits<{
  clearSelection: [];
  submit: [];
  argsChange: [value: Record<string, unknown>];
  argsValidation: [state: { valid: boolean }];
}>();
</script>

<template>
  <section :class="panelClass">
    <div :class="splitRowClass">
      <div>
        <strong>{{ selectedDiscoveryTool?.displayName ?? toolName.split('.').pop() }}</strong>
        <p class="m-0 text-text-secondary text-sm">{{ selectedToolDescription || 'No description.' }}</p>
      </div>
      <div :class="flexRowClass">
        <span v-if="selectedDiscoveryTool?.server" :class="[tagBaseClass, 'bg-text-primary/10']">{{ selectedDiscoveryTool.server }}</span>
        <span v-if="selectedToolHealth" :class="[tagBaseClass, tagToneClass(selectedToolHealth)]">{{ healthMeta[selectedToolHealth].label }}</span>
        <Button variant="ghost" size="xs" @click="pinAdapter.togglePinnedTool(toolName)">{{ pinAdapter.isToolPinned(toolName) ? 'Unpin' : 'Pin' }}</Button>
        <Button variant="ghost" size="xs" @click="emit('clearSelection')">Clear</Button>
      </div>
    </div>
    <p
      v-if="selectedDiscoveryTool?.error"
      class="m-0 text-sm"
      :class="selectedToolHealth === 'unavailable' ? 'text-status-abandoned' : 'text-text-secondary'"
    >{{ selectedDiscoveryTool.error }}</p>
    <div v-if="selectedToolHealth === 'auth_required' && selectedToolAuthAction" :class="panelClass">
      <p class="m-0 text-text-secondary text-sm">Complete auth setup before adding this tool.</p>
      <Button data-testid="tool-auth-setup-confirm" @click="emit('submit')">{{ selectedToolAuthAction.label }}</Button>
    </div>
    <template v-else>
      <div class="grid gap-1.5">
        <span>Required args</span>
        <div :class="flexRowClass">
          <span v-for="requiredName in requiredArgNames" :key="requiredName" :class="[tagBaseClass, 'bg-bg-app/85']">{{ requiredName }}</span>
          <span v-if="requiredArgNames.length === 0" class="text-text-secondary text-sm m-0">No required args</span>
        </div>
      </div>
      <SchemaForm
        :schema="selectedToolSchema"
        :data="toolArgs"
        :aria-label="`${selectedDiscoveryTool?.displayName ?? toolName.split('.').pop() ?? 'Tool'} arguments`"
        @change="(value: Record<string, unknown>) => emit('argsChange', value)"
        @validation="(state: { valid: boolean }) => emit('argsValidation', state)"
      />
      <div v-if="canSubmit" :class="splitRowClass">
        <Button data-testid="tool-palette-add-tool" @click="emit('submit')">Add Tool</Button>
      </div>
    </template>
  </section>
</template>
