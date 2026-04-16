<script setup lang="ts">
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import type { NormalizedMemorySearchItem } from '../composables/useMemoryPromotion';

type PromotionAction = 'create' | 'append' | 'update';

defineProps<{
  searchQuery: string;
  searchResults: NormalizedMemorySearchItem[];
  searching: boolean;
  writing: boolean;
  infoMessage: string;
  formError: string;
  promotionError: string | null;
  busy: boolean;
}>();

const emit = defineEmits<{
  (event: 'update:searchQuery', value: string): void;
  (event: 'search'): void;
  (event: 'select-action', action: PromotionAction, result?: NormalizedMemorySearchItem): void;
  (event: 'close'): void;
}>();
</script>

<template>
  <section class="dialog-section">
    <div class="search-row">
      <label class="field-block search-field">
        <span>Search existing notes</span>
        <Input
          class="border-0"
          type="text"
          placeholder="Search by title or topic"
          :value="searchQuery"
          @input="emit('update:searchQuery', ($event.target as HTMLInputElement).value.trim())"
        />
      </label>
      <Button
        type="button"
        class="search-button"
        variant="ghost"
        :disabled="searching || writing"
        @click="emit('search')"
      >
        {{ searching ? 'Searching…' : 'Search again' }}
      </Button>
    </div>

    <p v-if="infoMessage" class="info-message">{{ infoMessage }}</p>
    <p v-if="promotionError && !formError" class="form-error" role="alert">{{ promotionError }}</p>
    <p v-if="formError" class="form-error" role="alert">{{ formError }}</p>

    <div v-if="searchResults.length > 0" class="result-list">
      <article v-for="result in searchResults" :key="result.uri" class="result-card">
        <div class="result-card__body">
          <strong>{{ result.title }}</strong>
          <code class="mono">{{ result.uri }}</code>
          <p v-if="result.snippet">{{ result.snippet }}</p>
          <span class="result-score">Score {{ result.score.toFixed(2) }}</span>
        </div>
        <div class="result-card__actions">
          <Button type="button" variant="ghost" :disabled="busy" @click="emit('select-action', 'append', result)">
            Append to this
          </Button>
          <Button type="button" variant="ghost" :disabled="busy" @click="emit('select-action', 'update', result)">
            Update this
          </Button>
        </div>
      </article>
    </div>

    <div class="dialog-actions">
      <Button type="button" variant="ghost" :disabled="busy" @click="emit('close')">
        Cancel
      </Button>
      <Button type="button" :disabled="busy" @click="emit('select-action', 'create')">
        Create new
      </Button>
    </div>
  </section>
</template>
