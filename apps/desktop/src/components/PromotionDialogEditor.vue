<script setup lang="ts">
import { ref } from 'vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import type {
  NormalizedMemoryDocument,
  NormalizedMemorySearchItem,
} from '../composables/useMemoryPromotion';

type PromotionAction = 'create' | 'append' | 'update';

const props = defineProps<{
  selectedAction: PromotionAction;
  selectedResult: NormalizedMemorySearchItem | null;
  selectedMemory: NormalizedMemoryDocument | null;
  loadingMemory: boolean;
  title: string;
  folder: string;
  tagsInput: string;
  wikiLinkInput: string;
  wikiLinkError: string;
  nextPreview: string;
  formError: string;
  updateDisabledMessage: string;
  busy: boolean;
}>();

const emit = defineEmits<{
  (event: 'update:title', value: string): void;
  (event: 'update:folder', value: string): void;
  (event: 'update:tagsInput', value: string): void;
  (event: 'update:wikiLinkInput', value: string): void;
  (event: 'back'): void;
  (event: 'continue'): void;
  (event: 'retry'): void;
  (event: 'close'): void;
}>();

const wikiLinkInputRef = ref<HTMLElement | { $el?: HTMLElement } | null>(null);

function focusWikiLinkInput(): void {
  if (wikiLinkInputRef.value instanceof HTMLElement) {
    wikiLinkInputRef.value.focus();
    return;
  }
  const textarea = wikiLinkInputRef.value?.$el instanceof HTMLElement
    ? wikiLinkInputRef.value.$el
    : document.getElementById('promotion-connections-input');
  textarea?.focus();
}

defineExpose({ focusWikiLinkInput });

const heading = {
  create: 'Create new note',
  append: 'Append to existing note',
  update: 'Update existing note',
}[props.selectedAction];

const description = {
  create: 'We will create a new note with this promoted block.',
  append: 'We will add a new promoted block to the selected note.',
  update: 'We will replace the note’s Promoted Content section when we can do it safely.',
}[props.selectedAction];
</script>

<template>
  <section class="grid gap-4 px-4 pb-4">
    <div class="flex gap-3 max-[720px]:flex-col">
      <div>
        <h4 class="m-0 font-display text-base font-medium text-text-primary">{{ heading }}</h4>
        <p class="text-text-secondary">{{ description }}</p>
      </div>
      <Button type="button" variant="ghost" :disabled="busy" @click="emit('back')">Back</Button>
    </div>

    <div v-if="selectedAction === 'create'" class="grid gap-4 grid-cols-[repeat(auto-fit,minmax(14rem,1fr))]">
      <label class="grid min-w-0 gap-[0.35rem] text-xs text-text-secondary">
        <span>Title</span>
        <Input class="border-0" type="text" placeholder="Note title" :value="title" @input="emit('update:title', ($event.target as HTMLInputElement).value.trim())" />
      </label>
      <label class="grid min-w-0 gap-[0.35rem] text-xs text-text-secondary">
        <span>Folder</span>
        <Input class="border-0" type="text" placeholder="Optional folder" :value="folder" @input="emit('update:folder', ($event.target as HTMLInputElement).value.trim())" />
      </label>
      <label class="col-[1/-1] grid min-w-0 gap-[0.35rem] text-xs text-text-secondary">
        <span>Tags</span>
        <Input class="border-0" type="text" placeholder="Optional tags, separated by commas" :value="tagsInput" @input="emit('update:tagsInput', ($event.target as HTMLInputElement).value.trim())" />
      </label>
    </div>

    <label class="grid min-w-0 gap-[0.35rem] text-xs text-text-secondary">
      <span class="flex flex-wrap items-center justify-between gap-2">
        <span>Connections</span>
        <span
          v-if="wikiLinkError"
          id="promotion-connections-error"
          class="rounded-[4px] bg-danger/10 px-2 py-1 text-xs font-medium text-danger"
          role="alert"
        >
          {{ wikiLinkError }}
        </span>
      </span>
      <Textarea
        id="promotion-connections-input"
        ref="wikiLinkInputRef"
        class="border-0"
        rows="3"
        placeholder="Add one or more topics, for example: semantic zoom, pipeline-ui"
        :value="wikiLinkInput"
        :aria-invalid="Boolean(wikiLinkError)"
        :aria-describedby="wikiLinkError ? 'promotion-connections-error' : undefined"
        @input="emit('update:wikiLinkInput', ($event.target as HTMLTextAreaElement).value)"
      />
      <small class="text-text-secondary">Use plain topics or [[WikiLinks]]. We will format them for you.</small>
    </label>

    <div v-if="selectedAction !== 'create'" class="grid gap-4">
      <span class="text-xs text-text-primary">Current note</span>
      <pre v-if="selectedMemory" class="m-0 rounded-[4px] bg-surface p-3 font-[inherit] whitespace-pre-wrap break-words">{{ selectedMemory.content }}</pre>
      <pre v-else-if="loadingMemory" class="m-0 rounded-[4px] bg-surface p-3 font-[inherit] whitespace-pre-wrap break-words">Loading note…</pre>
      <pre v-else-if="selectedResult" class="m-0 rounded-[4px] bg-surface p-3 font-[inherit] whitespace-pre-wrap break-words">{{ selectedResult.uri }}</pre>
      <pre v-else class="m-0 rounded-[4px] bg-surface p-3 font-[inherit] whitespace-pre-wrap break-words">Select a note to continue.</pre>
    </div>

    <p v-if="selectedAction === 'update' && updateDisabledMessage" class="text-text-secondary">{{ updateDisabledMessage }}</p>

    <div class="grid gap-4">
      <span class="text-xs text-text-primary">What will be saved</span>
      <pre class="m-0 rounded-[4px] bg-surface p-3 font-[inherit] whitespace-pre-wrap break-words">{{ nextPreview }}</pre>
    </div>

    <p v-if="formError" class="m-0 text-status-abandoned text-xs" role="alert">{{ formError }}</p>

    <div class="flex flex-wrap items-center gap-3 max-[720px]:justify-stretch [&>:first-child:not(:only-child)]:mr-auto">
      <Button type="button" variant="ghost" :disabled="busy" @click="emit('close')">Cancel</Button>
      <Button type="button" variant="ghost" :disabled="busy" @click="emit('retry')">Retry</Button>
      <Button type="button" :disabled="busy" @click="emit('continue')">Continue</Button>
    </div>
  </section>
</template>
