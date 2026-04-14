<script setup lang="ts">
import { ContextMenuItem, ContextMenuSeparator } from '@/components/ui/context-menu'

defineProps<{
  target: 'canvas' | 'node'
  pipelineStatus?: string | null
  pipelineRunning?: boolean
  hasRewindableNodes?: boolean
  hasCompletedNodesWithOutput?: boolean
  isNonTerminal: boolean
  showRestart: boolean
  showRewind: boolean
  showFork: boolean
  showAbandon: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'restart'): void
  (e: 'rewind'): void
  (e: 'fork'): void
  (e: 'abandon'): void
  (e: 'run-pipeline'): void
  (e: 'rewind-all'): void
  (e: 'graft'): void
  (e: 'promote-to-memory'): void
}>()

function handleAction(action: () => void) {
  action()
  emit('close')
}
</script>

<template>
  <template v-if="target === 'node'">
    <ContextMenuItem v-if="showRestart" @select="handleAction(() => emit('restart'))">Restart</ContextMenuItem>
    <ContextMenuItem v-if="showRewind" @select="handleAction(() => emit('rewind'))">Rewind</ContextMenuItem>
    <ContextMenuItem v-if="showFork" @select="handleAction(() => emit('fork'))">Fork</ContextMenuItem>
    <ContextMenuItem v-if="showAbandon" variant="destructive" @select="handleAction(() => emit('abandon'))">Abandon</ContextMenuItem>
  </template>

  <template v-else>
    <ContextMenuItem v-if="pipelineStatus === 'active' && !pipelineRunning" @select="handleAction(() => emit('run-pipeline'))">▶ Run Pipeline</ContextMenuItem>
    <ContextMenuItem v-if="hasRewindableNodes" @select="handleAction(() => emit('rewind-all'))">↩ Rewind</ContextMenuItem>
    <ContextMenuItem v-if="isNonTerminal" @select="handleAction(() => emit('graft'))">⑂ Graft</ContextMenuItem>
    <ContextMenuSeparator v-if="hasCompletedNodesWithOutput && ((pipelineStatus === 'active' && !pipelineRunning) || hasRewindableNodes || isNonTerminal)" />
    <ContextMenuItem v-if="hasCompletedNodesWithOutput" @select="handleAction(() => emit('promote-to-memory'))">✦ Promote to Memory</ContextMenuItem>
  </template>
</template>
