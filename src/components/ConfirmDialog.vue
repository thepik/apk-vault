<script setup lang="ts">
defineProps<{
  title: string;
  message: string;
  confirmText?: string;
}>();

const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();
</script>

<template>
  <div class="sheet-backdrop" @mousedown.self="emit('cancel')">
    <div class="sheet confirm-sheet" @keydown.esc.stop="emit('cancel')">
      <h2>{{ title }}</h2>
      <p>{{ message }}</p>
      <div class="footer">
        <button class="btn" @click="emit('cancel')">取消</button>
        <button class="btn danger" @click="emit('confirm')">{{ confirmText ?? "删除" }}</button>
      </div>
    </div>
  </div>
</template>
