<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useMcpResources } from '@/composables/useMcpResources';
import { useSessionStore } from '@/stores/session';

const sessionStore = useSessionStore();
const mcpResources = useMcpResources();
const error = ref('');

async function connectToTeamsSession(): Promise<void> {
  if (sessionStore.currentSession) return;

  let agent = 'Codex';
  let cwd = '/Users/brett/maenifold/memory';
  try {
    const resource = await mcpResources.readResource('teams://config');
    if (typeof resource.text === 'string') {
      const config = JSON.parse(resource.text);
      if (typeof config.dispatchAgent === 'string' && config.dispatchAgent) {
        agent = config.dispatchAgent;
      }
      if (typeof config.maenifoldRoot === 'string' && config.maenifoldRoot) {
        cwd = `${config.maenifoldRoot}/memory`;
      }
    }
  } catch { /* use defaults */ }

  try {
    await sessionStore.createSession(agent, cwd);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

onMounted(() => { void connectToTeamsSession(); });
</script>

<template>
  <div v-if="error" class="flex h-full items-center justify-center p-8 text-sm text-text-secondary">
    {{ error }}
  </div>
</template>
