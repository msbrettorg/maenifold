<script setup lang="ts">
import { MessageCircle } from 'lucide-vue-next'
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
  (e: 'add-session-node'): void
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
    <ContextMenuItem @select="handleAction(() => emit('add-session-node'))">
      <MessageCircle />
      Add session node
    </ContextMenuItem>
    <ContextMenuSub>
      <ContextMenuSubTrigger :disabled="toolGroups.length === 0">
        <span>Run a tool…</span>
      </ContextMenuSubTrigger>
      <ContextMenuSubContent class="min-w-[240px] max-h-[min(420px,70vh)] overflow-auto">
        <template v-for="(group, index) in toolGroups" :key="group.server">
          <ContextMenuSub>
            <ContextMenuSubTrigger>
              <span class="flex-1 overflow-hidden text-ellipsis whitespace-nowrap">{{ group.server }}</span>
              <span class="shrink-0 rounded-[4px] bg-border-card px-[0.45rem] py-[0.1rem] text-text-secondary text-xs font-medium leading-[1.3]">{{ group.tools.length }}</span>
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
