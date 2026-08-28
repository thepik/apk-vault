# APK Vault — 产品与技术 Spec

> 版本：v0.2（已确认）· 目标平台：macOS（Apple 设计风格）
> 技术栈：Tauri 2（Rust 后端）+ Vue 3 + TypeScript + SQLite 本地存储

---

## 1. 产品定位

一个本地桌面小工具，用于集中管理多个平台的 API Key。核心动作只有两个：**录入** 和 **复制使用**。不做云同步、不做账号体系、不做团队协作。

### 1.1 用户故事

- 我拿到一个新的 API Key，想快速粘贴存进去，给它起个名字、选个平台。
- 我要用某个 Key 时，一键复制到剪贴板，不看明文也能复制。
- 我有同一个平台的多个 Key（比如 OpenAI 的个人 Key / 公司 Key），能清楚区分。
- 我重装/换机前能备份走数据（可选，见 §9 待确认项）。

---

## 2. 功能需求

### 2.1 平台管理（Platform）

| 项 | 说明 |
|---|---|
| 新增平台 | 输入名称即可（如 "OpenAI"、"DeepSeek"）；可选图标/颜色，默认按名称首字母生成彩色圆形标识（Apple 通讯录风格） |
| 编辑平台 | 改名 |
| 删除平台 | 仅当平台下无 Key 时可直接删除；有 Key 时弹确认：删除平台会连带删除其所有 Key |
| 排序 | 按使用频率自动排序（最近复制过的平台排前面），不支持手动拖拽（v1 不做） |

### 2.2 API Key 管理（KeyEntry）

| 项 | 说明 |
|---|---|
| 新增 Key | 三个字段：**名称**（必填，如 "个人主力 Key"）、**平台**（下拉选择，可在此面板内快速新建平台）、**Key 值**（必填，粘贴录入，支持多行粘贴自动去首尾空白） |
| 列表展示 | 按平台分组；每个 Key 显示：名称 + **遮罩后的 Key**（如 `sk-…a1b2`，只露前 3 后 4 位） |
| 复制 | 单击条目上的"复制"按钮（或双击条目）复制完整 Key 到剪贴板；复制后按钮短暂变为 ✓ + 触感反馈式动效；**不弹 Toast 遮挡** |
| 查看明文 | 默认遮罩；点击眼睛图标临时显示明文，再次点击或 15 秒后自动恢复遮罩 |
| 编辑 | 可改名称、换平台；Key 值本身只允许"替换"（粘贴新值），不在输入框里逐字编辑明文（防误触） |
| 删除 | 二次确认（macOS 风格红色确认按钮） |
| 使用统计 | 记录每个 Key 的 `last_copied_at` 和 `copy_count`，用于排序与展示（"3 天前用过"） |

### 2.3 搜索

- 顶部一个搜索框（macOS Spotlight 风格），实时过滤，匹配：平台名、Key 名称。
- 快捷键 `Cmd+F` 聚焦搜索框，`Esc` 清空并失焦。

### 2.4 录入与复制的交互细节（体验重点）

- **录入**：`Cmd+N` 打开新增面板（原生风格的 Sheet 弹层，从标题栏滑出），焦点自动落在名称输入框；Key 输入框支持 `Cmd+V` 直接粘贴，粘贴后自动 trim。
- **复制**：列表行 hover 时浮现复制按钮（Apple 风格的渐进披露）；复制成功有 200ms 的 ✓ 动画反馈。
- **键盘流**：`↑/↓` 在列表中移动选中，`Enter` 复制选中项，`Cmd+E` 编辑选中项。

---

## 3. 安全设计

需求：**数据加密保存，但软件自身可直接读取，启动无需登录/输密码。**

### 3.1 加密方案

- 数据库文件：SQLite，位于 `~/Library/Application Support/<bundle-id>/vault.db`。
- 敏感字段（`api_key` 值）**不落明文**：使用 **AES-256-GCM** 加密后存 BLOB，每条记录随机 nonce。
- 主密钥（32 字节）首次启动随机生成，存入 **macOS Keychain**（`kSecAttrAccessibleAfterFirstUnlock`），应用启动时从 Keychain 取出。
  - 效果：无需用户设密码；Key 串被复制到别的机器上也无法解密；Keychain 访问不弹系统授权框（同一签名应用）。
- 平台名、Key 名称、统计数据为明文（用于搜索和排序，不含机密）。

### 3.2 不做什么

- 不联网、不发任何网络请求（包括统计上报）。
- 不提供"解密导出明文"按钮（v1）；备份方案见 §9。
- 剪贴板复制后不做自动清空（macOS 无通用可靠方案，且影响体验；文档中提示风险）。

---

## 4. 数据模型（SQLite）

```sql
CREATE TABLE platform (
  id          TEXT PRIMARY KEY,          -- uuid
  name        TEXT NOT NULL UNIQUE,
  color       TEXT NOT NULL DEFAULT 'blue',  -- 预置色板 key
  created_at  INTEGER NOT NULL
);

CREATE TABLE key_entry (
  id             TEXT PRIMARY KEY,
  platform_id    TEXT NOT NULL REFERENCES platform(id) ON DELETE CASCADE,
  name           TEXT NOT NULL,
  key_encrypted  BLOB NOT NULL,          -- nonce(12B) || ciphertext || tag
  copy_count     INTEGER NOT NULL DEFAULT 0,
  last_copied_at INTEGER,
  created_at     INTEGER NOT NULL,
  updated_at     INTEGER NOT NULL
);
CREATE INDEX idx_key_entry_platform ON key_entry(platform_id);
```

排序规则：平台/Key 均按 `last_copied_at DESC NULLS LAST, created_at DESC`。

---

## 5. 界面设计（Apple 风格）

### 5.1 布局（单窗口，双栏）

```
┌──────────────────────────────────────────────┐
│  🔍 搜索框（Traffic-light 右侧，半透明标题栏） │
├────────────┬─────────────────────────────────┤
│  平台列表    │  OpenAI                    ＋   │
│  ────────  │  ─────────────────────────────  │
│  全部 (12)  │  🔑 个人主力   sk-…a1b2  ⧉ ✏ 🗑 │
│  OpenAI (4) │  🔑 公司测试   sk-…9f3c  ⧉ ✏ 🗑 │
│  DeepSeek(3)│                                │
│  Claude (2) │  DeepSeek                       │
│  ────────  │  ─────────────────────────────  │
│  ＋ 添加平台 │  🔑 本地部署   sk-…77dd  ⧉ ✏ 🗑 │
└────────────┴─────────────────────────────────┘
```

- 左侧栏：毛玻璃半透明（`NSVisualEffectView` / CSS `backdrop-filter`），SF Pro 风格字体栈。
- 选中"全部"显示所有平台分组的 Key；选中某平台只显示该平台。
- 窗口最小尺寸 720×480，记忆上次窗口尺寸/位置。

### 5.2 视觉规范

- 配色：跟随系统浅色/深色模式自动切换（`prefers-color-scheme`）。
- 强调色：系统蓝（`#0A84FF` 深色 / `#007AFF` 浅色）；删除用系统红。
- 圆角：卡片/按钮 10px，弹层 14px；字体栈：`-apple-system, SF Pro Text, PingFang SC`。
- 动效：120–200ms ease-out，仅用于反馈（复制成功、弹层进出），不滥用。

### 5.3 空状态

- 无平台时右侧显示引导插画位（简单图标+文字）："先添加一个平台，然后录入你的第一个 API Key"，附两个按钮。

---

## 6. 技术架构

```
┌─ Tauri 2 窗口 ───────────────────────────┐
│ 前端 Vue 3 + TypeScript + Vite             │
│  - 纯渲染与交互，不接触明文密钥之外的数据    │
│  - 复制走 navigator.clipboard / tauri API  │
├─ IPC (tauri commands) ────────────────────┤
│ Rust 后端                                  │
│  - commands: list/add/update/delete/copy   │
│  - crypto.rs: AES-256-GCM 加解密           │
│  - keychain.rs: 主密钥存取 (security-framework crate) │
│  - db.rs: rusqlite，启动时建表/迁移         │
└────────────────────────────────────────────┘
```

- 前端选择 **Vue 3**：单页面、组件少、模板直观，适合此类表单型工具。
- 前端**永远拿不到主密钥**；解密仅发生在后端，前端只在"显示明文"或"复制"时按需获取单条明文，用后即弃（不缓存进全局状态）。
- 依赖（Rust 侧）：`tauri`、`rusqlite`(bundled)、`aes-gcm`、`security-framework`、`uuid`、`serde`。

### 6.1 项目结构（预计）

```
apk_guard/                    （工作目录）
├── SPEC.md                   ← 本文档
├── src-tauri/                Rust 后端
│   ├── src/{main,lib,db,crypto,keychain,backup}.rs
│   │   （lib.rs 内含全部 tauri commands）
│   └── tauri.conf.json
└── src/                      Vue 前端
    ├── components/{Sidebar,KeyRow,EntrySheet,PlatformSheet,BackupSheet,ConfirmDialog}.vue
    ├── {api,types}.ts / styles.css
    └── App.vue / main.ts
```

---

## 7. 验收标准（Definition of Done）

1. 新增平台 → 新增 Key（名称+平台+粘贴 Key）→ 列表出现，明文遮罩显示。
2. 点复制 → 剪贴板内容为完整 Key，UI 有 ✓ 反馈；`copy_count`、`last_copied_at` 更新。
3. 重启应用 → 数据仍在，且不要求任何密码/登录。
4. 用文本编辑器直接打开 `vault.db` → 看不到任何 Key 明文（字段已加密）。
5. 搜索"openai" → 只显示匹配平台/Key；`Esc` 恢复。
6. 删除 Key 有二次确认；删除有 Key 的平台有连带删除警告。
7. 深色模式下界面无样式错乱。

---

## 8. 开发计划（确认 Spec 后执行）

| 步骤 | 内容 | 产出 |
|---|---|---|
| 1 | `pnpm create tauri-app` 初始化（Vue3+TS 模板），配置 bundle-id/窗口/标题栏 | 可运行空壳 |
| 2 | Rust 侧：db + crypto + keychain + commands | 后端接口可单测 |
| 3 | 前端：侧栏 + 列表 + 新增/编辑 Sheet + 复制交互 + 搜索 | 完整 UI |
| 4 | 深色模式、空状态、快捷键、窗口记忆 | 打磨 |
| 5 | 打包 `.app` / `.dmg`，本机实测 §7 全部验收项 | 可安装产物 |

---

## 9. 已确认决策（v0.2）

1. **备份/迁移**：✅ 确认包含。导出加密备份文件（`.apkvault`，内含 SQLite 转储，用导出时设置的一次性口令派生密钥加密；导入时输入口令恢复）。
2. **平台图标**：彩色首字母圆标（Apple 通讯录风格），不接官方 logo。
3. **Key 值格式校验**：不做，仅 trim 空白。
4. **应用名称**：**APK Vault**。bundle-id：`com.apkvault.app`。
5. **前端框架**：Vue 3 + TypeScript。
