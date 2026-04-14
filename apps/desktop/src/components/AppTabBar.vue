<script setup lang="ts">
const props = defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const tabs = [
  { value: 'agents', label: 'Agents' },
  { value: 'pipelines', label: 'Pipelines' },
  { value: 'gates', label: 'Gates' },
  { value: 'workflows', label: 'Workflows' },
  { value: 'graph', label: 'Graph' },
] as const;

function selectTab(tab: string) {
  emit('update:modelValue', tab);
}
</script>

<template>
  <nav class="app-tab-bar" aria-label="Primary navigation">
    <div class="app-tab-bar__wordmark">maenifold</div>

    <div class="app-tab-bar__tabs">
      <button
        v-for="tab in tabs"
        :key="tab.value"
        type="button"
        class="app-tab-bar__tab"
        :class="{ 'app-tab-bar__tab--active': props.modelValue === tab.value }"
        @click="selectTab(tab.value)"
      >
        {{ tab.label }}
      </button>
    </div>
  </nav>
</template>

<style scoped>
.app-tab-bar {
  position: absolute;
  inset: 0 0 auto 0;
  z-index: 20;
  display: flex;
  align-items: center;
  gap: 1.5rem;
  height: 40px;
  padding: 0 1rem;
  background: #201f1f;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  box-sizing: border-box;
}

.app-tab-bar__wordmark {
  color: #f4f4f4;
  font-family: var(--font-display);
  font-size: var(--text-body);
  font-weight: var(--weight-title);
  letter-spacing: 0.08em;
  text-transform: lowercase;
  white-space: nowrap;
}

.app-tab-bar__tabs {
  display: flex;
  align-items: center;
  height: 100%;
  gap: 0.25rem;
}

.app-tab-bar__tab {
  display: inline-flex;
  align-items: center;
  height: 100%;
  padding: 0 0.75rem;
  border: 0;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: #999;
  cursor: pointer;
  font: inherit;
  transition: color 150ms ease, border-color 150ms ease;
}

.app-tab-bar__tab:hover {
  color: #c7c7c7;
}

.app-tab-bar__tab--active {
  color: #4da6ff;
  border-bottom-color: #4da6ff;
}
</style>
