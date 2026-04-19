<script setup lang="ts">
import { computed } from 'vue';
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from '@/components/ui/breadcrumb';
import { compactProvenanceId, provenanceKindLabel, type ProvenanceKind, type ProvenanceLink } from '../lib/provenance';

const props = defineProps<{
  currentKind: ProvenanceKind;
  currentId?: string | null;
  currentLabel?: string;
  links: ProvenanceLink[];
}>();

const normalizedLinks = computed(() => {
  const seen = new Set<string>();
  const links: ProvenanceLink[] = [];
  for (const link of props.links) {
    const id = link.id.trim();
    if (!id) continue;
    const key = `${link.kind}:${id}`;
    if (seen.has(key)) continue;
    seen.add(key);
    links.push({ ...link, id });
  }
  return links;
});

const currentText = computed(() => {
  const label = props.currentLabel?.trim();
  if (label) {
    return label;
  }

  const id = props.currentId?.trim();
  return id ? compactProvenanceId(id) : 'Current';
});

const hasContent = computed(() => {
  return normalizedLinks.value.length > 0 || Boolean(props.currentLabel?.trim()) || Boolean(props.currentId?.trim());
});
</script>

<template>
  <div v-if="hasContent" class="rounded-[var(--radius-8)] border border-border-subtle bg-bg-surface px-3 py-2">
    <Breadcrumb>
      <BreadcrumbList class="text-[var(--text-body-sm)]">
        <template v-for="(link, index) in normalizedLinks" :key="`${link.kind}:${link.id}`">
          <BreadcrumbSeparator v-if="index > 0" />
          <BreadcrumbItem>
            <BreadcrumbLink
              as="button"
              class="inline-flex items-center gap-1 rounded-[var(--radius-6)] px-1 py-0.5 text-text-secondary hover:text-text-primary"
              @click="$emit('navigate', link)"
            >
              <span class="text-text-tertiary">{{ provenanceKindLabel(link.kind) }}</span>
              <span class="font-[var(--font-code)] text-[var(--text-tiny)]">
                {{ link.label ?? compactProvenanceId(link.id) }}
              </span>
            </BreadcrumbLink>
          </BreadcrumbItem>
        </template>
        <BreadcrumbSeparator v-if="normalizedLinks.length > 0" />
        <BreadcrumbItem>
          <BreadcrumbPage class="inline-flex items-center gap-1 rounded-[var(--radius-6)] px-1 py-0.5 text-text-primary">
            <span>{{ provenanceKindLabel(props.currentKind) }}</span>
            <span class="font-[var(--font-code)] text-[var(--text-tiny)]">{{ currentText }}</span>
          </BreadcrumbPage>
        </BreadcrumbItem>
      </BreadcrumbList>
    </Breadcrumb>
  </div>
</template>
