use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sha2::{Digest, Sha256};

pub struct BlobStore {
    root: PathBuf,
    /// Serialize mailbox tip CAS (read tip → compare prev → set tip).
    tip_lock: Mutex<()>,
}

impl BlobStore {
    pub fn open(root: PathBuf) -> Result<Self, std::io::Error> {
        fs::create_dir_all(root.join("blocks"))?;
        Ok(Self {
            root,
            tip_lock: Mutex::new(()),
        })
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

    /// Named tips (e.g. `chat`) — small cryptrees that must not rewrite the Space tip.
    pub fn get_named_tip(&self, name: &str) -> Result<String, std::io::Error> {
        let s = fs::read_to_string(self.named_tip_path(name)?)?;
        Ok(s.trim().to_string())
    }

    pub fn set_named_tip(&self, name: &str, root_hash: &str) -> Result<(), std::io::Error> {
        atomic_write(&self.named_tip_path(name)?, root_hash.as_bytes())
    }

    /// Append-once mailbox: store ciphertext block, CAS-advance named tip if `prev` matches.
    /// Returns `Ok(Ok(()))` on success, `Ok(Err(current_tip))` on CAS conflict, `Err` on I/O.
    pub fn append_mailbox(
        &self,
        name: &str,
        hash: &str,
        body: &[u8],
        prev: &str,
    ) -> Result<Result<(), String>, std::io::Error> {
        self.put_verified(hash, body)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        let _guard = self
            .tip_lock
            .lock()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "tip lock poisoned"))?;

        let tip_path = self.named_tip_path(name)?;
        let current = match fs::read_to_string(&tip_path) {
            Ok(s) => s.trim().to_string(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e),
        };
        if current != prev {
            return Ok(Err(current));
        }
        atomic_write(&tip_path, hash.as_bytes())?;
        Ok(Ok(()))
    }

    fn named_tip_path(&self, name: &str) -> Result<PathBuf, std::io::Error> {
        if !is_tip_name(name) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid tip name",
            ));
        }
        Ok(self.root.join("tips").join(name))
    }
}

fn is_tip_name(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    let rest_ok = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
    rest_ok && s.len() <= 64
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
