# APK Vault

本地加密 API Key 管理工具（macOS · Tauri 2 + Vue 3 + SQLite）。

## 功能

- 按平台分组管理多个 API Key（平台可增删改名，彩色首字母圆标）
- 粘贴录入（名称 + 选平台 + 粘贴 Key，可就地新建平台）
- 一键复制到剪贴板（双击行 / 按钮 / Enter），复制后 ✓ 反馈，默认遮罩 `sk-…a1b2`
- 搜索（⌘F）、键盘流（↑↓ 选择、Enter 复制、⌘E 编辑、⌘N 新增）
- 使用统计驱动排序：最近用过的平台/Key 自动靠前
- AES-256-GCM 字段加密，主密钥存 macOS Keychain，启动免登录
- 加密备份导出/导入（`.apkvault`，一次性口令 + Argon2，可跨机恢复）
- 浅色/深色跟随系统

## 开发

```bash
pnpm install
pnpm tauri dev
```

## 测试与打包

```bash
cd src-tauri && cargo test     # 后端单元测试（加密/CRUD/备份回环）
pnpm tauri build               # 产出 .app / .dmg
```

数据位置：`~/Library/Application Support/com.apkvault.app/vault.db`
设计文档：`SPEC.md`
