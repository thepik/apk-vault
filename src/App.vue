<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
import { api } from "./api";
import type { KeyEntry, Platform } from "./types";
import { PLATFORM_COLORS } from "./types";
import Sidebar from "./components/Sidebar.vue";
import KeyRow from "./components/KeyRow.vue";
import EntrySheet from "./components/EntrySheet.vue";
import PlatformSheet from "./components/PlatformSheet.vue";
import BackupSheet from "./components/BackupSheet.vue";
import ConfirmDialog from "./components/ConfirmDialog.vue";
import EndpointBar from "./components/EndpointBar.vue";

const platforms = ref<Platform[]>([]);
const entries = ref<KeyEntry[]>([]);
const selectedPlatform = ref<string | "all">("all");
const search = ref("");
const selectedEntryId = ref<string | null>(null);
const searchInput = ref<HTMLInputElement | null>(null);

// Sheet 状态
const showEntrySheet = ref(false);
const editingEntry = ref<KeyEntry | undefined>(undefined);
const showPlatformSheet = ref(false);
const renamingPlatform = ref<Platform | undefined>(undefined);
const showBackup = ref(false);
const deletingPlatform = ref<Platform | null>(null);

async function refresh() {
  const state = await api.listState();
  platforms.value = state.platforms;
  entries.value = state.entries;
}

onMounted(async () => {
  await refresh();
  window.addEventListener("keydown", onKeydown);
});
onUnmounted(() => window.removeEventListener("keydown", onKeydown));

const anySheetOpen = computed(
  () => showEntrySheet.value || showPlatformSheet.value || showBackup.value || !!deletingPlatform.value,
);

// ---------- 过滤与分组 ----------

const query = computed(() => search.value.trim().toLowerCase());

const visibleEntries = computed(() => {
  let list = entries.value;
  if (selectedPlatform.value !== "all") {
    list = list.filter((e) => e.platform_id === selectedPlatform.value);
  }
  if (query.value) {
    list = list.filter((e) => {
      const p = platforms.value.find((x) => x.id === e.platform_id);
      return (
        e.name.toLowerCase().includes(query.value) ||
        (p && p.name.toLowerCase().includes(query.value))
      );
    });
  }
  return list;
});

interface Group {
  platform: Platform;
  entries: KeyEntry[];
}

const groups = computed<Group[]>(() => {
  const byId = new Map(platforms.value.map((p) => [p.id, p]));
  const map = new Map<string, KeyEntry[]>();
  for (const e of visibleEntries.value) {
    if (!map.has(e.platform_id)) map.set(e.platform_id, []);
    map.get(e.platform_id)!.push(e);
  }
  const result: Group[] = [];
  for (const [pid, list] of map) {
    const platform = byId.get(pid);
    if (platform) result.push({ platform, entries: list });
  }
  // 组排序：沿用平台排序（最近使用优先）
  const order = new Map(platforms.value.map((p, i) => [p.id, i]));
  result.sort((a, b) => (order.get(a.platform.id) ?? 0) - (order.get(b.platform.id) ?? 0));
  return result;
});

const totalCount = computed(() => entries.value.length);

// 当前选中平台（「全部」时为 undefined）
const endpointPlatform = computed<Platform | undefined>(() =>
  selectedPlatform.value === "all"
    ? undefined
    : platforms.value.find((p) => p.id === selectedPlatform.value),
);

// ---------- 键盘 ----------

function onKeydown(e: KeyboardEvent) {
  const meta = e.metaKey || e.ctrlKey;

  if (meta && e.key.toLowerCase() === "f") {
    e.preventDefault();
    searchInput.value?.focus();
    searchInput.value?.select();
    return;
  }
  if (meta && e.key.toLowerCase() === "n") {
    if (!anySheetOpen.value) {
      e.preventDefault();
      openAddEntry();
    }
    return;
  }
  if (e.key === "Escape") {
    if (document.activeElement === searchInput.value) {
      search.value = "";
      searchInput.value?.blur();
    }
    return;
  }
  if (anySheetOpen.value) return;
  // 输入框聚焦时不劫持
  const tag = (document.activeElement as HTMLElement)?.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

  const list = visibleEntries.value;
  if (!list.length) return;
  const idx = list.findIndex((e) => e.id === selectedEntryId.value);

  if (e.key === "ArrowDown" || e.key === "ArrowUp") {
    e.preventDefault();
    const next =
      e.key === "ArrowDown"
        ? idx < list.length - 1
          ? idx + 1
          : 0
        : idx > 0
          ? idx - 1
          : list.length - 1;
    selectedEntryId.value = list[next].id;
    nextTick(() => {
      document
        .querySelector(".key-row.selected")
        ?.scrollIntoView({ block: "nearest" });
    });
  } else if (e.key === "Enter" && idx >= 0) {
    e.preventDefault();
    copyById(list[idx].id);
  } else if (meta && e.key.toLowerCase() === "e" && idx >= 0) {
    e.preventDefault();
    editingEntry.value = list[idx];
    showEntrySheet.value = true;
  }
}

async function copyById(id: string) {
  try {
    await api.copyEntry(id);
    await refresh();
  } catch (e) {
    console.error(e);
  }
}

// ---------- 操作 ----------

function openAddEntry() {
  editingEntry.value = undefined;
  showEntrySheet.value = true;
}

function openAddPlatform() {
  renamingPlatform.value = undefined;
  showPlatformSheet.value = true;
}

function openRenamePlatform(p: Platform) {
  renamingPlatform.value = p;
  showPlatformSheet.value = true;
}

function requestDeletePlatform(p: Platform) {
  if (!p) return;
  deletingPlatform.value = p;
}

async function confirmDeletePlatform() {
  if (!deletingPlatform.value) return;
  await api.deletePlatform(deletingPlatform.value.id);
  if (selectedPlatform.value === deletingPlatform.value.id) {
    selectedPlatform.value = "all";
  }
  deletingPlatform.value = null;
  await refresh();
}

async function onEntryCopied() {
  // 轻量刷新使用统计排序
  await refresh();
}
</script>

<template>
  <div class="layout">
    <Sidebar
      :platforms="platforms"
      :selected-id="selectedPlatform"
      :total-count="totalCount"
      @select="selectedPlatform = $event"
      @add-platform="openAddPlatform"
      @rename-platform="openRenamePlatform"
      @delete-platform="requestDeletePlatform"
      @open-backup="showBackup = true"
    />

    <main class="main">
      <div class="toolbar" data-tauri-drag-region>
        <div class="search-wrap" @mousedown.stop>
          <svg class="search-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg>
          <input
            ref="searchInput"
            v-model="search"
            class="search-input"
            placeholder="搜索平台或名称（⌘F）"
          />
        </div>
        <div class="toolbar-spacer" />
        <button class="btn primary" @mousedown.stop @click="openAddEntry" :disabled="!platforms.length">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>
          新增 Key
        </button>
      </div>

      <!-- 空状态 -->
      <div v-if="!platforms.length" class="empty" data-tauri-drag-region>
        <div class="glyph">🔑</div>
        <h3>管理你的所有 API Key</h3>
        <p>先添加一个平台（如 OpenAI、DeepSeek），然后粘贴录入你的第一个 Key。所有数据加密保存在本机。</p>
        <div class="row">
          <button class="btn" @mousedown.stop @click="openAddPlatform">添加平台</button>
        </div>
      </div>
      <template v-else>
        <!-- 平台视图顶部：调用地址（「全部」视图不展示；未设置时也可点击填写） -->
        <EndpointBar
          v-if="endpointPlatform"
          :platform="endpointPlatform"
          @edit="openRenamePlatform(endpointPlatform)"
        />

        <div v-if="!visibleEntries.length" class="empty" data-tauri-drag-region>
          <div class="glyph">🗂️</div>
          <h3>{{ query ? "没有匹配的结果" : "这里还没有 Key" }}</h3>
          <p>{{ query ? "换个关键词试试，或清空搜索。" : "点击右上角「新增 Key」，粘贴你的第一个 API Key。" }}</p>
          <div class="row">
            <button v-if="query" class="btn" @mousedown.stop @click="search = ''">清空搜索</button>
            <button v-else class="btn primary" @mousedown.stop @click="openAddEntry">新增 Key</button>
          </div>
        </div>

        <!-- Key 列表 -->
        <div v-else class="key-scroll">
          <div v-for="g in groups" :key="g.platform.id">
            <div class="group-header">
              <span
                class="platform-dot"
                :style="{ background: PLATFORM_COLORS[g.platform.color] || PLATFORM_COLORS.blue }"
              >
                {{ g.platform.name.charAt(0).toUpperCase() }}
              </span>
              {{ g.platform.name }}
              <span class="count">{{ g.entries.length }}</span>
            </div>
            <div class="key-card">
              <KeyRow
                v-for="e in g.entries"
                :key="e.id"
                :entry="e"
                :selected="selectedEntryId === e.id"
                @select="selectedEntryId = e.id"
                @copied="onEntryCopied"
                @edit="editingEntry = e; showEntrySheet = true"
                @deleted="refresh"
              />
            </div>
          </div>
        </div>
      </template>
    </main>
  </div>

  <!-- 弹层 -->
  <EntrySheet
    v-if="showEntrySheet"
    :platforms="platforms"
    :entry="editingEntry"
    :default-platform-id="selectedPlatform !== 'all' ? selectedPlatform : undefined"
    @close="showEntrySheet = false"
    @saved="refresh"
  />
  <PlatformSheet
    v-if="showPlatformSheet"
    :platform="renamingPlatform"
    @close="showPlatformSheet = false"
    @saved="refresh"
  />
  <BackupSheet
    v-if="showBackup"
    @close="showBackup = false"
    @imported="refresh"
  />
  <ConfirmDialog
    v-if="deletingPlatform"
    :title="`删除平台「${deletingPlatform.name}」？`"
    :message="deletingPlatform.key_count > 0
      ? `该平台下的 ${deletingPlatform.key_count} 个 Key 会被一并删除，且无法恢复。`
      : '删除后无法恢复。'"
    confirm-text="删除"
    @confirm="confirmDeletePlatform"
    @cancel="deletingPlatform = null"
  />
</template>
