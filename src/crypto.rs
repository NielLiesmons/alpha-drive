use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;

/// Generate a 32-byte root capability (keep on devices only).
pub fn generate_root_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

pub fn parse_root_key(hex_str: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(hex_str.trim()).map_err(|e| e.to_string())?;
    if bytes.len() != 32 {
        return Err(format!("root key must be 32 bytes, got {}", bytes.len()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// Encrypt plaintext. Returns (nonce || ciphertext, random 32-byte file key).
/// v0: file key is returned so the client can store it wrapped in the dir entry.
/// Later: wrap with parent cryptree key instead of storing under root only.
pub fn encrypt(
    _root: &[u8; 32],
    _path: &str,
    plaintext: &[u8],
) -> Result<(Vec<u8>, [u8; 32]), String> {
    let mut file_key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut file_key);
    let cipher = Aes256Gcm::new_from_slice(&file_key).map_err(|e| e.to_string())?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|_| "encrypt failed".to_string())?;
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok((out, file_key))
}

/// Decrypt a block that was encrypted under `root` with path-bound HKDF (dir nodes).
pub fn decrypt(root: &[u8; 32], path: &str, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let key = derive_path_key(root, path);
    decrypt_with_key(&key, ciphertext)
}

pub fn encrypt_with_root(
    root: &[u8; 32],
    path: &str,
    plaintext: &[u8],
) -> Result<Vec<u8>, String> {
    let key = derive_path_key(root, path);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|_| "encrypt failed".to_string())?;
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

fn derive_path_key(root: &[u8; 32], path: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"alpha-drive/v0/");
    hasher.update(root);
    hasher.update(path.as_bytes());
    let dig = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&dig);
    key
}

pub fn decrypt_with_key(key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    if key.len() != 32 {
        return Err("key must be 32 bytes".into());
    }
    if ciphertext.len() < 12 {
        return Err("ciphertext too short".into());
    }
    let (nonce_bytes, ct) = ciphertext.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ct)
        .map_err(|_| "decrypt failed".into())
}
