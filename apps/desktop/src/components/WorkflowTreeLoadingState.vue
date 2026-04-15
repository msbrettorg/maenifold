<script setup lang="ts">
const skeletonItems = [1, 2, 3] as const;
</script>

<template>
  <div class="tree-loading" aria-busy="true" aria-label="Loading workflow tree">
    <div class="loading-header">
      <div class="loading-pulse loading-badge" />
      <div class="loading-text-block">
        <div class="loading-pulse loading-line-wide" />
        <div class="loading-pulse loading-line-narrow" />
      </div>
    </div>
    <div class="loading-progress" />
    <div class="loading-steps">
      <div
        v-for="i in skeletonItems"
        :key="i"
        class="loading-step-row"
      >
        <div class="loading-pulse loading-step-icon" />
        <div class="loading-pulse loading-step-name" :style="{ width: `${55 + i * 13}%` }" />
      </div>
    </div>
    <div class="loading-connector-placeholder" />
    <div class="loading-children">
      <div
        v-for="i in skeletonItems"
        :key="i"
        class="skeleton-child"
      >
        <div class="loading-pulse skeleton-child-icon" />
        <div class="loading-pulse skeleton-child-label" :style="{ width: `${40 + i * 10}%` }" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.tree-loading {
  width: 320px;
  background: color-mix(in srgb, var(--muted) 88%, var(--background));
  border-radius: var(--radius);
  border-left: 3px solid color-mix(in srgb, var(--border) 32%, var(--muted));
  padding: 0.875rem 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}

.loading-pulse {
  border-radius: 4px;
  background: linear-gradient(
    90deg,
    color-mix(in srgb, var(--muted) 82%, var(--background)) 25%,
    color-mix(in srgb, var(--muted) 68%, var(--background)) 50%,
    color-mix(in srgb, var(--muted) 82%, var(--background)) 75%
  );
  background-size: 200% 100%;
  animation: shimmer 1.6s infinite;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

.loading-header {
  display: flex;
  align-items: center;
  gap: 0.625rem;
}

.loading-badge {
  width: 28px;
  height: 28px;
  border-radius: var(--radius);
  flex-shrink: 0;
}

.loading-text-block {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 0.35rem;
}

.loading-line-wide {
  height: 12px;
  width: 70%;
}

.loading-line-narrow {
  height: 9px;
  width: 40%;
}

.loading-progress {
  height: 4px;
  border-radius: 2px;
  background: linear-gradient(
    90deg,
    color-mix(in srgb, var(--muted) 82%, var(--background)) 25%,
    color-mix(in srgb, var(--muted) 68%, var(--background)) 50%,
    color-mix(in srgb, var(--muted) 82%, var(--background)) 75%
  );
  background-size: 200% 100%;
  animation: shimmer 1.6s infinite;
}

.loading-steps {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.loading-step-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.loading-step-icon {
  width: 12px;
  height: 12px;
  border-radius: var(--radius);
  flex-shrink: 0;
}

.loading-step-name {
  height: 10px;
}

.loading-connector-placeholder {
  height: 40px;
  display: flex;
  justify-content: center;
  align-items: center;
}

.loading-connector-placeholder::after {
  content: '';
  display: block;
  width: 2px;
  height: 100%;
  background: linear-gradient(
    180deg,
    color-mix(in srgb, var(--border) 72%, var(--muted)),
    transparent
  );
}

.loading-children {
  display: flex;
  gap: 0.5rem;
}

.skeleton-child {
  flex: 1;
  min-width: 80px;
  background: color-mix(in srgb, var(--muted) 92%, var(--background));
  border-radius: var(--radius);
  border-left: 3px solid color-mix(in srgb, var(--border) 32%, var(--muted));
  padding: 0.6rem;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.skeleton-child-icon {
  width: 24px;
  height: 24px;
  border-radius: var(--radius);
}

.skeleton-child-label {
  height: 9px;
  border-radius: 3px;
}
</style>
