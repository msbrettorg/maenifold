<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue';
import { RefreshCw, ScrollText } from 'lucide-vue-next';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import SamplingRequestResponseTable from './SamplingRequestResponseTable.vue';
import { useSamplingHistoryStore } from '../stores/sampling-history';

const samplingStore = useSamplingHistoryStore();

const totalEntries = computed(() => samplingStore.entries.length);
const activeEntries = computed(() => samplingStore.entries.filter((entry) => (
  entry.status === 'dispatching' || entry.status === 'streaming' || entry.status === 'budgetExceeded'
)).length);
const failedEntries = computed(() => samplingStore.entries.filter((entry) => entry.status === 'failed').length);

onMounted(() => {
  void samplingStore.start();
});

onBeforeUnmount(() => {
  void samplingStore.stop();
});
</script>

<template>
  <section class="flex h-full min-h-0 flex-col overflow-hidden bg-bg-app">
    <header class="border-b border-border-subtle bg-bg-elevated px-6 py-5">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div class="min-w-0">
          <div class="flex items-center gap-2">
            <ScrollText class="size-5 text-brand" aria-hidden="true" />
            <h2 class="text-xl font-semibold text-text-primary">Logs</h2>
          </div>
          <p class="mt-1 max-w-3xl text-sm text-text-secondary">
            Unified visibility for GatewayIntent MCP client sampling and direct inferencing. No ACP/Jockey here.
          </p>
        </div>
        <Button type="button" variant="outline" class="gap-2" @click="samplingStore.refresh">
          <RefreshCw class="size-4" aria-hidden="true" />
          Refresh
        </Button>
      </div>
      <div class="mt-4 flex flex-wrap gap-2">
        <Badge variant="secondary">{{ totalEntries }} inference events</Badge>
        <Badge variant="secondary" class="bg-warning/10 text-warning">{{ activeEntries }} active</Badge>
        <Badge variant="secondary" class="bg-destructive/10 text-destructive">{{ failedEntries }} failed</Badge>
      </div>
    </header>

    <div class="min-h-0 flex-1 overflow-auto p-6">
      <SamplingRequestResponseTable
        :entries="samplingStore.entries"
        @stop="(samplingRequestId) => samplingStore.cancelSamplingSession(samplingRequestId)"
      />
    </div>
  </section>
</template>