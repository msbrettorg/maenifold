import { ref, computed } from 'vue';
import { defineStore } from 'pinia';

type ZoomLevel = 'graph' | 'inspect' | 'session';

export const useSemanticZoomStore = defineStore('semanticZoom', () => {
  const level = ref<ZoomLevel>('graph');
  const focusedNodeId = ref<string | null>(null);
  const focusedSessionId = ref<string | null>(null);
  const sourceRect = ref<DOMRect | null>(null);

  const isGraph = computed(() => level.value === 'graph');
  const isInspecting = computed(() => level.value === 'inspect');
  const isSession = computed(() => level.value === 'session');

  function enterInspect(nodeId: string, rect: DOMRect) {
    if (!nodeId) return;
    focusedNodeId.value = nodeId;
    sourceRect.value = rect;
    level.value = 'inspect';
  }

  function enterSession(sessionId: string) {
    if (!sessionId || level.value !== 'inspect') return;
    focusedSessionId.value = sessionId;
    level.value = 'session';
  }

  function back() {
    if (level.value === 'session') {
      focusedSessionId.value = null;
      level.value = 'inspect';
    } else if (level.value === 'inspect') {
      focusedNodeId.value = null;
      sourceRect.value = null;
      level.value = 'graph';
    }
  }

  function reset() {
    level.value = 'graph';
    focusedNodeId.value = null;
    focusedSessionId.value = null;
    sourceRect.value = null;
  }

  return {
    level, focusedNodeId, focusedSessionId, sourceRect,
    isGraph, isInspecting, isSession,
    enterInspect, enterSession, back, reset,
  };
});
