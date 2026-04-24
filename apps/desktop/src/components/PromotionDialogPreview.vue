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
  <section class="grid gap-4 px-4 pb-4">
    <div class="flex gap-3 items-end max-[720px]:flex-col">
      <label class="grid min-w-0 flex-1 gap-[0.35rem] text-xs text-text-secondary">
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
        class="shrink-0"
        variant="ghost"
        :disabled="searching || writing"
        @click="emit('search')"
      >
        {{ searching ? 'Searching…' : 'Search again' }}
      </Button>
    </div>

    <p v-if="infoMessage" class="text-text-secondary">{{ infoMessage }}</p>
    <p v-if="promotionError && !formError" class="m-0 text-status-abandoned text-xs" role="alert">{{ promotionError }}</p>
    <p v-if="formError" class="m-0 text-status-abandoned text-xs" role="alert">{{ formError }}</p>

    <div v-if="searchResults.length > 0" class="grid gap-4">
      <article v-for="result in searchResults" :key="result.uri" class="flex items-start justify-between gap-3 rounded-[4px] bg-surface p-3.5 max-[720px]:flex-col">
        <div class="grid gap-[0.35rem] min-w-0">
          <strong>{{ result.title }}</strong>
          <code class="font-mono break-all">{{ result.uri }}</code>
          <p v-if="result.snippet" class="m-0 whitespace-pre-wrap text-text-secondary">{{ result.snippet }}</p>
          <span class="text-text-secondary">Score {{ result.score.toFixed(2) }}</span>
        </div>
        <div class="flex gap-3 flex-wrap max-[720px]:justify-stretch">
          <Button type="button" variant="ghost" :disabled="busy" @click="emit('select-action', 'append', result)">
            Append to this
          </Button>
          <Button type="button" variant="ghost" :disabled="busy" @click="emit('select-action', 'update', result)">
            Update this
          </Button>
        </div>
      </article>
    </div>

    <div class="flex flex-wrap items-center gap-3 max-[720px]:justify-stretch [&>:first-child:not(:only-child)]:mr-auto">
      <Button type="button" variant="ghost" :disabled="busy" @click="emit('close')">
        Cancel
      </Button>
      <Button type="button" :disabled="busy" @click="emit('select-action', 'create')">
        Create new
      </Button>
    </div>
  </section>
</template>
