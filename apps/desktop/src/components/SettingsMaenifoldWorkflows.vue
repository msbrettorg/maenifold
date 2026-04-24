<script setup lang="ts">
import type { MaenifoldSettings } from '../lib/types';
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

defineProps<{
  form: MaenifoldSettings;
  onUpdatePipelineDriver: (event: Event) => void;
  onPipelineDriverDefaultToggle: (event: Event) => void;
}>();
</script>

<template>
  <Accordion
    type="multiple"
    :default-value="['workflows-debug']"
    class="flex flex-col gap-4"
    role="group"
    aria-label="Maenifold workflow settings"
  >
    <AccordionItem value="workflows-debug" class="p-4 rounded bg-bg-elevated border border-border-subtle">
      <AccordionTrigger><h4 class="m-0">Workflows &amp; Debug</h4></AccordionTrigger>
      <AccordionContent>
        <div class="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-[0.9rem]">
        <div class="flex gap-[0.65rem] items-center min-h-[42px] p-3 border border-border-subtle rounded bg-bg-app">
          <Checkbox :checked="form.debug.trace" @update:checked="(val: boolean) => { form.debug.trace = val }" />
          <Label>Trace logging</Label>
        </div>

        <div class="flex flex-col gap-[0.45rem] p-[0.9rem] border border-border-subtle rounded bg-bg-app col-span-full">
          <div class="flex flex-col gap-[0.45rem]">
            <Label>Pipeline driver</Label>
            <Input
              :disabled="form.debug.pipelineDriver === null"
              :model-value="form.debug.pipelineDriver ?? ''"
              type="text"
              placeholder="inline"
              @update:model-value="(val) => onUpdatePipelineDriver({ target: { value: String(val) } } as unknown as Event)"
            />
          </div>
          <div class="flex gap-[0.65rem] items-center">
            <Checkbox
              :checked="form.debug.pipelineDriver === null"
              @update:checked="(val: boolean) => onPipelineDriverDefaultToggle({ target: { checked: val } } as unknown as Event)"
            />
            <Label>Use default</Label>
          </div>
        </div>
        </div>
      </AccordionContent>
    </AccordionItem>
  </Accordion>
</template>
