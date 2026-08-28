export interface Platform {
  id: string;
  name: string;
  color: string;
  key_count: number;
  last_copied_at: number | null;
}

export interface KeyEntry {
  id: string;
  platform_id: string;
  name: string;
  masked_key: string;
  copy_count: number;
  last_copied_at: number | null;
  created_at: number;
}

export interface VaultState {
  platforms: Platform[];
  entries: KeyEntry[];
}

export const PLATFORM_COLORS: Record<string, string> = {
  blue: "var(--p-blue)",
  purple: "var(--p-purple)",
  pink: "var(--p-pink)",
  red: "var(--p-red)",
  orange: "var(--p-orange)",
  yellow: "var(--p-yellow)",
  green: "var(--p-green)",
  gray: "var(--p-gray)",
};

export function relativeTime(ms: number | null): string {
  if (!ms) return "从未使用";
  const diff = Date.now() - ms;
  const min = Math.floor(diff / 60000);
  if (min < 1) return "刚刚用过";
  if (min < 60) return `${min} 分钟前用过`;
  const h = Math.floor(min / 60);
  if (h < 24) return `${h} 小时前用过`;
  const d = Math.floor(h / 24);
  if (d < 30) return `${d} 天前用过`;
  return new Date(ms).toLocaleDateString();
}
