<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { Brain, GitBranch, Home, MessagesSquare, MessageSquare, Settings, ShieldCheck, Workflow } from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { McpCapabilityUnsupportedError, useMcpResources } from '@/composables/useMcpResources';
import { TAB_ORDER } from '@/composables/useAppShell';
import type { AppShellTab } from '@/stores/app-shell';

interface TeamsStatusPayload {
  unreadCount?: unknown;
  unread_count?: unknown;
  matchCount?: unknown;
}

const TEAMS_STATUS_URI = 'teams://status';
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
  MessagesSquare,
  Workflow,
  Settings,
} as const;

const mcpResources = useMcpResources();
const teamsUnreadCount = ref<number | null>(null);
const dismissedTeamsUnreadCount = ref(0);
let unsubscribeStatusUpdates: (() => void) | null = null;

const visibleTeamsUnreadCount = computed(() => {
  if (teamsUnreadCount.value === null) {
    return null;
  }
  if (props.modelValue === 'teams') {
    return 0;
  }
  return Math.max(teamsUnreadCount.value - dismissedTeamsUnreadCount.value, 0);
});

function parseUnreadCount(text?: string): number {
  if (typeof text !== 'string' || text.length === 0) {
    throw new Error('Teams status resource did not include JSON text.');
  }

  const payload = JSON.parse(text) as TeamsStatusPayload;
  const rawCount = [payload.unreadCount, payload.unread_count, payload.matchCount]
    .find((value) => typeof value === 'number');
  return typeof rawCount === 'number' && Number.isFinite(rawCount) && rawCount > 0 ? rawCount : 0;
}

async function refreshTeamsStatus(): Promise<void> {
  try {
    const resource = await mcpResources.readResource(TEAMS_STATUS_URI);
    teamsUnreadCount.value = parseUnreadCount(resource.text);
  } catch (error) {
    teamsUnreadCount.value = null;
    console.error('[teams] Failed to refresh unread status:', error);
  }
}

function selectTab(tab: AppShellTab) {
  emit('update:modelValue', tab);
}

watch(() => props.modelValue, (value) => {
  if (value === 'teams' && teamsUnreadCount.value !== null) {
    dismissedTeamsUnreadCount.value = teamsUnreadCount.value;
  }
}, { immediate: true });

onMounted(() => {
  void refreshTeamsStatus();
  void mcpResources.subscribeResource(TEAMS_STATUS_URI, () => {
    void refreshTeamsStatus();
  }).then((unsubscribe) => {
    unsubscribeStatusUpdates = unsubscribe;
  }).catch((error) => {
    if (error instanceof McpCapabilityUnsupportedError) {
      return;
    }
    teamsUnreadCount.value = null;
    console.error('[teams] Failed to subscribe to unread status updates:', error);
    toast.error('Unable to subscribe to Teams unread status updates.');
  });
});

onBeforeUnmount(() => {
  unsubscribeStatusUpdates?.();
  unsubscribeStatusUpdates = null;
});
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
          <Badge
            v-if="tab.value === 'teams' && visibleTeamsUnreadCount !== null && visibleTeamsUnreadCount > 0"
            variant="destructive"
            class="h-4 min-w-4 rounded-full px-1 text-[10px] leading-none"
          >
            {{ visibleTeamsUnreadCount }}
          </Badge>
        </TabsTrigger>
      </TabsList>
    </Tabs>
  </nav>
</template>
