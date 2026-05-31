<script setup lang="ts">
// Generic placeholder for tabs that are infrastructure-only this sprint
// (Gates, Pipelines, Workflows). Themed to match the Home landing page so the
// shell feels coherent even before these surfaces are wired up.
import type { Component } from 'vue';

defineProps<{
  icon: Component;
  title: string;
  description: string;
}>();
</script>

<template>
  <div class="placeholder">
    <div class="placeholder-card">
      <div class="placeholder-icon-wrap">
        <component :is="icon" :size="40" :stroke-width="1.25" />
      </div>
      <h1 class="placeholder-title">{{ title }}</h1>
      <p class="placeholder-desc">{{ description }}</p>
      <span class="placeholder-badge">Coming soon</span>
    </div>
  </div>
</template>

<style scoped>
.placeholder {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  background:
    radial-gradient(80% 60% at 50% 0%, color-mix(in srgb, var(--text-accent) 7%, transparent) 0%, transparent 70%),
    var(--bg-main);
}

.placeholder-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 0.85rem;
  max-width: 420px;
}

.placeholder-icon-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 84px;
  height: 84px;
  border-radius: 20px;
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--text-accent) 25%, var(--border-color));
  box-shadow: 0 10px 30px color-mix(in srgb, var(--text-accent) 18%, transparent);
}

.placeholder-title {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 600;
  letter-spacing: -0.02em;
  color: var(--text-primary);
}

.placeholder-desc {
  margin: 0;
  font-size: 0.95rem;
  line-height: 1.5;
  color: var(--text-secondary);
}

.placeholder-badge {
  margin-top: 0.25rem;
  padding: 0.25rem 0.7rem;
  border-radius: 999px;
  border: 1px solid var(--border-color);
  background: var(--bg-sidebar);
  font-size: 0.7rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--text-muted);
}
</style>
