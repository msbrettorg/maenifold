<script setup lang="ts">
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
  <section class="dialog-section action-section">
    <div class="section-header">
      <div>
        <h4>{{ heading }}</h4>
        <p>{{ description }}</p>
      </div>
      <Button type="button" variant="ghost" :disabled="busy" @click="emit('back')">Back</Button>
    </div>

    <div v-if="selectedAction === 'create'" class="form-grid">
      <label class="field-block">
        <span>Title</span>
        <Input type="text" placeholder="Note title" :value="title" @input="emit('update:title', ($event.target as HTMLInputElement).value.trim())" />
      </label>
      <label class="field-block">
        <span>Folder</span>
        <Input type="text" placeholder="Optional folder" :value="folder" @input="emit('update:folder', ($event.target as HTMLInputElement).value.trim())" />
      </label>
      <label class="field-block field-wide">
        <span>Tags</span>
        <Input type="text" placeholder="Optional tags, separated by commas" :value="tagsInput" @input="emit('update:tagsInput', ($event.target as HTMLInputElement).value.trim())" />
      </label>
    </div>

    <label class="field-block">
      <span>Connections</span>
      <Textarea
        rows="3"
        placeholder="Add one or more topics, for example: semantic zoom, pipeline-ui"
        :value="wikiLinkInput"
        @input="emit('update:wikiLinkInput', ($event.target as HTMLTextAreaElement).value)"
      />
      <small class="field-help">Use plain topics or [[WikiLinks]]. We will format them for you.</small>
    </label>
    <p v-if="wikiLinkError" class="form-error" role="alert">{{ wikiLinkError }}</p>

    <div v-if="selectedAction !== 'create'" class="memory-preview">
      <span>Current note</span>
      <pre v-if="selectedMemory">{{ selectedMemory.content }}</pre>
      <pre v-else-if="loadingMemory">Loading note…</pre>
      <pre v-else-if="selectedResult">{{ selectedResult.uri }}</pre>
      <pre v-else>Select a note to continue.</pre>
    </div>

    <p v-if="selectedAction === 'update' && updateDisabledMessage" class="info-message">{{ updateDisabledMessage }}</p>

    <div class="memory-preview">
      <span>What will be saved</span>
      <pre>{{ nextPreview }}</pre>
    </div>

    <p v-if="formError" class="form-error" role="alert">{{ formError }}</p>

    <div class="dialog-actions">
      <Button type="button" variant="ghost" :disabled="busy" @click="emit('close')">Cancel</Button>
      <Button type="button" variant="ghost" :disabled="busy" @click="emit('retry')">Retry</Button>
      <Button type="button" :disabled="busy" @click="emit('continue')">Continue</Button>
    </div>
  </section>
</template>
