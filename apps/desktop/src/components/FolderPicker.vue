<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

const CREATE_NEW_FOLDER = '__create_new_folder__';

const props = withDefaults(defineProps<{
  modelValue: string;
  availableFolders: string[];
  placeholder?: string;
}>(), {
  placeholder: 'Choose folder',
});

const emit = defineEmits<{
  'update:modelValue': [value: string];
}>();

const creating = ref(false);
const draft = ref('');
const normalizedDraft = computed(() => draft.value.trim().replace(/^\/+|\/+$/g, ''));
const pathError = computed(() => {
  const raw = draft.value.trim();
  if (!raw) return 'Folder path is required.';
  if (raw !== normalizedDraft.value) return 'Folder path cannot start or end with a slash.';
  if (raw.split('/').some((segment) => !segment || segment === '..')) {
    return 'Folder path cannot contain empty segments or ...';
  }
  return null;
});

watch(() => props.modelValue, (value) => {
  if (!creating.value) draft.value = value;
});

function chooseFolder(value: unknown): void {
  const next = String(value);
  if (next === CREATE_NEW_FOLDER) {
    draft.value = props.modelValue;
    creating.value = true;
    return;
  }
  creating.value = false;
  emit('update:modelValue', next);
}

function useNewFolder(): void {
  if (pathError.value) return;
  emit('update:modelValue', normalizedDraft.value);
  creating.value = false;
}
</script>

<template>
  <div class="grid gap-2" data-testid="folder-picker">
    <Select :model-value="props.modelValue" @update:model-value="chooseFolder">
      <SelectTrigger class="w-full border-border-subtle bg-bg-app" data-testid="folder-picker-trigger">
        <SelectValue :placeholder="placeholder" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem
          v-for="folder in availableFolders"
          :key="folder"
          :value="folder"
          @click="chooseFolder(folder)"
        >
          {{ folder }}
        </SelectItem>
        <SelectItem :value="CREATE_NEW_FOLDER" @click="chooseFolder(CREATE_NEW_FOLDER)">
          Create new folder…
        </SelectItem>
      </SelectContent>
    </Select>

    <div v-if="creating" class="grid gap-2">
      <Input
        v-model="draft"
        data-testid="folder-picker-new-input"
        class="border-border-subtle bg-bg-app"
        placeholder="new/folder"
      />
      <p v-if="pathError" class="text-tiny text-danger" data-testid="folder-picker-error">{{ pathError }}</p>
      <p v-else class="text-tiny text-text-tertiary">
        Will be created at: <code>{{ normalizedDraft }}</code>
      </p>
      <Button type="button" size="sm" :disabled="!!pathError" data-testid="folder-picker-use-new" @click="useNewFolder">
        Use this folder
      </Button>
    </div>
  </div>
</template>
