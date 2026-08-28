import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { save, open } from "@tauri-apps/plugin-dialog";
import type { Platform, KeyEntry, VaultState } from "./types";

export const api = {
  listState: () => invoke<VaultState>("list_state"),

  addPlatform: (name: string, color?: string) =>
    invoke<Platform>("add_platform", { name, color: color ?? null }),
  renamePlatform: (id: string, name: string) =>
    invoke<void>("rename_platform", { id, name }),
  deletePlatform: (id: string) => invoke<void>("delete_platform", { id }),

  addEntry: (platformId: string, name: string, key: string) =>
    invoke<KeyEntry>("add_entry", { platformId, name, key }),
  updateEntry: (id: string, name: string, platformId: string, key?: string) =>
    invoke<void>("update_entry", { id, name, platformId, key: key ?? null }),
  deleteEntry: (id: string) => invoke<void>("delete_entry", { id }),
  revealEntry: (id: string) => invoke<string>("reveal_entry", { id }),

  /** 复制完整 Key 到剪贴板并更新使用统计 */
  copyEntry: async (id: string) => {
    const plain = await invoke<string>("copy_entry", { id });
    await writeText(plain);
  },

  /** 弹出保存对话框并写出加密备份；用户取消返回 false */
  exportBackup: async (passphrase: string): Promise<boolean> => {
    const path = await save({
      title: "导出加密备份",
      defaultPath: "apk-vault-backup.apkvault",
      filters: [{ name: "APK Vault 备份", extensions: ["apkvault"] }],
    });
    if (!path) return false;
    await invoke<void>("export_backup", { passphrase, path });
    return true;
  },

  /** 弹出选择对话框并导入；返回恢复的 Key 数量，取消返回 null */
  importBackup: async (passphrase: string): Promise<number | null> => {
    const path = await open({
      title: "导入加密备份",
      filters: [{ name: "APK Vault 备份", extensions: ["apkvault"] }],
    });
    if (!path || typeof path !== "string") return null;
    return await invoke<number>("import_backup", { passphrase, path });
  },
};
