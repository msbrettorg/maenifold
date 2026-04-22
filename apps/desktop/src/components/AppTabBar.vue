<script setup lang="ts">
import { Brain, GitBranch, Home, MessageSquare, Settings, ShieldCheck, Workflow } from 'lucide-vue-next';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { TAB_ORDER } from '@/composables/useAppShell';
import type { AppShellTab } from '@/stores/app-shell';

const props = defineProps<{
  modelValue: AppShellTab;
}>();

const emit = defineEmits<{
  'update:modelValue': [value: AppShellTab];
}>();

const ICONS = {
  Home,
  MessageSquare,
  GitBranch,
  ShieldCheck,
  Brain,
  Workflow,
  Settings,
} as const;

function selectTab(tab: AppShellTab) {
  emit('update:modelValue', tab);
}
</script>

<template>
  <nav
    aria-label="Primary navigation"
    class="flex h-14 shrink-0 items-center border-b border-border-subtle bg-bg-app px-4"
  >
    <div class="mr-6 text-sm font-semibold tracking-[0.08em] text-text-primary lowercase">maenifold</div>
    <Tabs :model-value="props.modelValue" class="w-full" @update:model-value="selectTab($event as AppShellTab)">
      <TabsList variant="line" class="h-14 gap-1 bg-transparent p-0">
        <TabsTrigger
          v-for="tab in TAB_ORDER"
          :key="tab.value"
          :value="tab.value"
          class="h-14 gap-2 rounded-none px-3 text-text-secondary hover:text-text-primary data-[state=active]:text-brand! data-[state=active]:hover:text-brand!"
        >
          <component :is="ICONS[tab.icon]" class="size-4" />
          <span>{{ tab.label }}</span>
        </TabsTrigger>
      </TabsList>
    </Tabs>
  </nav>
</template>
