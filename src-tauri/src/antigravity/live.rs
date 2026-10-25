//! The credential Antigravity is currently signed in with.
//!
//! Antigravity's language server stores it with go-keyring under service
//! `gemini`, account `antigravity`, and mirrors it to
//! `~/.gemini/jetski-standalone-oauth-token`, which it reads when the keyring
//! times out. OrbitSwitch reads the keyring first and always writes both, so the
//! two copies never disagree about the active account.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::antigravity::credential::Bundle;
use crate::error::{Error, Result};
use crate::secret_store;

const SERVICE: &str = "gemini";
const ACCOUNT: &str = "antigravity";

fn fallback_path() -> Result<PathBuf> {
    std::env::home_dir()
        .map(|home| home.join(".gemini").join("jetski-standalone-oauth-token"))
        .ok_or_else(|| Error::Invalid("home directory is unknown".into()))
}

pub fn read() -> Result<Option<Bundle>> {
    if let Some(raw) = secret_store::get(SERVICE, ACCOUNT)? {
        return Bundle::decode(&raw).map(Some);
    }
    match fs::read_to_string(fallback_path()?) {
        Ok(raw) => Bundle::decode(&raw).map(Some),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

pub fn write(bundle: &Bundle) -> Result<()> {
    secret_store::set(SERVICE, ACCOUNT, &bundle.to_keyring_value()?)?;
    write_private_file(&fallback_path()?, bundle.to_json()?.as_bytes())
}

/// Signs Antigravity out locally by removing both copies.
pub fn clear() -> Result<()> {
    secret_store::delete(SERVICE, ACCOUNT)?;
    match fs::remove_file(fallback_path()?) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err.into()),
        _ => Ok(()),
    }
}

/// Atomically replaces `path` with owner-only permissions.
fn write_private_file(path: &PathBuf, contents: &[u8]) -> Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| Error::Invalid("credential path has no parent".into()))?;
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".orbitswitch-{}.tmp", std::process::id()));
    {
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let mut file = options.open(&tmp)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })?;
    Ok(())
}
