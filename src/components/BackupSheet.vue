<script setup lang="ts">
import { ref } from "vue";
import { api } from "../api";

const emit = defineEmits<{
  close: [];
  imported: [];
}>();

const tab = ref<"export" | "import">("export");
const passphrase = ref("");
const busy = ref(false);
const error = ref("");
const message = ref("");

async function doExport() {
  error.value = "";
  message.value = "";
  if (passphrase.value.length < 4) {
    error.value = "口令至少 4 位";
    return;
  }
  busy.value = true;
  try {
    const done = await api.exportBackup(passphrase.value);
    if (done) {
      message.value = "备份已导出。请妥善保管文件与口令，两者缺一不可。";
      passphrase.value = "";
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function doImport() {
  error.value = "";
  message.value = "";
  if (!passphrase.value) {
    error.value = "请输入导出时设置的口令";
    return;
  }
  busy.value = true;
  try {
    const count = await api.importBackup(passphrase.value);
    if (count !== null) {
      message.value = `已恢复 ${count} 个 Key。`;
      passphrase.value = "";
      emit("imported");
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="sheet-backdrop" @mousedown.self="emit('close')">
    <div class="sheet" style="width: 380px" @keydown.esc.stop="emit('close')">
      <h2>加密备份</h2>
      <div style="display: flex; gap: 8px; margin-bottom: 14px">
        <button class="btn" :class="{ primary: tab === 'export' }" style="flex: 1" @click="tab = 'export'">导出备份</button>
        <button class="btn" :class="{ primary: tab === 'import' }" style="flex: 1" @click="tab = 'import'">导入恢复</button>
      </div>

      <div class="field">
        <label>{{ tab === "export" ? "设置备份口令（至少 4 位）" : "输入导出时的口令" }}</label>
        <input
          v-model="passphrase"
          type="password"
          :placeholder="tab === 'export' ? '换机恢复时需要此口令' : ''"
          @keydown.enter="tab === 'export' ? doExport() : doImport()"
        />
        <div class="hint">
          {{ tab === "export"
            ? "备份文件内不含任何明文，用此口令单独加密（与系统 Keychain 无关，可跨机器恢复）。"
            : "导入会按平台名称合并，同名平台的 Key 会追加到已有平台下。" }}
        </div>
      </div>

      <p v-if="error" class="error">{{ error }}</p>
      <p v-if="message" style="font-size: 12px; color: var(--p-green); margin: 4px 0 0">{{ message }}</p>

      <div class="footer">
        <button class="btn" @click="emit('close')">关闭</button>
        <button class="btn primary" :disabled="busy" @click="tab === 'export' ? doExport() : doImport()">
          {{ tab === "export" ? "选择位置并导出" : "选择文件并导入" }}
        </button>
      </div>
    </div>
  </div>
</template>
