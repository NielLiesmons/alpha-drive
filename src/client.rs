use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::crypto::{decrypt, decrypt_with_key, encrypt, encrypt_with_root, parse_root_key};
use crate::store::hash_hex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DirChild {
    pub hash: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrapped_key: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct DirNode {
    pub schema: String,
    pub children: std::collections::BTreeMap<String, DirChild>,
}

/// List entries in the encrypted root directory.
pub async fn list_dir(
    base: &str,
    token: &str,
    root_key_hex: &str,
) -> Result<DirNode, Box<dyn std::error::Error>> {
    let root = parse_root_key(root_key_hex).map_err(|e| e.to_string())?;
    load_root_dir(base, token, &root).await
}

pub async fn put_path(
    base: &str,
    token: &str,
    root_key_hex: &str,
    logical_path: &str,
    kind: &str,
    plaintext: &[u8],
) -> Result<String, Box<dyn std::error::Error>> {
    let root = parse_root_key(root_key_hex).map_err(|e| e.to_string())?;
    let (ciphertext, file_key) = encrypt(&root, logical_path, plaintext).map_err(|e| e.to_string())?;
    let hash = hash_hex(&ciphertext);

    put_block(base, token, &hash, &ciphertext).await?;

    let mut dir = load_root_dir(base, token, &root).await?;
    let name = normalize_path(logical_path);
    if name.is_empty() {
        return Err("empty path".into());
    }
    dir.schema = "alpha.dir/1".into();
    dir.children.insert(
        name,
        DirChild {
            hash: hash.clone(),
            kind: kind.to_string(),
            // v0: store file key hex in the encrypted dir node (dir node keyed by root).
            // Later: proper cryptree wrap to parent directory key.
            wrapped_key: Some(hex::encode(file_key)),
        },
    );
    let dir_bytes = serde_json::to_vec(&dir)?;
    let dir_ct = encrypt_with_root(&root, "", &dir_bytes).map_err(|e| e.to_string())?;
    let dir_hash = hash_hex(&dir_ct);
    put_block(base, token, &dir_hash, &dir_ct).await?;
    put_tip(base, token, &dir_hash).await?;
    Ok(hash)
}

pub async fn get_path(
    base: &str,
    token: &str,
    root_key_hex: &str,
    logical_path: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (bytes, _) = get_path_with_key(base, token, root_key_hex, logical_path).await?;
    Ok(bytes)
}

/// Like `get_path`, but also returns the entry's wrapped file-key hex (for media chunks).
pub async fn get_path_with_key(
    base: &str,
    token: &str,
    root_key_hex: &str,
    logical_path: &str,
) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let root = parse_root_key(root_key_hex).map_err(|e| e.to_string())?;
    let dir = load_root_dir(base, token, &root).await?;
    let name = normalize_path(logical_path);
    let child = dir
        .children
        .get(&name)
        .ok_or_else(|| format!("path not found: {logical_path}"))?;
    let ciphertext = get_block(base, token, &child.hash).await?;
    let file_key = child
        .wrapped_key
        .as_ref()
        .ok_or("missing wrapped_key on entry")?
        .clone();
    let key_bytes = hex::decode(&file_key)?;
    let pt = decrypt_with_key(&key_bytes, &ciphertext).map_err(|e| e.to_string())?;
    Ok((pt, file_key))
}

/// Encrypt plaintext under a caller-supplied file key (media manifests share this key with chunks).
pub async fn put_path_with_key(
    base: &str,
    token: &str,
    root_key_hex: &str,
    logical_path: &str,
    kind: &str,
    plaintext: &[u8],
    file_key: &[u8; 32],
) -> Result<String, Box<dyn std::error::Error>> {
    let root = parse_root_key(root_key_hex).map_err(|e| e.to_string())?;
    let ciphertext = encrypt_with_file_key(file_key, plaintext).map_err(|e| e.to_string())?;
    let hash = hash_hex(&ciphertext);
    put_block(base, token, &hash, &ciphertext).await?;

    let mut dir = load_root_dir(base, token, &root).await?;
    let name = normalize_path(logical_path);
    if name.is_empty() {
        return Err("empty path".into());
    }
    dir.schema = "alpha.dir/1".into();
    dir.children.insert(
        name,
        DirChild {
            hash: hash.clone(),
            kind: kind.to_string(),
            wrapped_key: Some(hex::encode(file_key)),
        },
    );
    let dir_bytes = serde_json::to_vec(&dir)?;
    let dir_ct = encrypt_with_root(&root, "", &dir_bytes).map_err(|e| e.to_string())?;
    let dir_hash = hash_hex(&dir_ct);
    put_block(base, token, &dir_hash, &dir_ct).await?;
    put_tip(base, token, &dir_hash).await?;
    Ok(hash)
}

fn encrypt_with_file_key(file_key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce};
    use rand::RngCore;
    let cipher = Aes256Gcm::new_from_slice(file_key).map_err(|e| e.to_string())?;
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

pub async fn put_block_public(
    base: &str,
    token: &str,
    hash: &str,
    body: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    put_block(base, token, hash, body).await
}

pub async fn get_block_public(
    base: &str,
    token: &str,
    hash: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    get_block(base, token, hash).await
}

fn normalize_path(logical_path: &str) -> String {
    logical_path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

async fn load_root_dir(
    base: &str,
    token: &str,
    root: &[u8; 32],
) -> Result<DirNode, Box<dyn std::error::Error>> {
    let tip = match get_tip(base, token).await {
        Ok(h) if !h.is_empty() => h,
        _ => {
            return Ok(DirNode {
                schema: "alpha.dir/1".into(),
                children: Default::default(),
            });
        }
    };
    let ct = get_block(base, token, &tip).await?;
    let pt = decrypt(root, "", &ct).map_err(|e| e.to_string())?;
    Ok(serde_json::from_slice(&pt)?)
}

async fn put_block(
    base: &str,
    token: &str,
    hash: &str,
    body: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let res = client
        .put(format!("{base}/v1/blocks/{hash}"))
        .header("Authorization", format!("Bearer {token}"))
        .body(body.to_vec())
        .send()
        .await?;
    if !res.status().is_success() {
        return Err(format!("put block {}: {}", hash, res.status()).into());
    }
    Ok(())
}

async fn get_block(
    base: &str,
    token: &str,
    hash: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{base}/v1/blocks/{hash}"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await?;
    if !res.status().is_success() {
        return Err(format!("get block {}: {}", hash, res.status()).into());
    }
    Ok(res.bytes().await?.to_vec())
}

async fn get_tip(base: &str, token: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{base}/v1/tip"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await?;
    if res.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("no tip".into());
    }
    if !res.status().is_success() {
        return Err(format!("get tip: {}", res.status()).into());
    }
    let body: serde_json::Value = res.json().await?;
    Ok(body["root_hash"].as_str().unwrap_or("").to_string())
}

async fn put_tip(
    base: &str,
    token: &str,
    root_hash: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let res = client
        .put(format!("{base}/v1/tip"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "root_hash": root_hash }))
        .send()
        .await?;
    if !res.status().is_success() {
        return Err(format!("put tip: {}", res.status()).into());
    }
    Ok(())
}
