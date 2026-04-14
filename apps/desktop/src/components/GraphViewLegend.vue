<script setup lang="ts">
defineProps<{
  viewLevel: 'communities' | 'concepts';
  activeCommunityAnchor: string | null;
}>();
</script>

<template>
  <div class="graph-legend" role="note" aria-label="Knowledge graph legend">
    <span class="graph-legend__item">
      <span class="graph-legend__swatch graph-legend__swatch--primary" />
      {{ viewLevel === 'communities' ? 'Community clusters' : 'Anchor concept' }}
    </span>
    <span class="graph-legend__item">
      <span class="graph-legend__swatch graph-legend__swatch--muted" />
      {{ viewLevel === 'communities' ? 'Linked communities' : 'Related concepts' }}
    </span>
    <span class="graph-legend__copy">
      {{ viewLevel === 'communities' ? 'Colors identify connected knowledge clusters.' : `${activeCommunityAnchor ?? 'Selected'} relationships are highlighted.` }}
    </span>
  </div>
</template>

<style scoped>
.graph-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  padding: 0.55rem 1rem;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
  background: color-mix(in srgb, var(--card) 78%, var(--background));
  color: var(--muted-foreground);
  font-size: var(--text-tiny);
}

.graph-legend__item {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.graph-legend__swatch {
  width: 0.8rem;
  height: 0.8rem;
  border-radius: 999px;
}

.graph-legend__swatch--primary {
  background: #4d8eff;
}

.graph-legend__swatch--muted {
  background: color-mix(in srgb, #4d8eff 60%, #1f2937);
}

.graph-legend__copy {
  color: var(--muted-foreground);
}
</style>
