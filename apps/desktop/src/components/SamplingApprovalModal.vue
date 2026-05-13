<script setup lang="ts">
import { computed, ref } from 'vue';
import { Sparkles } from 'lucide-vue-next';
import { Button } from './ui/button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from './ui/select';
import type { SamplingAction, SamplingDecision, SamplingRequest } from '../stores/sampling';

const props = defineProps<{
  request: SamplingRequest;
}>();

const emit = defineEmits<{
  resolve: [decision: SamplingDecision];
}>();

const selectedModel = ref<string>('');

const visibleMessages = computed(() => props.request.messages.filter((message) => message.text));
const modelChoices = computed(() => {
  const hint = props.request.modelHint;
  return hint ? [hint, `${hint}-mini`] : ['gpt-4.1', 'gpt-4.1-mini'];
});

function decide(action: SamplingAction): void {
  emit('resolve', {
    action,
    ...(selectedModel.value ? { selectedModel: selectedModel.value } : {}),
  });
}
</script>

<template>
  <Dialog :open="true">
    <DialogContent class="gap-4 p-0 sm:max-w-[36rem]" :show-close-button="false">
      <DialogHeader class="px-6 pt-6">
        <div class="flex items-start gap-3">
          <div class="rounded-[12px] bg-bg-elevated p-2 text-brand">
            <Sparkles class="size-4" aria-hidden="true" />
          </div>
          <div class="min-w-0">
            <DialogTitle>Approve sampling request</DialogTitle>
            <p class="m-0 mt-2 text-sm text-text-secondary">
              {{ request.serverName }} wants to sample a model response.
            </p>
          </div>
        </div>
      </DialogHeader>

      <div class="grid gap-4 px-6">
        <div class="grid gap-2 rounded-[12px] bg-bg-elevated p-3">
          <p class="m-0 text-xs uppercase tracking-wide text-text-tertiary">Messages</p>
          <div v-for="message in visibleMessages" :key="`${message.role}-${message.text}`" class="grid gap-1">
            <p class="m-0 text-xs text-text-tertiary">{{ message.role }}</p>
            <p class="m-0 text-sm text-text-primary">{{ message.text }}</p>
          </div>
        </div>

        <div class="grid gap-2 rounded-[12px] bg-bg-elevated p-3">
          <p class="m-0 text-xs uppercase tracking-wide text-text-tertiary">Model</p>
          <p class="m-0 text-sm text-text-secondary">
            Requested model hint: <span class="text-text-primary">{{ request.modelHint ?? 'none' }}</span>
          </p>
          <Select :model-value="selectedModel" @update:model-value="(value) => selectedModel = String(value)">
            <SelectTrigger class="bg-bg-app text-text-primary">
              <SelectValue placeholder="Use requested model" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem v-for="model in modelChoices" :key="model" :value="model">{{ model }}</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      <DialogFooter class="gap-2 px-6 pb-6">
        <Button type="button" variant="ghost" @click="decide('deny')">Deny</Button>
        <Button type="button" variant="outline" @click="decide('allow_always')">Allow Always</Button>
        <Button type="button" @click="decide('allow_once')">Allow Once</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
