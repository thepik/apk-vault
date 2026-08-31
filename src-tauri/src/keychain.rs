//! 主密钥存取：macOS Keychain / Windows 凭据管理器（免登录、静默访问）。

#[cfg(target_os = "macos")]
mod imp {
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
}

#[cfg(target_os = "windows")]
mod imp {
    use crate::crypto::{generate_master_key, KEY_LEN};

    const TARGET: &str = "com.apkvault.app.master-key";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn get_or_create_master_key() -> Result<[u8; KEY_LEN], String> {
        use windows::core::PCWSTR;
        use windows::Win32::Security::Credentials::{
            CredDeleteW, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
        };

        let target_w = wide(TARGET);
        let target = PCWSTR(target_w.as_ptr());

        unsafe {
            let mut cred: *mut CREDENTIALW = std::ptr::null_mut();
            match CredReadW(target, CRED_TYPE_GENERIC, None, &mut cred) {
                Ok(()) => {
                    debug_assert!(!cred.is_null());
                    let blob_ptr = (*cred).CredentialBlob;
                    let blob_len = (*cred).CredentialBlobSize as usize;
                    let result = if blob_len == KEY_LEN {
                        let mut key = [0u8; KEY_LEN];
                        std::ptr::copy_nonoverlapping(
                            blob_ptr as *const u8,
                            key.as_mut_ptr(),
                            KEY_LEN,
                        );
                        Ok(key)
                    } else {
                        // 异常长度：删掉重建
                        let key = generate_master_key();
                        let _ = CredDeleteW(target, CRED_TYPE_GENERIC, None);
                        store(&key).map(|_| key)
                    };
                    let _ = windows::Win32::Foundation::LocalFree(Some(
                        windows::Win32::Foundation::HLOCAL(
                            blob_ptr.cast::<core::ffi::c_void>(),
                        ),
                    ));
                    result
                }
                Err(_) => {
                    // 未找到（首次启动）或其他读取异常：统一重新生成并写入，
                    // 写失败会向上返回错误信息。
                    let key = generate_master_key();
                    store(&key)?;
                    Ok(key)
                }
            }
        }
    }

    fn store(key: &[u8; KEY_LEN]) -> Result<(), String> {
        use windows::core::PWSTR;
        use windows::Win32::Security::Credentials::{
            CredWriteW, CREDENTIALW, CRED_FLAGS, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
        };

        let mut target_w = wide(TARGET);
        let mut comment_w = wide("APK Vault master key");

        let cred = CREDENTIALW {
            Flags: CRED_FLAGS(0),
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(target_w.as_mut_ptr()),
            Comment: PWSTR(comment_w.as_mut_ptr()),
            LastWritten: Default::default(),
            CredentialBlobSize: KEY_LEN as u32,
            CredentialBlob: key.as_ptr() as *mut _,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: PWSTR::null(),
            UserName: PWSTR::null(),
        };

        unsafe {
            CredWriteW(&cred, 0)
                .map_err(|e| format!("credential manager write failed: {e}"))
        }
    }
}

pub use imp::get_or_create_master_key;
