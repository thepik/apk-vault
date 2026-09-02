<script setup lang="ts">
import { computed, onUnmounted, ref } from "vue";
import { api } from "../api";
import type { Platform } from "../types";

/** 平台视图顶部的调用地址卡片：OpenAI / Anthropic 两个协议各一条 */
const props = defineProps<{
  platform: Platform;
}>();

const emit = defineEmits<{
  edit: [];
}>();

const rows = computed(() => [
  { key: "openai", label: "OpenAI 协议", url: props.platform.endpoint_openai },
  { key: "anthropic", label: "Anthropic 协议", url: props.platform.endpoint_anthropic },
]);

const copiedKey = ref<string | null>(null);
let timer: ReturnType<typeof setTimeout> | null = null;

async function copy(row: { key: string; url: string }) {
  try {
    await api.copyText(row.url);
    copiedKey.value = row.key;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => (copiedKey.value = null), 1500);
  } catch (e) {
    console.error("copy failed", e);
  }
}

onUnmounted(() => {
  if (timer) clearTimeout(timer);
});
</script>

<template>
  <div class="endpoints-wrap">
    <div class="endpoints-card">
      <div class="endpoints-head">
        <span>调用地址</span>
        <button class="icon-btn" title="编辑调用地址" @click="emit('edit')">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/></svg>
        </button>
      </div>
      <div
        v-for="r in rows"
        :key="r.key"
        class="endpoint-row"
        :class="{ unset: !r.url }"
        @click="!r.url && emit('edit')"
      >
        <span class="endpoint-tag">{{ r.label }}</span>
        <span class="endpoint-url" :class="{ unset: !r.url }" :title="r.url || undefined">
          {{ r.url || "未设置，点击填写" }}
        </span>
        <button
          v-if="r.url"
          class="icon-btn copy-btn"
          :title="copiedKey === r.key ? '已复制' : '复制地址'"
          @click="copy(r)"
        >
          <svg v-if="copiedKey === r.key" class="check" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6L9 17l-5-5"/></svg>
          <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
        </button>
        <button v-else class="icon-btn" title="填写地址" @click.stop="emit('edit')">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>
        </button>
      </div>
    </div>
  </div>
</template>
