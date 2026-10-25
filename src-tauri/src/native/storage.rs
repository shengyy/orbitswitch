//! Large secret bundles stay in the OS vault, split below Windows' blob limit.
//! A new generation becomes visible only after every chunk is safely written.
use crate::{
    error::{Error, Result},
    secret_store,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path};
const SERVICE: &str = "orbitswitch.sessions";
const CHUNK: usize = 2000;
const MAX_CHUNKS: usize = 256;
#[derive(Serialize, Deserialize)]
struct Index {
    generation: String,
    chunks: usize,
    digest: String,
}
fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn key(id: &str, index: &Index, n: usize) -> String {
    format!("{id}:{}:{n}", index.generation)
}
fn index(id: &str) -> Result<Option<Index>> {
    secret_store::get(SERVICE, id)?
        .map(|raw| serde_json::from_str(&raw).map_err(Error::from))
        .transpose()
}
pub fn get(id: &str) -> Result<Option<String>> {
    let Some(index) = index(id)? else {
        return Ok(None);
    };
    if index.chunks == 0 || index.chunks > MAX_CHUNKS {
        return Err(Error::Invalid("invalid session vault index".into()));
    }
    let mut encoded = String::new();
    for n in 0..index.chunks {
        encoded.push_str(
            &secret_store::get(SERVICE, &key(id, &index, n))?.ok_or(Error::VerifyFailed)?,
        );
    }
    let bytes = STANDARD.decode(encoded).map_err(|_| Error::VerifyFailed)?;
    if digest(&bytes) != index.digest {
        return Err(Error::VerifyFailed);
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| Error::VerifyFailed)
}
pub fn set(id: &str, raw: &str) -> Result<()> {
    let old = index(id)?;
    if old
        .as_ref()
        .is_some_and(|old| old.digest == digest(raw.as_bytes()))
    {
        return Ok(());
    }
    let encoded = STANDARD.encode(raw);
    let chunks = encoded.as_bytes().chunks(CHUNK).collect::<Vec<_>>();
    if chunks.is_empty() || chunks.len() > MAX_CHUNKS {
        return Err(Error::Invalid(
            "session exceeds the credential-store limit".into(),
        ));
    }
    let current = Index {
        generation: crate::google::oauth::random_token(12)?,
        chunks: chunks.len(),
        digest: digest(raw.as_bytes()),
    };
    let result = (|| {
        for (n, chunk) in chunks.into_iter().enumerate() {
            secret_store::set(
                SERVICE,
                &key(id, &current, n),
                std::str::from_utf8(chunk).expect("base64 is ASCII"),
            )?;
        }
        secret_store::set(SERVICE, id, &serde_json::to_string(&current)?)
    })();
    if result.is_err() {
        cleanup(id, &current);
    } else if let Some(old) = old {
        cleanup(id, &old);
    }
    result
}
fn cleanup(id: &str, index: &Index) {
    for n in 0..index.chunks.min(MAX_CHUNKS) {
        let _ = secret_store::delete(SERVICE, &key(id, index, n));
    }
}
pub fn delete(id: &str) -> Result<()> {
    let old = index(id)?;
    secret_store::delete(SERVICE, id)?;
    if let Some(old) = old {
        cleanup(id, &old);
    }
    Ok(())
}
pub fn read_json(path: &Path) -> Result<Option<serde_json::Value>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::Invalid("invalid credential destination".into()))?;
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        ".orbitswitch-{}.tmp",
        crate::google::oauth::random_token(12)?
    ));
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}
pub fn write_json(path: &Path, value: &serde_json::Value) -> Result<()> {
    write_private(path, &serde_json::to_vec(value)?)
}
pub fn restore_file(path: &Path, content: Option<&serde_json::Value>) -> Result<()> {
    match content {
        Some(value) => write_json(path, value),
        None => match fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        },
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
