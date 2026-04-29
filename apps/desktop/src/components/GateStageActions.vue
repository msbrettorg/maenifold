<script setup lang="ts">
import { ref } from 'vue';
import { Button } from '@/components/ui/button';
import { useGateMcp } from '../composables/useGateMcp';
import type { GateTask } from '../lib/gate';
import GateAppendDialog from './GateAppendDialog.vue';
import GateFailDialog from './GateFailDialog.vue';
import GatePassDialog from './GatePassDialog.vue';

const props = defineProps<{ task: GateTask; root: string }>();
const emit = defineEmits<{ completed: [] }>();

const gateMcp = useGateMcp();
const appendOpen = ref(false);
const passOpen = ref(false);
const failOpen = ref(false);
const syncing = ref(false);
const syncError = ref('');

async function syncGate() {
  syncing.value = true;
  syncError.value = '';
  try {
    await gateMcp.sync({ root: props.root, rebuild: true });
    emit('completed');
  } catch (errorValue) {
    syncError.value = errorValue instanceof Error ? errorValue.message : String(errorValue);
  } finally {
    syncing.value = false;
  }
}
</script>

<template>
  <section class="flex flex-col gap-2" aria-label="Gate stage actions">
    <div class="flex flex-wrap gap-2">
      <Button type="button" size="sm" variant="ghost" data-testid="gate-append-trigger" @click="appendOpen = true">
        Append note
      </Button>
      <Button type="button" size="sm" data-testid="gate-pass-trigger" @click="passOpen = true">
        Pass
      </Button>
      <Button type="button" size="sm" variant="destructive" data-testid="gate-fail-trigger" @click="failOpen = true">
        Fail
      </Button>
      <Button type="button" size="sm" variant="ghost" :disabled="syncing" data-testid="gate-sync-trigger" @click="syncGate">
        {{ syncing ? 'Syncing…' : 'Sync' }}
      </Button>
    </div>
    <p v-if="syncError" class="text-sm text-danger">{{ syncError }}</p>
    <GateAppendDialog v-model:open="appendOpen" :root="root" :task="task" @completed="$emit('completed')" />
    <GatePassDialog v-model:open="passOpen" :root="root" :task="task" @completed="$emit('completed')" />
    <GateFailDialog v-model:open="failOpen" :root="root" :task="task" @completed="$emit('completed')" />
  </section>
</template>
