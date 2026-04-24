<script setup lang="ts">
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { MaenifoldSettings } from '../lib/types';

const props = defineProps<{
  form: MaenifoldSettings;
  onCommunityDefaultToggle: (event: Event) => void;
  onUpdateCommunityRecompute: (event: Event) => void;
}>();

function emitCommunityRecompute(value: string | number) {
  const synthetic = {
    target: { value: String(value) },
  } as unknown as Event;
  props.onUpdateCommunityRecompute(synthetic);
}

function emitCommunityDefaultToggle(checked: boolean | 'indeterminate') {
  const synthetic = {
    target: { checked: checked === true },
  } as unknown as Event;
  props.onCommunityDefaultToggle(synthetic);
}
</script>

<template>
  <Accordion
    type="multiple"
    :default-value="['decay', 'community', 'vector']"
    class="flex flex-col gap-4"
    role="group"
    aria-label="Maenifold graph settings"
  >
    <AccordionItem value="decay" class="p-4 rounded bg-bg-elevated border border-border-subtle">
      <AccordionTrigger><h4 class="m-0">Decay</h4></AccordionTrigger>
      <AccordionContent>
        <div class="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-[0.9rem]">
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Grace days sequential</Label>
          <Input
            :model-value="form.decay.graceDaysSequential"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.decay.graceDaysSequential = Number(val)"
          />
        </div>
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Grace days workflows</Label>
          <Input
            :model-value="form.decay.graceDaysWorkflows"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.decay.graceDaysWorkflows = Number(val)"
          />
        </div>
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Grace days default</Label>
          <Input
            :model-value="form.decay.graceDaysDefault"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.decay.graceDaysDefault = Number(val)"
          />
        </div>
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Half life days</Label>
          <Input
            :model-value="form.decay.halfLifeDays"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.decay.halfLifeDays = Number(val)"
          />
        </div>
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Decay function</Label>
          <Select v-model="form.decay.function">
            <SelectTrigger class="w-full">
              <SelectValue placeholder="Select decay function" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="power-law">power-law</SelectItem>
              <SelectItem value="exponential">exponential</SelectItem>
              <SelectItem value="linear">linear</SelectItem>
            </SelectContent>
          </Select>
        </div>
        </div>
      </AccordionContent>
    </AccordionItem>

    <AccordionItem value="community" class="p-4 rounded bg-bg-elevated border border-border-subtle">
      <AccordionTrigger><h4 class="m-0">Community Detection</h4></AccordionTrigger>
      <AccordionContent>
        <div class="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-[0.9rem]">
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Louvain gamma</Label>
          <Input
            :model-value="form.community.louvainGamma"
            type="number"
            min="0"
            step="0.1"
            @update:model-value="(val: string | number) => form.community.louvainGamma = Number(val)"
          />
        </div>
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Min shared neighbors</Label>
          <Input
            :model-value="form.community.minSharedNeighbors"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.community.minSharedNeighbors = Number(val)"
          />
        </div>
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Min overlap</Label>
          <Input
            :model-value="form.community.minOverlap"
            type="number"
            min="0"
            max="1"
            step="0.05"
            @update:model-value="(val: string | number) => form.community.minOverlap = Number(val)"
          />
        </div>
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Max siblings</Label>
          <Input
            :model-value="form.community.maxSiblings"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.community.maxSiblings = Number(val)"
          />
        </div>
        <div class="flex flex-col gap-[0.45rem]">
          <Label>Debounce ms</Label>
          <Input
            :model-value="form.community.debounceMs"
            type="number"
            min="0"
            step="1"
            @update:model-value="(val: string | number) => form.community.debounceMs = Number(val)"
          />
        </div>
        <div class="flex flex-col gap-[0.45rem]">
          <Label>NMI threshold</Label>
          <Input
            :model-value="form.community.nmiThreshold"
            type="number"
            min="0"
            max="1"
            step="0.05"
            @update:model-value="(val: string | number) => form.community.nmiThreshold = Number(val)"
          />
        </div>
        <div class="flex flex-col gap-[0.45rem] p-[0.9rem] border border-border-subtle rounded bg-bg-app">
          <div class="flex flex-col gap-[0.45rem]">
            <Label>Recompute file threshold</Label>
            <Input
              :disabled="form.community.recomputeFileThreshold === null"
              :model-value="form.community.recomputeFileThreshold ?? 0"
              type="number"
              min="0"
              step="1"
              @update:model-value="emitCommunityRecompute"
            />
          </div>
          <Label class="flex gap-[0.65rem] items-center">
            <Checkbox
              :model-value="form.community.recomputeFileThreshold === null"
              @update:model-value="emitCommunityDefaultToggle"
            />
            <span>Use default</span>
          </Label>
        </div>
        </div>
      </AccordionContent>
    </AccordionItem>

    <AccordionItem value="vector" class="p-4 rounded bg-bg-elevated border border-border-subtle">
      <AccordionTrigger><h4 class="m-0">Vector / Embedding</h4></AccordionTrigger>
      <AccordionContent>
        <div class="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-[0.9rem]">
        <Label class="flex gap-[0.65rem] items-center min-h-[42px] p-3 border border-border-subtle rounded bg-bg-app"><Checkbox v-model="form.vector.embeddingLogs" /><span>Embedding logs</span></Label>
        <Label class="flex gap-[0.65rem] items-center min-h-[42px] p-3 border border-border-subtle rounded bg-bg-app"><Checkbox v-model="form.vector.vectorLogs" /><span>Vector logs</span></Label>
        <Label class="flex gap-[0.65rem] items-center min-h-[42px] p-3 border border-border-subtle rounded bg-bg-app"><Checkbox v-model="form.vector.disableVector" /><span>Disable vector</span></Label>
      </div>
      </AccordionContent>
    </AccordionItem>
  </Accordion>
</template>
