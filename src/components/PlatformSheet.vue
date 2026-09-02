<script setup lang="ts">
import { ref, onMounted } from "vue";
import { api } from "../api";
import { PLATFORM_COLORS } from "../types";
import type { Platform } from "../types";

const props = defineProps<{
  /** 重命名模式传入已有平台 */
  platform?: Platform;
}>();

const emit = defineEmits<{
  close: [];
  saved: [];
}>();

const isRename = !!props.platform;
const name = ref(props.platform?.name ?? "");
const color = ref(props.platform?.color ?? "blue");
const endpointOpenai = ref(props.platform?.endpoint_openai ?? "");
const endpointAnthropic = ref(props.platform?.endpoint_anthropic ?? "");
const error = ref("");
const input = ref<HTMLInputElement | null>(null);

onMounted(() => input.value?.focus());

async function submit() {
  error.value = "";
  if (!name.value.trim()) {
    error.value = "请填写平台名称";
    return;
  }
  try {
    if (isRename) {
      await api.renamePlatform(props.platform!.id, name.value);
      await api.setPlatformEndpoints(
        props.platform!.id,
        endpointOpenai.value.trim(),
        endpointAnthropic.value.trim(),
      );
    } else {
      await api.addPlatform(
        name.value,
        color.value,
        endpointOpenai.value.trim(),
        endpointAnthropic.value.trim(),
      );
    }
    emit("saved");
    emit("close");
  } catch (e) {
    error.value = String(e);
  }
}
</script>

<template>
  <div class="sheet-backdrop" @mousedown.self="emit('close')">
    <div class="sheet" style="width: 340px" @keydown.esc.stop="emit('close')" @keydown.enter.prevent="submit">
      <h2>{{ isRename ? "编辑平台" : "添加平台" }}</h2>
      <div class="field">
        <label>名称</label>
        <input ref="input" v-model="name" placeholder="例如：OpenAI" />
      </div>
      <div v-if="!isRename" class="field">
        <label>标识颜色</label>
        <div class="color-row">
          <span
            v-for="(css, key) in PLATFORM_COLORS"
            :key="key"
            class="color-swatch"
            :class="{ active: color === key }"
            :style="{ background: css }"
            @click="color = key"
          />
        </div>
      </div>
      <div class="field">
        <label>OpenAI 协议地址（可选）</label>
        <input
          v-model="endpointOpenai"
          class="mono"
          placeholder="https://api.openai.com/v1"
          spellcheck="false"
        />
      </div>
      <div class="field">
        <label>Anthropic 协议地址（可选）</label>
        <input
          v-model="endpointAnthropic"
          class="mono"
          placeholder="https://api.anthropic.com"
          spellcheck="false"
        />
        <div class="hint">留空表示未设置；需以 http:// 或 https:// 开头。</div>
      </div>
      <p class="error">{{ error }}</p>
      <div class="footer">
        <button class="btn" @click="emit('close')">取消</button>
        <button class="btn primary" @click="submit">{{ isRename ? "保存" : "添加" }}</button>
      </div>
    </div>
  </div>
</template>
