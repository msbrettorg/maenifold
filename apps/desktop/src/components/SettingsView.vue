<script setup lang="ts">
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import SettingsConnection from './SettingsConnection.vue';
import SettingsMaenifold from './SettingsMaenifold.vue';
import SettingsMcpServers from './SettingsMcpServers.vue';
import SettingsEmbeddings from './SettingsEmbeddings.vue';
import SettingsPreferences from './SettingsPreferences.vue';
import SettingsSchedules from './SettingsSchedules.vue';
import TeamsSettings from './teams/TeamsSettings.vue';
import { ScrollArea } from '@/components/ui/scroll-area';
import { MessageSquare } from 'lucide-vue-next';

const sections = [
  { id: 'connection', label: 'Agents', description: 'Manage ACP agents and their configuration.' },
  { id: 'mcp-servers', label: 'MCP servers', description: 'Configure global MCP server connections.' },
  { id: 'maenifold', label: 'Maenifold', description: 'Configure maenifold runtime behavior and tuning.' },
  { id: 'embeddings', label: 'Embeddings', description: 'Manage embedding models and vector index.' },
  { id: 'schedules', label: 'Schedules', description: 'Manage recurring agent tasks and automations.' },
  { id: 'teams', label: 'Teams', description: 'Configure Teams monitor keywords, senders, and dispatch.', icon: MessageSquare, component: TeamsSettings },
  { id: 'preferences', label: 'Preferences', description: 'Control persisted application defaults.' },
];
</script>

<template>
  <div class="h-full p-6">
    <h2 class="text-lg font-semibold text-text-primary mb-1">Settings</h2>
    <p class="text-sm text-text-secondary mb-6">
      Configure your agent catalog and local application preferences.
    </p>

    <Tabs default-value="connection" orientation="vertical" class="flex gap-6 h-[calc(100%-80px)]">
      <TabsList variant="line" class="flex flex-col gap-1 w-[280px] flex-shrink-0 bg-transparent p-0 rounded-none h-fit" aria-label="Settings sections">
        <TabsTrigger
          v-for="s in sections"
          :key="s.id"
          :value="s.id"
          class="flex flex-col items-start text-left gap-1 rounded border-l-2 border-l-transparent px-3 py-2.5 cursor-pointer transition-colors duration-150 bg-transparent hover:bg-bg-surface-hover data-[state=active]:border-l-brand data-[state=active]:bg-bg-surface-hover h-auto w-full justify-start whitespace-normal"
        >
          <span class="text-sm font-semibold text-text-primary">{{ s.label }}</span>
          <span class="text-xs text-text-secondary leading-snug line-clamp-2">{{ s.description }}</span>
        </TabsTrigger>
      </TabsList>

      <ScrollArea class="flex-1">
        <TabsContent value="connection"><SettingsConnection /></TabsContent>
        <TabsContent value="mcp-servers"><SettingsMcpServers /></TabsContent>
        <TabsContent value="maenifold"><SettingsMaenifold /></TabsContent>
        <TabsContent value="embeddings"><SettingsEmbeddings /></TabsContent>
        <TabsContent value="schedules"><SettingsSchedules /></TabsContent>
        <TabsContent value="teams"><TeamsSettings /></TabsContent>
        <TabsContent value="preferences"><SettingsPreferences /></TabsContent>
      </ScrollArea>
    </Tabs>
  </div>
</template>
