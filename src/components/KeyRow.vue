<script setup lang="ts">
import { ref, onUnmounted } from "vue";
import { api } from "../api";
import type { KeyEntry } from "../types";
import { relativeTime } from "../types";

const props = defineProps<{
  entry: KeyEntry;
  selected: boolean;
}>();

const emit = defineEmits<{
  select: [];
  copied: [];
  edit: [];
  deleted: [];
}>();

const copied = ref(false);
const revealing = ref(false);
const plain = ref<string | null>(null);
const confirmingDelete = ref(false);
let revealTimer: ReturnType<typeof setTimeout> | null = null;
let copiedTimer: ReturnType<typeof setTimeout> | null = null;

async function copy() {
  try {
    await api.copyEntry(props.entry.id);
    copied.value = true;
    emit("copied");
    if (copiedTimer) clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => (copied.value = false), 1500);
  } catch (e) {
    console.error("copy failed", e);
  }
}

async function toggleReveal() {
  if (plain.value) {
    hidePlain();
    return;
  }
  revealing.value = true;
  try {
    plain.value = await api.revealEntry(props.entry.id);
    if (revealTimer) clearTimeout(revealTimer);
    revealTimer = setTimeout(hidePlain, 15000); // 15 秒后自动恢复遮罩
  } finally {
    revealing.value = false;
  }
}

function hidePlain() {
  plain.value = null;
  if (revealTimer) clearTimeout(revealTimer);
}

async function remove() {
  if (!confirmingDelete.value) {
    confirmingDelete.value = true;
    setTimeout(() => (confirmingDelete.value = false), 3000);
    return;
  }
  await api.deleteEntry(props.entry.id);
  emit("deleted");
}

onUnmounted(() => {
  if (revealTimer) clearTimeout(revealTimer);
  if (copiedTimer) clearTimeout(copiedTimer);
});
</script>

<template>
  <div
    class="key-row"
    :class="{ selected }"
    @click="emit('select')"
    @dblclick="copy"
  >
    <span class="name" :title="entry.name">{{ entry.name }}</span>
    <span class="masked">{{ plain ?? entry.masked_key }}</span>
    <span class="meta">{{ relativeTime(entry.last_copied_at) }}</span>

    <span class="actions">
      <button class="icon-btn copy-btn" :title="copied ? '已复制' : '复制（或双击行）'" @click.stop="copy">
        <svg v-if="copied" class="check" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6L9 17l-5-5"/></svg>
        <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
      </button>
      <button class="icon-btn" :title="plain ? '隐藏明文' : '显示明文（15 秒后自动隐藏）'" @click.stop="toggleReveal">
        <svg v-if="plain" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19M14.12 14.12a3 3 0 1 1-4.24-4.24"/><path d="M1 1l22 22"/></svg>
        <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
      </button>
      <button class="icon-btn" title="编辑" @click.stop="emit('edit')">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/></svg>
      </button>
      <button
        class="icon-btn danger"
        :title="confirmingDelete ? '再次点击确认删除' : '删除'"
        :style="confirmingDelete ? 'color: var(--danger); background: rgba(255,59,48,0.12)' : ''"
        @click.stop="remove"
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2m3 0v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/></svg>
      </button>
    </span>
  </div>
</template>
