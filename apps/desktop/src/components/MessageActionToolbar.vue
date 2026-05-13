<script setup lang="ts">
import { computed, ref } from 'vue';
import { Copy, Pencil, RotateCcw } from 'lucide-vue-next';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';

const props = defineProps<{
  messageRole: 'user' | 'assistant' | string;
  messageText: string;
  disabled: boolean;
}>();

const emit = defineEmits<{
  copy: [];
  regenerate: [];
  edit: [{ text: string }];
}>();

const confirmOpen = ref(false);
const canEdit = computed(() => props.messageRole === 'user');

function copyWithCommand(text: string): boolean {
  let copied = false;
  const listener = (event: ClipboardEvent) => {
    event.clipboardData?.setData('text/plain', text);
    event.preventDefault();
    copied = true;
  };
  document.addEventListener('copy', listener, { once: true });
  const commandSucceeded = document.execCommand('copy');
  document.removeEventListener('copy', listener);
  return commandSucceeded && copied;
}

async function copyMessage(): Promise<void> {
  if (props.disabled) return;
  try {
    await navigator.clipboard.writeText(props.messageText);
  } catch {
    if (!copyWithCommand(props.messageText)) {
      toast.error('Unable to copy message.');
      return;
    }
  }
  toast.success('Copied message to clipboard.');
  emit('copy');
}

function editMessage(): void {
  if (!props.disabled) emit('edit', { text: props.messageText });
}

function confirmRegenerate(): void {
  if (!props.disabled) emit('regenerate');
}
</script>

<template>
  <TooltipProvider :delay-duration="150">
    <div class="flex items-center gap-1" aria-label="Message actions">
      <Tooltip>
        <TooltipTrigger as-child>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            class="size-7 text-text-secondary"
            :disabled="disabled"
            aria-label="Copy message"
            @click="copyMessage"
          >
            <Copy class="size-4" aria-hidden="true" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Copy</TooltipContent>
      </Tooltip>

      <AlertDialog v-model:open="confirmOpen">
        <Tooltip>
          <TooltipTrigger as-child>
            <AlertDialogTrigger as-child>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                class="size-7 text-text-secondary"
                :disabled="disabled"
                aria-label="Regenerate from here"
              >
                <RotateCcw class="size-4" aria-hidden="true" />
              </Button>
            </AlertDialogTrigger>
          </TooltipTrigger>
          <TooltipContent>Regenerate from here</TooltipContent>
        </Tooltip>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Regenerate from here?</AlertDialogTitle>
            <AlertDialogDescription>
              Discard the messages after this one and regenerate? This cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction variant="destructive" @click="confirmRegenerate">
              Discard
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <Tooltip v-if="canEdit">
        <TooltipTrigger as-child>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            class="size-7 text-text-secondary"
            :disabled="disabled"
            aria-label="Edit and resubmit"
            @click="editMessage"
          >
            <Pencil class="size-4" aria-hidden="true" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Edit and resubmit</TooltipContent>
      </Tooltip>
    </div>
  </TooltipProvider>
</template>
