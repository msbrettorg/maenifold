<script setup lang="ts">
import { onMounted } from 'vue';
import { storeToRefs } from 'pinia';
import ElicitationModal from './ElicitationModal.vue';
import { useUserPromptStore } from '../stores/user-prompt';

const userPromptStore = useUserPromptStore();
const { open: elicitationOpen } = storeToRefs(userPromptStore);

onMounted(() => {
  userPromptStore.setupIpcListener();
});
</script>

<template>
  <div v-if="elicitationOpen" class="relative z-[1000]">
    <ElicitationModal />
  </div>
</template>
