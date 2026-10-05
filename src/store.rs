use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

pub struct BlobStore {
    root: PathBuf,
}

impl BlobStore {
    pub fn open(root: PathBuf) -> Result<Self, std::io::Error> {
        fs::create_dir_all(root.join("blocks"))?;
        Ok(Self { root })
    }

    fn block_path(&self, hash: &str) -> PathBuf {
        self.root.join("blocks").join(hash)
    }

    pub fn exists(&self, hash: &str) -> bool {
        self.block_path(hash).is_file()
    }

    pub fn get(&self, hash: &str) -> Result<Vec<u8>, std::io::Error> {
        fs::read(self.block_path(hash))
    }

    /// Store ciphertext only if `hash` equals SHA-256(hex) of body.
    pub fn put_verified(&self, hash: &str, body: &[u8]) -> Result<(), String> {
        let computed = hash_hex(body);
        if computed != hash {
            return Err(format!("hash mismatch: expected {hash}, got {computed}"));
        }
        if !is_hex_sha256(hash) {
            return Err("invalid hash".into());
        }
        let path = self.block_path(hash);
        if path.exists() {
            return Ok(());
        }
        atomic_write(&path, body).map_err(|e| e.to_string())
    }

    pub fn get_tip(&self) -> Result<String, std::io::Error> {
        let s = fs::read_to_string(self.root.join("tip"))?;
        Ok(s.trim().to_string())
    }

    pub fn set_tip(&self, root_hash: &str) -> Result<(), std::io::Error> {
        atomic_write(&self.root.join("tip"), root_hash.as_bytes())
    }
}

pub fn hash_hex(data: &[u8]) -> String {
    let dig = Sha256::digest(data);
    hex::encode(dig)
}

fn is_hex_sha256(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn atomic_write(path: &Path, body: &[u8]) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, body)?;
    fs::rename(tmp, path)?;
    Ok(())
}
