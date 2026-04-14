<script setup lang="ts">
import {
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuSub,
  ContextMenuSubContent,
  ContextMenuSubTrigger,
} from '@/components/ui/context-menu'

interface ToolMenuItem {
  name: string
  displayName: string
  description?: string | null
}

interface ToolMenuGroup {
  server: string
  tools: ToolMenuItem[]
}

defineProps<{
  target: 'canvas' | 'node'
  toolGroups: ToolMenuGroup[]
  canAskAgent: boolean
  canThink: boolean
  canRunWorkflow: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'ask-agent'): void
  (e: 'think-problem'): void
  (e: 'run-workflow'): void
  (e: 'run-tool', toolName: string): void
  (e: 'edit'): void
  (e: 'duplicate'): void
  (e: 'delete'): void
  (e: 'disconnect-edges'): void
}>()

function handleAction(action: () => void) {
  action()
  emit('close')
}

function handleTool(toolName: string) {
  emit('run-tool', toolName)
  emit('close')
}
</script>

<template>
  <template v-if="target === 'canvas'">
    <ContextMenuItem :disabled="!canAskAgent" @select="handleAction(() => emit('ask-agent'))">Ask an agent…</ContextMenuItem>
    <ContextMenuSub>
      <ContextMenuSubTrigger :disabled="toolGroups.length === 0">
        <span>Run a tool…</span>
      </ContextMenuSubTrigger>
      <ContextMenuSubContent class="composer-context-menu__submenu-panel">
        <template v-for="(group, index) in toolGroups" :key="group.server">
          <ContextMenuSub>
            <ContextMenuSubTrigger>
              <span class="composer-context-menu__server-name">{{ group.server }}</span>
              <span class="composer-context-menu__tool-count">{{ group.tools.length }}</span>
            </ContextMenuSubTrigger>
            <ContextMenuSubContent>
              <ContextMenuItem
                v-for="tool in group.tools"
                :key="tool.name"
                :title="tool.description ?? tool.name"
                @select="handleTool(tool.name)"
              >
                {{ tool.displayName }}
              </ContextMenuItem>
            </ContextMenuSubContent>
          </ContextMenuSub>
          <ContextMenuSeparator v-if="index < toolGroups.length - 1" />
        </template>
      </ContextMenuSubContent>
    </ContextMenuSub>
    <ContextMenuSeparator />
    <ContextMenuItem :disabled="!canThink" @select="handleAction(() => emit('think-problem'))">Think through a problem…</ContextMenuItem>
    <ContextMenuItem :disabled="!canRunWorkflow" @select="handleAction(() => emit('run-workflow'))">Run a workflow…</ContextMenuItem>
  </template>

  <template v-else>
    <ContextMenuItem @select="handleAction(() => emit('edit'))">Edit</ContextMenuItem>
    <ContextMenuItem @select="handleAction(() => emit('duplicate'))">Duplicate</ContextMenuItem>
    <ContextMenuItem variant="destructive" @select="handleAction(() => emit('delete'))">Delete</ContextMenuItem>
    <ContextMenuSeparator />
    <ContextMenuItem @select="handleAction(() => emit('disconnect-edges'))">Disconnect all edges</ContextMenuItem>
  </template>
</template>

<style scoped>
.composer-context-menu__submenu-panel { min-width: 240px; max-height: min(420px, 70vh); overflow: auto; }
.composer-context-menu__server-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.composer-context-menu__tool-count { flex-shrink: 0; padding: 0.1rem 0.45rem; border-radius: 9999px; background: rgba(255, 255, 255, 0.1); color: rgba(255, 255, 255, 0.56); font-size: var(--text-xs); font-weight: var(--weight-display); line-height: 1.3; }
</style>
