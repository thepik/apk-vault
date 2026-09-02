//! SQLite 数据层：platform / key_entry 表与全部 CRUD。

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::Serialize;
use uuid::Uuid;

use crate::crypto;

pub const PLATFORM_COLORS: [&str; 8] = [
    "blue", "purple", "pink", "red", "orange", "yellow", "green", "gray",
];

#[derive(Serialize, Clone)]
pub struct PlatformDto {
    pub id: String,
    pub name: String,
    pub color: String,
    /// OpenAI 协议调用地址，空串表示未设置
    pub endpoint_openai: String,
    /// Anthropic 协议调用地址，空串表示未设置
    pub endpoint_anthropic: String,
    pub key_count: i64,
    pub last_copied_at: Option<i64>,
}

#[derive(Serialize, Clone)]
pub struct EntryDto {
    pub id: String,
    pub platform_id: String,
    pub name: String,
    pub masked_key: String,
    pub copy_count: i64,
    pub last_copied_at: Option<i64>,
    pub created_at: i64,
}

pub struct Db {
    conn: Mutex<Connection>,
    master_key: [u8; crypto::KEY_LEN],
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 调用地址规范化：trim；留空表示未设置；非空时必须是 http(s) URL。
fn normalize_endpoint(url: &str) -> Result<String, String> {
    let u = url.trim();
    if u.is_empty() {
        return Ok(String::new());
    }
    if !(u.starts_with("http://") || u.starts_with("https://")) {
        return Err("调用地址需以 http:// 或 https:// 开头".into());
    }
    Ok(u.to_string())
}

impl Db {
    pub fn open(path: &Path, master_key: [u8; crypto::KEY_LEN]) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create data dir: {e}"))?;
        }
        let conn = Connection::open(path).map_err(|e| format!("open db: {e}"))?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS platform (
              id          TEXT PRIMARY KEY,
              name        TEXT NOT NULL UNIQUE,
              color       TEXT NOT NULL DEFAULT 'blue',
              created_at  INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS key_entry (
              id             TEXT PRIMARY KEY,
              platform_id    TEXT NOT NULL REFERENCES platform(id) ON DELETE CASCADE,
              name           TEXT NOT NULL,
              key_encrypted  BLOB NOT NULL,
              copy_count     INTEGER NOT NULL DEFAULT 0,
              last_copied_at INTEGER,
              created_at     INTEGER NOT NULL,
              updated_at     INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_key_entry_platform ON key_entry(platform_id);
            ",
        )
        .map_err(|e| format!("migrate db: {e}"))?;

        // 增量迁移：为旧库补充平台调用地址字段（新库由上面的建表语句直接带出）。
        let existing_cols: Vec<String> = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(platform)")
                .map_err(|e| format!("migrate db: {e}"))?;
            let cols = stmt
                .query_map([], |r| r.get::<_, String>(1))
                .map_err(|e| format!("migrate db: {e}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("migrate db: {e}"))?;
            cols
        };
        for col in ["endpoint_openai", "endpoint_anthropic"] {
            if !existing_cols.iter().any(|c| c == col) {
                conn.execute(
                    &format!("ALTER TABLE platform ADD COLUMN {col} TEXT NOT NULL DEFAULT ''"),
                    [],
                )
                .map_err(|e| format!("migrate db: {e}"))?;
            }
        }

        Ok(Db {
            conn: Mutex::new(conn),
            master_key,
        })
    }

    // ---------- 平台 ----------

    pub fn list_platforms(&self) -> Result<Vec<PlatformDto>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT p.id, p.name, p.color, p.endpoint_openai, p.endpoint_anthropic,
                        (SELECT COUNT(*) FROM key_entry k WHERE k.platform_id = p.id) AS key_count,
                        (SELECT MAX(k.last_copied_at) FROM key_entry k WHERE k.platform_id = p.id) AS last_copied_at
                 FROM platform p
                 ORDER BY last_copied_at IS NULL, last_copied_at DESC, p.created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(PlatformDto {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    color: r.get(2)?,
                    endpoint_openai: r.get(3)?,
                    endpoint_anthropic: r.get(4)?,
                    key_count: r.get(5)?,
                    last_copied_at: r.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn add_platform(
        &self,
        name: &str,
        color: &str,
        endpoint_openai: &str,
        endpoint_anthropic: &str,
    ) -> Result<PlatformDto, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("平台名称不能为空".into());
        }
        let endpoint_openai = normalize_endpoint(endpoint_openai)?;
        let endpoint_anthropic = normalize_endpoint(endpoint_anthropic)?;
        let color = if PLATFORM_COLORS.contains(&color) {
            color.to_string()
        } else {
            let idx = name.chars().fold(0usize, |a, c| a + c as usize) % PLATFORM_COLORS.len();
            PLATFORM_COLORS[idx].to_string()
        };
        let id = Uuid::new_v4().to_string();
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO platform (id, name, color, endpoint_openai, endpoint_anthropic, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, name, color, endpoint_openai, endpoint_anthropic, now_ms()],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                "平台已存在".to_string()
            } else {
                e.to_string()
            }
        })?;
        Ok(PlatformDto {
            id,
            name: name.to_string(),
            color,
            endpoint_openai,
            endpoint_anthropic,
            key_count: 0,
            last_copied_at: None,
        })
    }

    /// 更新平台的两个调用地址（传空串表示清除）。
    pub fn set_platform_endpoints(
        &self,
        id: &str,
        openai: &str,
        anthropic: &str,
    ) -> Result<(), String> {
        let openai = normalize_endpoint(openai)?;
        let anthropic = normalize_endpoint(anthropic)?;
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let changed = conn
            .execute(
                "UPDATE platform SET endpoint_openai = ?2, endpoint_anthropic = ?3 WHERE id = ?1",
                params![id, openai, anthropic],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("平台不存在".into());
        }
        Ok(())
    }

    pub fn rename_platform(&self, id: &str, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("平台名称不能为空".into());
        }
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE platform SET name = ?2 WHERE id = ?1",
            params![id, name],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                "平台已存在".to_string()
            } else {
                e.to_string()
            }
        })?;
        Ok(())
    }

    /// 返回被连带删除的 Key 数量（供前端确认文案用：删除前先调用 platform_key_count）。
    pub fn delete_platform(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM platform WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ---------- Key ----------

    fn decrypt_blob(&self, blob: &[u8]) -> Result<String, String> {
        let plain = crypto::decrypt(&self.master_key, blob)?;
        String::from_utf8(plain).map_err(|e| format!("utf8: {e}"))
    }

    pub fn list_entries(&self) -> Result<Vec<EntryDto>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, platform_id, name, key_encrypted, copy_count, last_copied_at, created_at
                 FROM key_entry
                 ORDER BY last_copied_at IS NULL, last_copied_at DESC, created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Vec<u8>>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                    r.get::<_, i64>(6)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())? {
            let plain = self.decrypt_blob(&row.3)?;
            out.push(EntryDto {
                id: row.0,
                platform_id: row.1,
                name: row.2,
                masked_key: crypto::mask_key(&plain),
                copy_count: row.4,
                last_copied_at: row.5,
                created_at: row.6,
            });
        }
        Ok(out)
    }

    pub fn add_entry(&self, platform_id: &str, name: &str, key: &str) -> Result<EntryDto, String> {
        let name = name.trim();
        let key = key.trim();
        if name.is_empty() {
            return Err("名称不能为空".into());
        }
        if key.is_empty() {
            return Err("Key 值不能为空".into());
        }
        let id = Uuid::new_v4().to_string();
        let blob = crypto::encrypt(&self.master_key, key.as_bytes())?;
        let now = now_ms();
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO key_entry (id, platform_id, name, key_encrypted, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, platform_id, name, blob, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(EntryDto {
            id,
            platform_id: platform_id.to_string(),
            name: name.to_string(),
            masked_key: crypto::mask_key(key),
            copy_count: 0,
            last_copied_at: None,
            created_at: now,
        })
    }

    /// key 为 None 表示不替换 Key 值。
    pub fn update_entry(
        &self,
        id: &str,
        name: &str,
        platform_id: &str,
        key: Option<&str>,
    ) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("名称不能为空".into());
        }
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE key_entry SET name = ?2, platform_id = ?3, updated_at = ?4 WHERE id = ?1",
            params![id, name, platform_id, now_ms()],
        )
        .map_err(|e| e.to_string())?;
        if let Some(k) = key {
            let k = k.trim();
            if !k.is_empty() {
                let blob = crypto::encrypt(&self.master_key, k.as_bytes())?;
                conn.execute(
                    "UPDATE key_entry SET key_encrypted = ?2, updated_at = ?3 WHERE id = ?1",
                    params![id, blob, now_ms()],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    pub fn delete_entry(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM key_entry WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn reveal_entry(&self, id: &str) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let blob: Vec<u8> = conn
            .query_row(
                "SELECT key_encrypted FROM key_entry WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        drop(conn);
        self.decrypt_blob(&blob)
    }

    /// 复制：返回明文并更新统计。
    pub fn copy_entry(&self, id: &str) -> Result<String, String> {
        let plain = self.reveal_entry(id)?;
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE key_entry SET copy_count = copy_count + 1, last_copied_at = ?2 WHERE id = ?1",
            params![id, now_ms()],
        )
        .map_err(|e| e.to_string())?;
        Ok(plain)
    }

    // ---------- 备份内部接口 ----------

    pub fn dump_all(
        &self,
    ) -> Result<
        (
            Vec<(String, String, String, String)>,
            Vec<(String, String, String, i64, Option<i64>, i64)>,
        ),
        String,
    > {
        // platforms: (name, color, endpoint_openai, endpoint_anthropic)
        // entries: (platform_name, name, plaintext_key, copy_count, last_copied_at, created_at)
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut p_stmt = conn
            .prepare("SELECT name, color, endpoint_openai, endpoint_anthropic FROM platform ORDER BY created_at")
            .map_err(|e| e.to_string())?;
        let platforms: Vec<(String, String, String, String)> = p_stmt
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;

        let mut e_stmt = conn
            .prepare(
                "SELECT p.name, k.name, k.key_encrypted, k.copy_count, k.last_copied_at, k.created_at
                 FROM key_entry k JOIN platform p ON p.id = k.platform_id
                 ORDER BY k.created_at",
            )
            .map_err(|e| e.to_string())?;
        let raw: Vec<(String, String, Vec<u8>, i64, Option<i64>, i64)> = e_stmt
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        drop(e_stmt);
        drop(p_stmt);
        drop(conn);

        let mut entries = Vec::new();
        for (pname, name, blob, cc, lca, ca) in raw {
            let plain = self.decrypt_blob(&blob)?;
            entries.push((pname, name, plain, cc, lca, ca));
        }
        Ok((platforms, entries))
    }

    /// 从备份恢复：平台按名称合并（同名复用），Key 全部新增。
    pub fn restore_all(
        &self,
        platforms: &[(String, String, String, String)],
        entries: &[(String, String, String, i64, Option<i64>, i64)],
    ) -> Result<usize, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        for (name, color, ep_openai, ep_anthropic) in platforms {
            let color = if PLATFORM_COLORS.contains(&color.as_str()) {
                color.as_str()
            } else {
                "blue"
            };
            let ep_openai = normalize_endpoint(ep_openai).unwrap_or_default();
            let ep_anthropic = normalize_endpoint(ep_anthropic).unwrap_or_default();
            conn.execute(
                "INSERT OR IGNORE INTO platform (id, name, color, endpoint_openai, endpoint_anthropic, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![Uuid::new_v4().to_string(), name, color, ep_openai, ep_anthropic, now_ms()],
            )
            .map_err(|e| e.to_string())?;
        }
        let mut count = 0usize;
        for (pname, name, plain, cc, lca, ca) in entries {
            let pid: Option<String> = conn
                .query_row(
                    "SELECT id FROM platform WHERE name = ?1",
                    params![pname],
                    |r| r.get(0),
                )
                .ok();
            let Some(pid) = pid else { continue };
            let blob = crypto::encrypt(&self.master_key, plain.as_bytes())?;
            let now = now_ms();
            conn.execute(
                "INSERT INTO key_entry (id, platform_id, name, key_encrypted, copy_count, last_copied_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    Uuid::new_v4().to_string(),
                    pid,
                    name,
                    blob,
                    cc,
                    lca,
                    ca,
                    now
                ],
            )
            .map_err(|e| e.to_string())?;
            count += 1;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Db {
        let path = std::env::temp_dir().join(format!("apkvault-test-{}.db", Uuid::new_v4()));
        Db::open(&path, [7u8; crypto::KEY_LEN]).unwrap()
    }

    #[test]
    fn full_crud_and_masking() {
        let db = test_db();
        let p = db.add_platform("OpenAI", "", "", "").unwrap();
        assert!(db.add_platform("openai", "", "", "").is_err() || db.add_platform("OpenAI", "", "", "").is_err());

        let e = db
            .add_entry(&p.id, "  个人主力 ", "  sk-test-abcd1234  ")
            .unwrap();
        assert_eq!(e.name, "个人主力"); // trim
        assert_eq!(e.masked_key, "sk-…1234");
        assert_eq!(e.copy_count, 0);

        // 明文不落盘：直接读库字段
        {
            let conn = db.conn.lock().unwrap();
            let blob: Vec<u8> = conn
                .query_row("SELECT key_encrypted FROM key_entry WHERE id = ?1", params![e.id], |r| r.get(0))
                .unwrap();
            let blob_str = String::from_utf8_lossy(&blob);
            assert!(!blob_str.contains("sk-test-abcd1234"));
        }

        // reveal / copy
        assert_eq!(db.reveal_entry(&e.id).unwrap(), "sk-test-abcd1234");
        assert_eq!(db.copy_entry(&e.id).unwrap(), "sk-test-abcd1234");
        let entries = db.list_entries().unwrap();
        assert_eq!(entries[0].copy_count, 1);
        assert!(entries[0].last_copied_at.is_some());

        // 编辑：改名换平台，不替换 key
        let p2 = db.add_platform("DeepSeek", "green", "", "").unwrap();
        db.update_entry(&e.id, "改名后", &p2.id, None).unwrap();
        let entries = db.list_entries().unwrap();
        assert_eq!(entries[0].name, "改名后");
        assert_eq!(entries[0].platform_id, p2.id);
        assert_eq!(db.reveal_entry(&e.id).unwrap(), "sk-test-abcd1234");

        // 替换 key
        db.update_entry(&e.id, "改名后", &p2.id, Some("sk-new-9999")).unwrap();
        assert_eq!(db.reveal_entry(&e.id).unwrap(), "sk-new-9999");

        // 删除平台级联
        db.delete_platform(&p2.id).unwrap();
        assert!(db.list_entries().unwrap().is_empty());
    }

    #[test]
    fn short_key_masking() {
        assert_eq!(crypto::mask_key("abc"), "abc…");
        assert_eq!(crypto::mask_key("sk-ant-xyz123"), "sk-…z123");
    }

    #[test]
    fn platform_endpoints_crud_and_validation() {
        let db = test_db();
        let p = db
            .add_platform(
                "Relay",
                "",
                " https://api.example.com/v1 ",
                "https://anthropic.example.com",
            )
            .unwrap();
        // trim 生效
        assert_eq!(p.endpoint_openai, "https://api.example.com/v1");
        assert_eq!(p.endpoint_anthropic, "https://anthropic.example.com");

        // 更新 + 清空
        db.set_platform_endpoints(&p.id, "http://localhost:8080/v1", "").unwrap();
        let listed = db.list_platforms().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].endpoint_openai, "http://localhost:8080/v1");
        assert_eq!(listed[0].endpoint_anthropic, "");

        // 非法地址被拒绝
        assert!(db.set_platform_endpoints(&p.id, "api.example.com/v1", "").is_err());
        // 平台不存在报错
        assert!(db.set_platform_endpoints("no-such-id", "https://x.com", "").is_err());
    }
}
