<script setup lang="ts">
// Surgery status toast (T-600.5). Teleported into `#pipeline-overlay`. Reflects
// the shared surgery action state (busy / notice / lastError) from the module
// singleton so EVERY surgery surface — menu, toolbar, dialogs, inspector button —
// reports through one consistent place. Dismissable; auto-cleared by the next op.
import { computed } from 'vue';
import { Loader, CheckCircle2, AlertTriangle, X } from '@lucide/vue';
import { useSurgeryController } from './useSurgeryController';

const controller = useSurgeryController();

const kind = computed<'busy' | 'error' | 'notice' | null>(() => {
  if (controller.busy.value) return 'busy';
  if (controller.lastError.value) return 'error';
  if (controller.notice.value) return 'notice';
  return null;
});

const text = computed(() => {
  if (controller.busy.value) return 'Applying graph surgery…';
  return controller.lastError.value ?? controller.notice.value ?? '';
});

function dismiss(): void {
  controller.lastError.value = null;
  controller.notice.value = null;
}
</script>

<template>
  <Teleport defer to="#pipeline-overlay">
    <div v-if="kind" class="surg-toast" :class="kind" role="status" aria-live="polite">
      <Loader v-if="kind === 'busy'" :size="15" :stroke-width="2" class="spin" />
      <CheckCircle2 v-else-if="kind === 'notice'" :size="15" :stroke-width="2" />
      <AlertTriangle v-else :size="15" :stroke-width="2" />
      <span class="surg-toast-text">{{ text }}</span>
      <button v-if="kind !== 'busy'" type="button" class="surg-toast-x" @click="dismiss">
        <X :size="14" :stroke-width="2" />
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
.surg-toast {
  position: absolute;
  left: 50%;
  bottom: 1rem;
  transform: translateX(-50%);
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  max-width: 80%;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-main);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.28);
  font-size: 0.8rem;
  color: var(--text-primary);
  z-index: 24;
}

.surg-toast.error {
  border-color: color-mix(in srgb, var(--bg-danger, #d9534f) 55%, var(--border-color));
  color: var(--bg-danger, #d9534f);
}

.surg-toast.notice {
  border-color: color-mix(in srgb, var(--text-accent) 45%, var(--border-color));
}

.surg-toast-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.surg-toast-x {
  display: inline-flex;
  padding: 0.1rem;
  border: none;
  background: transparent;
  color: inherit;
  cursor: pointer;
  opacity: 0.7;
}

.surg-toast-x:hover {
  opacity: 1;
}

.spin {
  animation: surg-spin 0.9s linear infinite;
}

@keyframes surg-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
