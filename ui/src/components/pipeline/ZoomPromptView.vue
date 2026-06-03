<script setup lang="ts">
// SemanticZoom content renderer: the assembled prompt, split (best-effort) into
// predecessor / ambient / instruction sections. When no recognizable section
// headers parse, the whole prompt is shown as a single honest block with a note.
import { computed } from 'vue';
import { ArrowDownToLine, Info, ListTree, Sparkles } from '@lucide/vue';
import { parseAssembledPrompt, type PromptSectionKind } from './zoomLogic';

const props = defineProps<{ assembledPrompt: string }>();

const sections = computed(() => parseAssembledPrompt(props.assembledPrompt));

interface SectionView {
  kind: PromptSectionKind;
  title: string;
  body: string;
}

const SECTION_META: Record<PromptSectionKind, { title: string }> = {
  predecessor: { title: 'Predecessor' },
  ambient: { title: 'Ambient context' },
  instruction: { title: 'Instruction' },
};

const ICONS = { predecessor: ArrowDownToLine, ambient: ListTree, instruction: Sparkles } as const;

const visibleSections = computed<SectionView[]>(() => {
  const parsed = sections.value;
  const order: PromptSectionKind[] = ['predecessor', 'ambient', 'instruction'];
  return order
    .filter((kind) => parsed[kind] !== undefined)
    .map((kind) => ({ kind, title: SECTION_META[kind].title, body: parsed[kind] as string }));
});
</script>

<template>
  <div class="zprompt">
    <template v-if="sections.parsed">
      <section v-for="sec in visibleSections" :key="sec.kind" class="zprompt-section">
        <header class="zprompt-head">
          <component :is="ICONS[sec.kind]" :size="14" :stroke-width="2" />
          <span>{{ sec.title }}</span>
        </header>
        <pre class="zprompt-body">{{ sec.body }}</pre>
      </section>
    </template>
    <template v-else>
      <p class="zprompt-note">
        <Info :size="13" :stroke-width="2" />
        No recognizable section delimiters — showing the full assembled prompt.
      </p>
      <pre class="zprompt-body">{{ sections.whole }}</pre>
    </template>
  </div>
</template>

<style scoped>
.zprompt {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.zprompt-section {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.zprompt-head {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.74rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-accent);
}

.zprompt-note {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  margin: 0;
  font-size: 0.76rem;
  color: var(--text-muted);
}

.zprompt-body {
  margin: 0;
  padding: 0.65rem 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-main) 92%, var(--bg-sidebar));
  color: var(--text-primary);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.78rem;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
