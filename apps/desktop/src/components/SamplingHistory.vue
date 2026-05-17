<script setup lang="ts">
import { onMounted } from 'vue';
import { Button } from '@/components/ui/button';
import SamplingRequestResponseTable from './SamplingRequestResponseTable.vue';
import { useSamplingHistoryStore } from '../stores/sampling-history';

const store = useSamplingHistoryStore();

onMounted(() => {
  if (typeof store.init === 'function') {
    void store.init();
  }
});

</script>

<template>
  <section class="flex flex-col gap-3">
    <header class="flex flex-wrap items-start justify-between gap-3">
      <div class="flex flex-col gap-1">
        <h2 class="text-lg font-semibold text-text-primary">Sampling history</h2>
        <p class="text-sm text-text-secondary">
          Read-only mirror of recent MCP sampling and GatewayIntent inference requests.
        </p>
      </div>
      <Button type="button" variant="outline" @click="store.refresh">
        Refresh
      </Button>
    </header>

    <div
      v-if="store.entries.length === 0"
      class="rounded-lg border border-border-subtle bg-bg-elevated p-4 text-sm text-text-secondary"
    >
      No sampling history.
    </div>

    <SamplingRequestResponseTable v-else :entries="store.entries" />
  </section>
</template>
