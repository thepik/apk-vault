<script setup lang="ts">
import type { Platform } from "../types";
import { PLATFORM_COLORS } from "../types";

defineProps<{
  platforms: Platform[];
  selectedId: string | "all";
  totalCount: number;
}>();

const emit = defineEmits<{
  select: [id: string | "all"];
  addPlatform: [];
  openBackup: [];
  renamePlatform: [p: Platform];
  deletePlatform: [p: Platform];
}>();

function initial(name: string) {
  return name.trim().charAt(0).toUpperCase() || "?";
}
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-header" data-tauri-drag-region>
      <div class="app-title">APK Vault</div>
    </div>
    <div class="sidebar-section">平台</div>
    <div class="sidebar-list">
      <button
        class="sidebar-item"
        :class="{ active: selectedId === 'all' }"
        @click="emit('select', 'all')"
      >
        <span class="platform-dot" style="background: var(--text-3)">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="10" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>
        </span>
        全部
        <span class="count">{{ totalCount }}</span>
      </button>

      <button
        v-for="p in platforms"
        :key="p.id"
        class="sidebar-item"
        :class="{ active: selectedId === p.id }"
        @click="emit('select', p.id)"
        @contextmenu.prevent="emit('renamePlatform', p)"
        :title="`${p.name}（右键重命名）`"
      >
        <span class="platform-dot" :style="{ background: PLATFORM_COLORS[p.color] || PLATFORM_COLORS.blue }">
          {{ initial(p.name) }}
        </span>
        <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap">{{ p.name }}</span>
        <span class="count">{{ p.key_count }}</span>
      </button>
    </div>

    <div class="sidebar-footer">
      <button class="icon-btn" title="添加平台" @click="emit('addPlatform')">
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>
      </button>
      <button
        v-if="selectedId !== 'all'"
        class="icon-btn danger"
        title="删除当前平台"
        @click="emit('deletePlatform', platforms.find(p => p.id === selectedId)!)"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2m3 0v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/></svg>
      </button>
      <div style="flex: 1" />
      <button class="icon-btn" title="加密备份 / 恢复" @click="emit('openBackup')">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3"/></svg>
      </button>
    </div>
  </aside>
</template>
