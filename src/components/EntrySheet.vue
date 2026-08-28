<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { api } from "../api";
import type { KeyEntry, Platform } from "../types";
import { PLATFORM_COLORS } from "../types";

const props = defineProps<{
  platforms: Platform[];
  /** 编辑模式传入已有 entry */
  entry?: KeyEntry;
  defaultPlatformId?: string;
}>();

const emit = defineEmits<{
  close: [];
  saved: [];
}>();

const isEdit = computed(() => !!props.entry);

const name = ref(props.entry?.name ?? "");
const platformId = ref(props.entry?.platform_id ?? props.defaultPlatformId ?? props.platforms[0]?.id ?? "");
const keyValue = ref("");
const newPlatformName = ref("");
const showNewPlatform = ref(false);
const error = ref("");
const saving = ref(false);
const nameInput = ref<HTMLInputElement | null>(null);

onMounted(() => nameInput.value?.focus());

async function quickAddPlatform() {
  const n = newPlatformName.value.trim();
  if (!n) return;
  error.value = "";
  try {
    const p = await api.addPlatform(n);
    emit("saved"); // 让父级刷新平台列表
    platformId.value = p.id;
    showNewPlatform.value = false;
    newPlatformName.value = "";
  } catch (e) {
    error.value = String(e);
  }
}

async function submit() {
  error.value = "";
  if (!name.value.trim()) {
    error.value = "请填写名称";
    return;
  }
  if (!platformId.value) {
    error.value = "请选择平台";
    return;
  }
  if (!isEdit.value && !keyValue.value.trim()) {
    error.value = "请粘贴 Key 值";
    return;
  }
  saving.value = true;
  try {
    if (isEdit.value && props.entry) {
      await api.updateEntry(
        props.entry.id,
        name.value,
        platformId.value,
        keyValue.value.trim() ? keyValue.value : undefined,
      );
    } else {
      await api.addEntry(platformId.value, name.value, keyValue.value);
    }
    emit("saved");
    emit("close");
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

const colors = Object.keys(PLATFORM_COLORS);
void colors;
</script>

<template>
  <div class="sheet-backdrop" @mousedown.self="emit('close')">
    <div class="sheet" @keydown.esc.stop="emit('close')" @keydown.meta.enter.prevent="submit">
      <h2>{{ isEdit ? "编辑 Key" : "新增 API Key" }}</h2>

      <div class="field">
        <label>名称</label>
        <input ref="nameInput" v-model="name" placeholder="例如：个人主力 Key" @keydown.enter="submit" />
      </div>

      <div class="field">
        <label>平台</label>
        <div style="display: flex; gap: 8px">
          <select v-model="platformId" style="flex: 1" :disabled="showNewPlatform">
            <option v-for="p in platforms" :key="p.id" :value="p.id">{{ p.name }}</option>
          </select>
          <button class="btn" style="flex-shrink: 0" @click="showNewPlatform = !showNewPlatform">
            {{ showNewPlatform ? "选择已有" : "新建平台" }}
          </button>
        </div>
        <div v-if="showNewPlatform" style="display: flex; gap: 8px; margin-top: 8px">
          <input v-model="newPlatformName" placeholder="新平台名称" @keydown.enter="quickAddPlatform" />
          <button class="btn primary" style="flex-shrink: 0" :disabled="!newPlatformName.trim()" @click="quickAddPlatform">添加</button>
        </div>
      </div>

      <div class="field">
        <label>{{ isEdit ? "替换 Key 值（留空则不修改）" : "Key 值" }}</label>
        <textarea
          v-model="keyValue"
          rows="2"
          :placeholder="isEdit ? '粘贴新值以替换' : '在此粘贴（Cmd+V）'"
          class="mono"
          spellcheck="false"
          autocorrect="off"
          autocapitalize="off"
        />
        <div class="hint">保存后加密存储，此处不会保留明文。</div>
      </div>

      <p class="error">{{ error }}</p>

      <div class="footer">
        <button class="btn" @click="emit('close')">取消</button>
        <button class="btn primary" :disabled="saving" @click="submit">
          {{ isEdit ? "保存" : "添加" }}
        </button>
      </div>
    </div>
  </div>
</template>
