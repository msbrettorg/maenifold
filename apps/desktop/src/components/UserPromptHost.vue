<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { storeToRefs } from 'pinia';
import ElicitationModal from './ElicitationModal.vue';
import SamplingApprovalModal from './SamplingApprovalModal.vue';
import { useUserPromptStore } from '../stores/user-prompt';
import { useSamplingStore, type SamplingDecision } from '../stores/sampling';

const userPromptStore = useUserPromptStore();
const samplingStore = useSamplingStore();
const { open: elicitationOpen } = storeToRefs(userPromptStore);
const { current: samplingCurrent } = storeToRefs(samplingStore);
const samplingOpen = computed(() => samplingCurrent.value !== null);

async function onSamplingResolve(decision: SamplingDecision): Promise<void> {
  if (samplingCurrent.value) {
    await samplingStore.resolve(samplingCurrent.value.id, decision);
  }
}

onMounted(() => {
  userPromptStore.setupIpcListener();
});
</script>

<template>
  <div v-if="elicitationOpen" class="relative z-[1000]">
    <ElicitationModal />
  </div>
  <div v-if="samplingOpen && samplingCurrent" class="relative z-[1000]">
    <SamplingApprovalModal :request="samplingCurrent" @resolve="onSamplingResolve" />
  </div>
</template>
