//! Chunked media blobs for Player stream-on-play (`alpha.media_blob/1`).

use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::client::{
    get_block_public, get_path_with_key, put_block_public, put_path_with_key,
};
use crate::crypto::{decrypt_chunk, encrypt_chunk};
use crate::store::hash_hex;

/// Default plaintext chunk size (256 KiB).
pub const DEFAULT_CHUNK_SIZE: usize = 256 * 1024;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaChunkMeta {
    pub i: u32,
    pub hash: String,
    pub plain_len: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaBlobManifest {
    pub schema: String,
    pub chunk_size: usize,
    pub plaintext_len: u64,
    pub content_type: String,
    pub chunks: Vec<MediaChunkMeta>,
}

/// Split plaintext, encrypt chunks into CAS, write encrypted manifest at `manifest_path`.
/// Manifest `wrapped_key` = media file key (same key used for every chunk).
pub async fn put_media_blob(
    base: &str,
    token: &str,
    root_key: &str,
    manifest_path: &str,
    content_type: &str,
    plaintext: &[u8],
    chunk_size: Option<usize>,
) -> Result<(MediaBlobManifest, String), Box<dyn std::error::Error>> {
    let chunk_size = chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE).max(64 * 1024);
    let mut file_key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut file_key);

    let mut chunks = Vec::new();
    let mut offset = 0usize;
    let mut i = 0u32;
    while offset < plaintext.len() {
        let end = (offset + chunk_size).min(plaintext.len());
        let slice = &plaintext[offset..end];
        let ct = encrypt_chunk(&file_key, i, slice).map_err(|e| e.to_string())?;
        let hash = hash_hex(&ct);
        put_block_public(base, token, &hash, &ct).await?;
        chunks.push(MediaChunkMeta {
            i,
            hash,
            plain_len: slice.len(),
        });
        offset = end;
        i += 1;
    }

    let manifest = MediaBlobManifest {
        schema: "alpha.media_blob/1".into(),
        chunk_size,
        plaintext_len: plaintext.len() as u64,
        content_type: content_type.to_string(),
        chunks,
    };
    let body = serde_json::to_vec_pretty(&manifest)?;
    put_path_with_key(
        base,
        token,
        root_key,
        manifest_path,
        "media",
        &body,
        &file_key,
    )
    .await?;
    Ok((manifest, hex::encode(file_key)))
}

/// Load and decrypt one chunk by index.
pub async fn get_media_chunk(
    base: &str,
    token: &str,
    root_key: &str,
    manifest_path: &str,
    chunk_index: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (manifest, file_key) = load_manifest_and_key(base, token, root_key, manifest_path).await?;
    let meta = manifest
        .chunks
        .iter()
        .find(|c| c.i == chunk_index)
        .ok_or_else(|| format!("chunk {chunk_index} not found"))?;
    let ct = get_block_public(base, token, &meta.hash).await?;
    let mut key = [0u8; 32];
    let kb = hex::decode(&file_key)?;
    if kb.len() != 32 {
        return Err("bad media key".into());
    }
    key.copy_from_slice(&kb);
    Ok(decrypt_chunk(&key, chunk_index, &ct).map_err(|e| e.to_string())?)
}

/// Fetch + decrypt all chunks (tests / small files). Prefer `get_media_chunk` for streaming.
pub async fn get_media_blob(
    base: &str,
    token: &str,
    root_key: &str,
    manifest_path: &str,
) -> Result<(MediaBlobManifest, Vec<u8>), Box<dyn std::error::Error>> {
    let (manifest, file_key) = load_manifest_and_key(base, token, root_key, manifest_path).await?;
    let mut key = [0u8; 32];
    let kb = hex::decode(&file_key)?;
    if kb.len() != 32 {
        return Err("bad media key".into());
    }
    key.copy_from_slice(&kb);

    let mut out = Vec::with_capacity(manifest.plaintext_len as usize);
    for meta in &manifest.chunks {
        let ct = get_block_public(base, token, &meta.hash).await?;
        let pt = decrypt_chunk(&key, meta.i, &ct).map_err(|e| e.to_string())?;
        if pt.len() != meta.plain_len {
            return Err("chunk length mismatch".into());
        }
        out.extend_from_slice(&pt);
    }
    if out.len() as u64 != manifest.plaintext_len {
        return Err("plaintext length mismatch".into());
    }
    Ok((manifest, out))
}

async fn load_manifest_and_key(
    base: &str,
    token: &str,
    root_key: &str,
    manifest_path: &str,
) -> Result<(MediaBlobManifest, String), Box<dyn std::error::Error>> {
    let (bytes, key_hex) = get_path_with_key(base, token, root_key, manifest_path).await?;
    let manifest: MediaBlobManifest = serde_json::from_slice(&bytes)?;
    Ok((manifest, key_hex))
}
