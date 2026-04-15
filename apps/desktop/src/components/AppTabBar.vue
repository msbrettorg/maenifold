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
  gap: 2rem;
  height: 40px;
  padding: 0 1rem;
  background: #131313;
  box-sizing: border-box;
}

.app-tab-bar__wordmark {
  color: #e5e2e1;
  font-family: var(--font-display);
  font-size: var(--text-body-sm);
  font-weight: var(--weight-title);
  letter-spacing: 0.08em;
  text-transform: lowercase;
  white-space: nowrap;
}

.app-tab-bar__tabs {
  display: flex;
  align-items: center;
  height: 100%;
  gap: 1rem;
}

.app-tab-bar__tab {
  position: relative;
  display: inline-flex;
  align-items: center;
  height: 100%;
  padding: 0;
  border: 0;
  background: transparent;
  color: #c2c6d6;
  cursor: pointer;
  font: inherit;
  transition: color 150ms ease;
}

.app-tab-bar__tab:hover {
  color: #e5e2e1;
}

.app-tab-bar__tab::after {
  content: '';
  position: absolute;
  right: 0;
  bottom: 0;
  left: 0;
  height: 2px;
  border-radius: 4px;
  background: transparent;
  transition: background-color 150ms ease;
}

.app-tab-bar__tab--active {
  color: #e5e2e1;
}

.app-tab-bar__tab--active::after {
  background: #4d8eff;
}
</style>
