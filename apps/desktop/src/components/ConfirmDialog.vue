<script setup lang="ts">
import { storeToRefs } from 'pinia';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from './ui/alert-dialog';
import { useConfirmDialogStore } from '../stores/confirm-dialog';

const confirmDialogStore = useConfirmDialogStore();
const {
  open,
  title,
  description,
} = storeToRefs(confirmDialogStore);
</script>

<template>
  <AlertDialog :open="open" @update:open="(nextOpen) => !nextOpen && confirmDialogStore.handleCancel()">
    <AlertDialogContent class="sm:max-w-[28rem]">
      <AlertDialogHeader>
        <AlertDialogTitle>{{ title }}</AlertDialogTitle>
        <AlertDialogDescription v-if="description">
          {{ description }}
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel @click="confirmDialogStore.handleCancel()">
          Cancel
        </AlertDialogCancel>
        <AlertDialogAction @click="confirmDialogStore.handleConfirm()">
          Continue
        </AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
