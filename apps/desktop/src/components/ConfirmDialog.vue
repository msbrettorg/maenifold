<script setup lang="ts">
import { storeToRefs } from 'pinia';
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from './ui/alert-dialog';
import { Button } from './ui/button';
import { useConfirmDialogStore } from '../stores/confirm-dialog';

const confirmDialogStore = useConfirmDialogStore();
const {
  open,
  title,
  description,
} = storeToRefs(confirmDialogStore);
</script>

<template>
  <AlertDialog :open="open">
    <AlertDialogContent class="sm:max-w-[28rem]">
      <AlertDialogHeader>
        <AlertDialogTitle>{{ title }}</AlertDialogTitle>
        <AlertDialogDescription v-if="description">
          {{ description }}
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <Button variant="outline" @click="confirmDialogStore.handleCancel()">
          Cancel
        </Button>
        <Button @click="confirmDialogStore.handleConfirm()">
          Continue
        </Button>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
