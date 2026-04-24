<script setup lang="ts">
import type { GateSnapshot } from '../lib/gate';
import { Alert, AlertDescription } from './ui/alert';

defineProps<{
  uiError: string;
  hasGateRoot: boolean;
  isLoading: boolean;
  snapshot: GateSnapshot | null;
  selectedGateRoot: string;
}>();
</script>

<template>
  <Alert v-if="uiError" variant="destructive" role="alert">
    <AlertDescription>{{ uiError }}</AlertDescription>
  </Alert>
  <Alert v-else-if="!hasGateRoot" role="status">
    <AlertDescription>No Gate root selected.</AlertDescription>
  </Alert>
  <Alert v-else-if="isLoading && !snapshot" role="status">
    <AlertDescription>Loading gate snapshot…</AlertDescription>
  </Alert>
  <Alert v-else-if="snapshot && !snapshot.exists" role="status">
    <AlertDescription>No .gate/gate.db found in {{ selectedGateRoot }}.</AlertDescription>
  </Alert>
</template>
