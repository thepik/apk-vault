//! AES-256-GCM 加解密：Key 值落盘前加密，读取时解密。

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::Rng;

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;

pub fn generate_master_key() -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    rand::rng().fill_bytes(&mut key);
    key
}

/// 返回 nonce(12B) || ciphertext+tag
pub fn encrypt(master: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(master));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("encrypt failed: {e}"))?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// 输入 nonce(12B) || ciphertext+tag
pub fn decrypt(master: &[u8; KEY_LEN], blob: &[u8]) -> Result<Vec<u8>, String> {
    if blob.len() < NONCE_LEN {
        return Err("encrypted blob too short".into());
    }
    let (nonce_bytes, ct) = blob.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(master));
    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ct)
        .map_err(|e| format!("decrypt failed: {e}"))
}

pub fn mask_key(plaintext: &str) -> String {
    let head: String = plaintext.chars().take(3).collect();
    let tail: String = {
        let t: Vec<char> = plaintext.chars().rev().take(4).collect();
        t.into_iter().rev().collect()
    };
    if plaintext.chars().count() <= 7 {
        format!("{head}…")
    } else {
        format!("{head}…{tail}")
    }
}
