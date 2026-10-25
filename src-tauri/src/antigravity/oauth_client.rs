//! Antigravity's own OAuth client.
//!
//! OrbitSwitch signs accounts in with the same client the IDE uses, so a refresh
//! token from OrbitSwitch's browser sign-in is interchangeable with one from the
//! IDE's own sign-in. The client ID is public (it appears in every sign-in
//! URL). The installed-app secret is read from the local Antigravity binaries
//! at runtime and verified against Google's token endpoint, so this
//! repository never redistributes it and follows Antigravity upgrades.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use memchr::memmem;

use crate::antigravity::install::Install;
use crate::error::{Error, Result};
use crate::google::oauth::TOKEN_ENDPOINT;

/// The consumer sign-in client (the `aud` of Antigravity 2.19.1 ID tokens).
pub const CLIENT_ID: &str =
    "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com";

const SECRET_PREFIX: &[u8] = b"GOCSPX-";
const SECRET_LEN: usize = 35;
const CHUNK: usize = 8 << 20;

#[derive(Clone)]
pub struct OAuthClient {
    pub id: String,
    pub secret: String,
}

impl fmt::Debug for OAuthClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuthClient")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

pub async fn discover(http: &reqwest::Client, install: &Install) -> Result<OAuthClient> {
    let bin_dir = install.bin_dir.clone();
    let secrets = tokio::task::spawn_blocking(move || scan_dir(&bin_dir))
        .await
        .map_err(|err| Error::Invalid(err.to_string()))??;
    for secret in secrets {
        if pairs_with_client(http, &secret).await? {
            log::info!("OAuth client secret discovered from the installed Antigravity");
            return Ok(OAuthClient {
                id: CLIENT_ID.into(),
                secret,
            });
        }
    }
    Err(Error::ClientDiscovery)
}

/// Exchanging a bogus code fails with `invalid_grant` only when the client
/// authenticated; a wrong secret fails with `invalid_client` instead.
async fn pairs_with_client(http: &reqwest::Client, secret: &str) -> Result<bool> {
    let response = http
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", "agyorbit-client-probe"),
            ("redirect_uri", "http://127.0.0.1"),
            ("client_id", CLIENT_ID),
            ("client_secret", secret),
        ])
        .send()
        .await?;
    let body: serde_json::Value = response.json().await.unwrap_or_default();
    Ok(body["error"] == "invalid_grant")
}

fn scan_dir(dir: &Path) -> Result<Vec<String>> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            for secret in scan_file(&path)? {
                if !found.contains(&secret) {
                    found.push(secret);
                }
            }
        }
    }
    Ok(found)
}

/// Streams the file in chunks, carrying a tail so matches across chunk
/// boundaries are not missed.
fn scan_file(path: &Path) -> Result<Vec<String>> {
    let mut file = File::open(path)?;
    let finder = memmem::Finder::new(SECRET_PREFIX);
    let mut found = Vec::new();
    let mut buf = Vec::with_capacity(CHUNK + SECRET_LEN);
    let mut chunk = vec![0u8; CHUNK];
    loop {
        let read = file.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..read]);
        for start in finder.find_iter(&buf) {
            // Go packs string literals back to back, so the secret is cut at
            // its fixed length; the token endpoint probe confirms it.
            if let Some(candidate) = buf.get(start..start + SECRET_LEN)
                && candidate[SECRET_PREFIX.len()..]
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
            {
                let secret = String::from_utf8_lossy(candidate).into_owned();
                if !found.contains(&secret) {
                    found.push(secret);
                }
            }
        }
        let keep = buf.len().min(SECRET_LEN);
        buf.drain(..buf.len() - keep);
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_secrets_across_chunk_boundaries() {
        let dir = std::env::temp_dir().join(format!("agyorbit-scan-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bin");
        // Built at runtime so the source never contains a secret-shaped literal.
        let secret = [
            std::str::from_utf8(SECRET_PREFIX).unwrap(),
            "abcdefghijklmnopqrstuvwx_-01",
        ]
        .concat();
        assert_eq!(secret.len(), SECRET_LEN);
        let mut data = vec![b'.'; CHUNK - 10];
        data.extend_from_slice(secret.as_bytes());
        data.extend_from_slice(b"https://next");
        std::fs::write(&path, &data).unwrap();
        assert_eq!(scan_file(&path).unwrap(), vec![secret]);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
