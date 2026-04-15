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
      <span class="graph-legend__swatch graph-legend__swatch--secondary" />
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
  padding: 0.45rem 1rem;
  background: #1c1b1b;
  color: #707070;
  font-size: 0.6875rem;
}

.graph-legend__item {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
}

.graph-legend__swatch {
  width: 8px;
  height: 8px;
  border-radius: 4px;
  flex-shrink: 0;
}

.graph-legend__swatch--primary {
  background: #4d8eff;
}

.graph-legend__swatch--secondary {
  background: #4edea3;
}

.graph-legend__copy {
  color: #707070;
}
</style>
