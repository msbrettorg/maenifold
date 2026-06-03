<script setup lang="ts">
// Pipeline runner panel (T-600.3). Teleported into the shell's `#pipeline-runner`
// mount target so it floats over the bottom of the canvas without the shell
// owning it. Composes the lifecycle action bar, the run-input form, the live
// event log, and the run-history table around the selected pipeline. All
// behavior flows through `usePipelineRunner` (polling + lifecycle ops); pure
// decision logic lives in the sibling `.ts` helpers.
import { computed, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { TriangleAlert, X } from '@lucide/vue';
import { usePipelinesStore } from '../../../stores/pipelines';
import type { PipelineInputs } from '../../../lib/pipeline-types';
import { usePipelineRunner } from './usePipelineRunner';
import RunnerActionBar from './RunnerActionBar.vue';
import RunInputForm from './RunInputForm.vue';
import EventLogPanel from './EventLogPanel.vue';
import RunHistoryPanel from './RunHistoryPanel.vue';

type RunnerTab = 'run' | 'events' | 'history';

const store = usePipelinesStore();
const { pipelines } = storeToRefs(store);
const runner = usePipelineRunner();

const tab = ref<RunnerTab>('events');
const createOpen = ref(false);
const createName = ref('');
const createDescription = ref('');
const createConcurrency = ref(8);

// The list item backs the run-history outcomes (per-run `outcome` lives on the
// list payload, not the view).
const listItem = computed(
  () => pipelines.value.find((item) => item.pipelineId === runner.selectedPipelineId.value) ?? null
);

const inputSchema = computed(() => runner.selectedView.value?.inputSchema ?? null);

function onRunClicked(): void {
  // With an input schema (or to capture an optional prompt) route the operator to
  // the Run tab to fill the form; the form's submit triggers the actual run.
  tab.value = 'run';
}

function onRunSubmit(payload: { inputs?: PipelineInputs; prompt?: string }): void {
  void runner.run(payload);
  tab.value = 'events';
}

function onNew(): void {
  createOpen.value = true;
}

async function onCreate(): Promise<void> {
  const name = createName.value.trim();
  if (!name) return;
  await runner.create({
    name,
    description: createDescription.value.trim() || undefined,
    maxConcurrency: createConcurrency.value,
  });
  if (!runner.actionError.value) {
    createOpen.value = false;
    createName.value = '';
    createDescription.value = '';
  }
}

async function onDelete(): Promise<void> {
  await runner.remove(false);
}

async function onForceDelete(): Promise<void> {
  await runner.remove(true);
}
</script>

<template>
  <Teleport defer to="#pipeline-runner">
    <section class="prunner" aria-label="Pipeline runner">
      <RunnerActionBar
        :flags="runner.flags.value"
        :status="runner.selectedView.value?.status ?? null"
        :busy="runner.busy.value || runner.designModeActive.value"
        @new="onNew"
        @run="onRunClicked"
        @pause="runner.pause"
        @resume="runner.resume"
        @cancel="runner.cancel"
        @delete="onDelete"
      />

      <p v-if="runner.actionError.value" class="prunner-error" role="status">
        {{ runner.actionError.value }}
      </p>

      <!-- Force-delete confirm: shown only after a plain delete fails. No auto-force. -->
      <div v-if="runner.deleteNeedsForce.value" class="confirm" role="alertdialog">
        <span class="confirm-msg">
          <TriangleAlert :size="15" :stroke-width="2" />
          Delete failed — this pipeline requires a force delete.
        </span>
        <span class="confirm-actions">
          <button type="button" class="confirm-btn danger" :disabled="runner.busy.value" @click="onForceDelete">
            Force delete
          </button>
          <button type="button" class="confirm-btn" @click="runner.dismissForceDelete">Keep</button>
        </span>
      </div>

      <!-- Minimal create entry. Full node/edge designer is T-600.4. Available
           regardless of selection so New always works. -->
      <form v-if="createOpen" class="create" @submit.prevent="onCreate">
        <div class="create-head">
          <strong>New pipeline</strong>
          <button type="button" class="create-x" aria-label="Close" @click="createOpen = false">
            <X :size="14" :stroke-width="2" />
          </button>
        </div>
        <input v-model="createName" class="create-input" placeholder="Name (required)" aria-label="Pipeline name" />
        <input
          v-model="createDescription"
          class="create-input"
          placeholder="Description (optional)"
          aria-label="Pipeline description"
        />
        <label class="create-conc">
          Max concurrency
          <input v-model.number="createConcurrency" type="number" min="1" class="create-num" />
        </label>
        <button type="submit" class="create-btn" :disabled="runner.busy.value || !createName.trim()">
          Create empty pipeline
        </button>
        <p class="create-note">Creates an empty-graph session; design nodes in the canvas (T-600.4).</p>
      </form>

      <!-- Empty state: no pipeline selected and not mid-create. -->
      <p v-if="!runner.selectedView.value && !createOpen" class="prunner-empty">
        No pipeline selected. Choose one from the list, or create a new one.
      </p>

      <!-- Selection-scoped body: tabs + run/events/history only with a live view. -->
      <template v-if="runner.selectedView.value">
        <nav class="ptabs" aria-label="Runner sections">
          <button type="button" class="ptab" :class="{ active: tab === 'run' }" @click="tab = 'run'">Run</button>
          <button type="button" class="ptab" :class="{ active: tab === 'events' }" @click="tab = 'events'">
            Events
          </button>
          <button type="button" class="ptab" :class="{ active: tab === 'history' }" @click="tab = 'history'">
            History
          </button>
        </nav>

        <div class="pbody">
          <RunInputForm
            v-show="tab === 'run'"
            :schema="inputSchema"
            :disabled="runner.busy.value"
            @submit="onRunSubmit"
          />
          <EventLogPanel
            v-show="tab === 'events'"
            :entries="runner.eventLog.value"
            :polling-active="runner.pollingActive.value"
            :poll-error="runner.pollError.value"
          />
          <RunHistoryPanel
            v-show="tab === 'history'"
            :view="runner.selectedView.value"
            :list-item="listItem"
          />
        </div>
      </template>
    </section>
  </Teleport>
</template>

<style scoped>
.prunner {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  max-height: 45%;
  padding: 0.55rem 0.65rem;
  border: 1px solid var(--border-color);
  border-radius: 9px;
  background: color-mix(in srgb, var(--bg-main) 92%, var(--bg-sidebar));
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.16);
}

.prunner-error {
  margin: 0;
  padding: 0.4rem 0.5rem;
  border-radius: 6px;
  background: color-mix(in srgb, var(--bg-danger) 14%, var(--bg-main));
  color: var(--text-primary);
  font-size: 0.76rem;
}

.prunner-empty {
  margin: 0;
  padding: 0.6rem 0.55rem;
  border: 1px dashed var(--border-color);
  border-radius: 7px;
  color: var(--text-secondary);
  font-size: 0.78rem;
  text-align: center;
}

.confirm {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  flex-wrap: wrap;
  padding: 0.45rem 0.55rem;
  border: 1px solid color-mix(in srgb, var(--bg-danger) 40%, var(--border-color));
  border-radius: 7px;
  background: color-mix(in srgb, var(--bg-danger) 10%, var(--bg-main));
}

.confirm-msg {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.78rem;
  color: var(--text-primary);
}

.confirm-actions {
  display: inline-flex;
  gap: 0.3rem;
}

.confirm-btn {
  padding: 0.3rem 0.6rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-main);
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.76rem;
  cursor: pointer;
}

.confirm-btn.danger {
  color: var(--bg-danger);
  border-color: color-mix(in srgb, var(--bg-danger) 40%, transparent);
}

.confirm-btn:disabled {
  cursor: default;
  opacity: 0.5;
}

.create {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding: 0.5rem 0.55rem;
  border: 1px solid var(--border-color);
  border-radius: 7px;
  background: var(--bg-main);
}

.create-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: var(--text-primary);
  font-size: 0.82rem;
}

.create-x {
  display: inline-flex;
  padding: 0.2rem;
  border: none;
  border-radius: 5px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}

.create-x:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.create-input,
.create-num {
  padding: 0.35rem 0.5rem;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: color-mix(in srgb, var(--bg-main) 96%, var(--bg-sidebar));
  color: var(--text-primary);
  font: inherit;
  font-size: 0.8rem;
  outline: none;
}

.create-conc {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.78rem;
  color: var(--text-secondary);
}

.create-num {
  width: 5rem;
}

.create-btn {
  align-self: flex-start;
  padding: 0.4rem 0.7rem;
  border: 1px solid color-mix(in srgb, var(--text-accent) 35%, transparent);
  border-radius: 6px;
  background: color-mix(in srgb, var(--text-accent) 12%, transparent);
  color: var(--text-accent);
  font: inherit;
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
}

.create-btn:disabled {
  cursor: default;
  opacity: 0.45;
}

.create-note {
  margin: 0;
  font-size: 0.72rem;
  color: var(--text-muted);
}

.ptabs {
  display: inline-flex;
  gap: 0.2rem;
  border-bottom: 1px solid var(--border-color);
}

.ptab {
  padding: 0.3rem 0.6rem;
  border: none;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.8rem;
  cursor: pointer;
}

.ptab:hover {
  color: var(--text-primary);
}

.ptab.active {
  color: var(--text-accent);
  border-bottom-color: var(--text-accent);
}

.pbody {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.pbody > * {
  flex: 1;
  min-height: 0;
}
</style>
