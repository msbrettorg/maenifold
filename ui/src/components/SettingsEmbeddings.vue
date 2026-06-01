<script setup lang="ts">
import { Package } from '@lucide/vue';
</script>

<template>
  <div class="embeddings-shell">
    <div class="placeholder-card">
      <div class="placeholder-icon-wrap" aria-hidden="true">
        <Package :size="26" :stroke-width="1.75" />
      </div>
      <div class="placeholder-copy">
        <span class="placeholder-eyebrow">Shell placeholder</span>
        <h3 class="placeholder-title">Embeddings tab coming in T-413.3</h3>
        <p class="placeholder-description">
          This panel is reserved for model catalog, active-model state, and embedding health
          details.
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.embeddings-shell {
  padding: 1.15rem;
}

.placeholder-card {
  display: flex;
  align-items: flex-start;
  gap: 1rem;
  padding: 1.2rem 1.15rem;
  border: 1px dashed color-mix(in srgb, var(--text-accent) 26%, var(--border-color));
  border-radius: 14px;
  background: color-mix(in srgb, var(--bg-sidebar) 45%, var(--bg-main));
}

.placeholder-icon-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 56px;
  height: 56px;
  border-radius: 16px;
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--text-accent) 25%, var(--border-color));
  box-shadow: 0 8px 22px color-mix(in srgb, var(--text-accent) 12%, transparent);
  flex-shrink: 0;
}

.placeholder-copy {
  min-width: 0;
}

.placeholder-eyebrow {
  display: inline-flex;
  align-items: center;
  margin-bottom: 0.45rem;
  color: var(--text-muted);
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.placeholder-title {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 600;
  color: var(--text-primary);
}

.placeholder-description {
  margin: 0.4rem 0 0;
  color: var(--text-secondary);
  line-height: 1.6;
}

@media (max-width: 640px) {
  .embeddings-shell {
    padding: 1rem;
  }

  .placeholder-card {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
