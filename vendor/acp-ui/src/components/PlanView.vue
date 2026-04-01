<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { PlanEntry } from '../lib/types';

const props = defineProps<{
  entries: PlanEntry[];
}>();

const isExpanded = ref(true);

const entryCount = computed(() => props.entries.length);

const completedCount = computed(
  () => props.entries.filter((entry) => entry.status === 'completed').length
);

const statusLabelMap: Record<PlanEntry['status'], string> = {
  pending: 'Pending',
  in_progress: 'In Progress',
  completed: 'Completed',
};

const statusIconMap: Record<PlanEntry['status'], string> = {
  pending: '○',
  in_progress: '◐',
  completed: '✓',
};

watch(
  () => props.entries,
  () => {
    isExpanded.value = true;
  },
  { deep: true }
);

function toggleExpanded(): void {
  isExpanded.value = !isExpanded.value;
}
</script>

<template>
  <section class="plan-view">
    <button
      type="button"
      class="plan-toggle"
      :aria-expanded="isExpanded"
      @click="toggleExpanded"
    >
      <div class="plan-heading">
        <span class="plan-icon">📐</span>
        <span class="plan-title">Agent Plan</span>
        <span class="plan-summary">
          {{ completedCount }}/{{ entryCount }} completed
        </span>
      </div>
      <span class="plan-chevron">{{ isExpanded ? '▲' : '▼' }}</span>
    </button>

    <div v-if="isExpanded" class="plan-body">
      <ol class="plan-list">
        <li
          v-for="(entry, index) in entries"
          :key="`${index}-${entry.content}`"
          class="plan-entry"
        >
          <span
            :class="['plan-status', `status-${entry.status}`]"
            :title="statusLabelMap[entry.status]"
            :aria-label="statusLabelMap[entry.status]"
          >
            {{ statusIconMap[entry.status] }}
          </span>
          <div class="plan-entry-content">
            <div class="plan-entry-header">
              <span :class="['priority-badge', `priority-${entry.priority}`]">
                {{ entry.priority }}
              </span>
              <span class="plan-entry-status-text">
                {{ statusLabelMap[entry.status] }}
              </span>
            </div>
            <p class="plan-entry-text">{{ entry.content }}</p>
          </div>
        </li>
      </ol>
    </div>
  </section>
</template>

<style scoped>
.plan-view {
  margin-bottom: 1rem;
  border: 1px solid var(--border-color, #e0e0e0);
  border-radius: 10px;
  background: var(--bg-assistant, #f5f5f5);
  overflow: hidden;
}

.plan-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  width: 100%;
  padding: 0.875rem 1rem;
  border: none;
  background: transparent;
  color: var(--text-primary, #111827);
  cursor: pointer;
  text-align: left;
}

.plan-toggle:hover {
  background: var(--bg-hover, rgba(0, 0, 0, 0.03));
}

.plan-heading {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  min-width: 0;
}

.plan-icon {
  flex-shrink: 0;
}

.plan-title {
  font-weight: 600;
}

.plan-summary,
.plan-chevron,
.plan-entry-status-text {
  color: var(--text-muted, #6b7280);
  font-size: 0.85rem;
}

.plan-body {
  border-top: 1px solid var(--border-color, #e0e0e0);
  padding: 0.75rem 1rem 1rem;
}

.plan-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.plan-entry {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
}

.plan-status {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  margin-top: 0.125rem;
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.05);
  font-size: 0.95rem;
  font-weight: 700;
  flex-shrink: 0;
}

.status-pending {
  color: #9ca3af;
}

.status-in_progress {
  color: #2563eb;
  animation: pulse 1.2s ease-in-out infinite;
}

.status-completed {
  color: #059669;
}

.plan-entry-content {
  min-width: 0;
  flex: 1;
}

.plan-entry-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.25rem;
}

.priority-badge {
  display: inline-flex;
  align-items: center;
  padding: 0.125rem 0.5rem;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: capitalize;
}

.priority-high {
  background: rgba(239, 68, 68, 0.12);
  color: #b91c1c;
}

.priority-medium {
  background: rgba(245, 158, 11, 0.16);
  color: #b45309;
}

.priority-low {
  background: rgba(107, 114, 128, 0.14);
  color: #4b5563;
}

.plan-entry-text {
  margin: 0;
  color: var(--text-primary, #111827);
  line-height: 1.45;
  white-space: pre-wrap;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }

  50% {
    opacity: 0.45;
  }
}
</style>
