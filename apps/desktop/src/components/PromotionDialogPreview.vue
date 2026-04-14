<script setup lang="ts">
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
        <input
          class="field-input"
          type="text"
          placeholder="Search by title or topic"
          :value="searchQuery"
          @input="emit('update:searchQuery', ($event.target as HTMLInputElement).value.trim())"
        />
      </label>
      <button
        type="button"
        class="btn btn-ghost search-button"
        :disabled="searching || writing"
        @click="emit('search')"
      >
        {{ searching ? 'Searching…' : 'Search again' }}
      </button>
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
          <button type="button" class="btn btn-ghost" :disabled="busy" @click="emit('select-action', 'append', result)">
            Append to this
          </button>
          <button type="button" class="btn btn-ghost" :disabled="busy" @click="emit('select-action', 'update', result)">
            Update this
          </button>
        </div>
      </article>
    </div>

    <div class="dialog-actions dialog-actions--start">
      <button type="button" class="btn btn-primary" :disabled="busy" @click="emit('select-action', 'create')">
        Create new
      </button>
      <button type="button" class="btn btn-ghost" :disabled="busy" @click="emit('close')">
        Cancel
      </button>
    </div>
  </section>
</template>
