//! 主密钥存取：macOS Keychain（免登录、同签名应用静默访问）。

use crate::crypto::{generate_master_key, KEY_LEN};

const SERVICE: &str = "com.apkvault.app.master-key";
const ACCOUNT: &str = "apk-vault";

pub fn get_or_create_master_key() -> Result<[u8; KEY_LEN], String> {
    match security_framework::passwords::get_generic_password(SERVICE, ACCOUNT) {
        Ok(bytes) => {
            if bytes.len() == KEY_LEN {
                let mut key = [0u8; KEY_LEN];
                key.copy_from_slice(&bytes);
                Ok(key)
            } else {
                // 异常长度：覆盖重写
                let key = generate_master_key();
                security_framework::passwords::set_generic_password(SERVICE, ACCOUNT, &key)
                    .map_err(|e| format!("keychain write failed: {e}"))?;
                Ok(key)
            }
        }
        Err(_) => {
            let key = generate_master_key();
            security_framework::passwords::set_generic_password(SERVICE, ACCOUNT, &key)
                .map_err(|e| format!("keychain write failed: {e}"))?;
            Ok(key)
        }
    }
}
