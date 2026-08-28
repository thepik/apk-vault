//! 加密备份导出/导入（.apkvault）：一次性口令经 Argon2 派生密钥，AES-256-GCM 加密整体 JSON。

use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::crypto;
use crate::db::Db;

const MAGIC: &[u8; 5] = b"APKV1";
const SALT_LEN: usize = 16;

#[derive(Serialize, Deserialize)]
struct BackupPayload {
    version: u32,
    platforms: Vec<BackupPlatform>,
    entries: Vec<BackupEntry>,
}

#[derive(Serialize, Deserialize)]
struct BackupPlatform {
    name: String,
    color: String,
}

#[derive(Serialize, Deserialize)]
struct BackupEntry {
    platform: String,
    name: String,
    key: String,
    copy_count: i64,
    last_copied_at: Option<i64>,
    created_at: i64,
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; crypto::KEY_LEN], String> {
    let mut key = [0u8; crypto::KEY_LEN];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| format!("kdf failed: {e}"))?;
    Ok(key)
}

pub fn export_backup(db: &Db, passphrase: &str) -> Result<String, String> {
    if passphrase.len() < 4 {
        return Err("备份口令至少 4 位".into());
    }
    let (platforms, entries) = db.dump_all()?;
    let payload = BackupPayload {
        version: 1,
        platforms: platforms
            .into_iter()
            .map(|(name, color)| BackupPlatform { name, color })
            .collect(),
        entries: entries
            .into_iter()
            .map(|(platform, name, key, copy_count, last_copied_at, created_at)| BackupEntry {
                platform,
                name,
                key,
                copy_count,
                last_copied_at,
                created_at,
            })
            .collect(),
    };
    let json = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;

    let mut salt = [0u8; SALT_LEN];
    rand::rng().fill_bytes(&mut salt);
    let key = derive_key(passphrase, &salt)?;
    let blob = crypto::encrypt(&key, &json)?; // nonce || ct

    let mut file = Vec::with_capacity(MAGIC.len() + SALT_LEN + blob.len());
    file.extend_from_slice(MAGIC);
    file.extend_from_slice(&salt);
    file.extend_from_slice(&blob);
    Ok(B64.encode(file))
}

/// 返回恢复的 Key 数量。
pub fn import_backup(db: &Db, passphrase: &str, content: &str) -> Result<usize, String> {
    let file = B64
        .decode(content.trim())
        .map_err(|_| "备份文件格式无效".to_string())?;
    if file.len() < MAGIC.len() + SALT_LEN + crypto::NONCE_LEN || &file[..5] != MAGIC {
        return Err("备份文件格式无效".into());
    }
    let salt = &file[5..5 + SALT_LEN];
    let blob = &file[5 + SALT_LEN..];
    let key = derive_key(passphrase, salt)?;
    let json = crypto::decrypt(&key, blob).map_err(|_| "口令错误或文件已损坏".to_string())?;
    let payload: BackupPayload =
        serde_json::from_slice(&json).map_err(|_| "备份内容解析失败".to_string())?;
    if payload.version != 1 {
        return Err(format!("不支持的备份版本 v{}", payload.version));
    }
    let platforms: Vec<(String, String)> = payload
        .platforms
        .into_iter()
        .map(|p| (p.name, p.color))
        .collect();
    let entries: Vec<(String, String, String, i64, Option<i64>, i64)> = payload
        .entries
        .into_iter()
        .map(|e| (e.platform, e.name, e.key, e.copy_count, e.last_copied_at, e.created_at))
        .collect();
    db.restore_all(&platforms, &entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KEY_LEN;
    use uuid::Uuid;

    fn test_db() -> Db {
        let path = std::env::temp_dir().join(format!("apkvault-bak-{}.db", Uuid::new_v4()));
        Db::open(&path, [9u8; KEY_LEN]).unwrap()
    }

    #[test]
    fn export_import_roundtrip() {
        let src = test_db();
        let p = src.add_platform("OpenAI", "blue").unwrap();
        src.add_entry(&p.id, "主力", "sk-live-abcdef").unwrap();

        let content = export_backup(&src, "口令1234").unwrap();
        assert!(!content.contains("sk-live-abcdef")); // 备份无无明文

        // 换一台"机器"（不同主密钥的库）
        let dst = test_db();
        let n = import_backup(&dst, "口令1234", &content).unwrap();
        assert_eq!(n, 1);

        let entries = dst.list_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "主力");
        assert_eq!(dst.reveal_entry(&entries[0].id).unwrap(), "sk-live-abcdef");
        let plats = dst.list_platforms().unwrap();
        assert_eq!(plats[0].name, "OpenAI");

        // 同平台名合并：再次导入不新增平台
        let n2 = import_backup(&dst, "口令1234", &content).unwrap();
        assert_eq!(n2, 1);
        assert_eq!(dst.list_platforms().unwrap().len(), 1);
        assert_eq!(dst.list_entries().unwrap().len(), 2);

        // 错误口令
        assert!(import_backup(&dst, "wrong", &content).is_err());
        // 短口令导出拒绝
        assert!(export_backup(&src, "abc").is_err());
    }
}
