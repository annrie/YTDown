<script setup lang="ts">
defineProps<{ message: string }>()
</script>

<template>
  <div class="px-3 py-2 rounded-md bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-xs">
    {{ message }}
  </div>
</template>
