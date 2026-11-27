//! Claude's desktop browser sign-in is separate from its CLI OAuth credential.
//! Capture only authentication cookies, never history, projects or settings.
use super::storage;
use crate::error::{Error, Result};
#[cfg(target_os = "macos")]
use crate::secret_store;
use base64::{Engine, engine::general_purpose::STANDARD};
use rusqlite::{
    Connection, OpenFlags,
    types::{Value as SqlValue, ValueRef},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
const NAMES: &[&str] = &[
    "sessionKey",
    "sessionKeyV2",
    "sessionKeyV3",
    "sessionKeyLC",
    "sessionKeyV3LC",
    "lastActiveOrg",
    "cf_clearance",
    "__cf_bm",
];
const CONFIG_KEYS: &[&str] = &[
    "lastKnownAccountUuid",
    "oauth:tokenCache",
    "oauth:tokenCacheV2",
];
#[derive(Clone, Serialize, Deserialize, PartialEq)]
enum Cell {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(String),
}
impl Cell {
    fn sql(&self) -> Result<SqlValue> {
        Ok(match self {
            Self::Null => SqlValue::Null,
            Self::Integer(n) => SqlValue::Integer(*n),
            Self::Real(n) => SqlValue::Real(*n),
            Self::Text(s) => SqlValue::Text(s.clone()),
            Self::Blob(s) => SqlValue::Blob(STANDARD.decode(s).map_err(|_| Error::VerifyFailed)?),
        })
    }
    fn text(&self) -> Option<&str> {
        if let Self::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    columns: Vec<String>,
    rows: Vec<Vec<Cell>>,
    config: Value,
}
fn directory() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("CLAUDE_USER_DATA_DIR") {
        return Ok(path.into());
    }
    #[cfg(target_os = "macos")]
    let path = std::env::home_dir().map(|p| p.join("Library/Application Support/Claude"));
    #[cfg(windows)]
    let path = std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("Claude"));
    #[cfg(not(any(target_os = "macos", windows)))]
    let path = std::env::home_dir().map(|p| p.join(".config/Claude"));
    path.ok_or_else(|| Error::Invalid("desktop data directory unavailable".into()))
}
fn database(dir: &Path) -> PathBuf {
    let network = dir.join("Network/Cookies");
    if network.is_file() {
        network
    } else {
        dir.join("Cookies")
    }
}
fn sql_error(_: rusqlite::Error) -> Error {
    Error::Invalid("could not access the desktop authentication database".into())
}
fn columns(db: &Connection) -> Result<Vec<String>> {
    db.prepare("PRAGMA table_info(cookies)")
        .map_err(sql_error)?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
}
fn filter() -> String {
    format!(
        "host_key IN ('claude.ai','.claude.ai') AND name IN ({})",
        NAMES
            .iter()
            .map(|s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(",")
    )
}
pub fn capture() -> Result<Option<Session>> {
    capture_at(&directory()?)
}
fn capture_at(dir: &Path) -> Result<Option<Session>> {
    let path = database(dir);
    if !path.is_file() {
        return Ok(None);
    }
    let db =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(sql_error)?;
    db.busy_timeout(std::time::Duration::from_secs(3))
        .map_err(sql_error)?;
    let columns = columns(&db)?;
    let mut stmt = db
        .prepare(&format!(
            "SELECT * FROM cookies WHERE {} ORDER BY name,host_key,path",
            filter()
        ))
        .map_err(sql_error)?;
    let rows = stmt
        .query_map([], |row| {
            (0..columns.len())
                .map(|i| {
                    Ok(match row.get_ref(i)? {
                        ValueRef::Null => Cell::Null,
                        ValueRef::Integer(n) => Cell::Integer(n),
                        ValueRef::Real(n) => Cell::Real(n),
                        ValueRef::Text(s) => Cell::Text(String::from_utf8_lossy(s).into()),
                        ValueRef::Blob(b) => Cell::Blob(STANDARD.encode(b)),
                    })
                })
                .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)?;
    let all =
        storage::read_json(&dir.join("config.json"))?.unwrap_or_else(|| serde_json::json!({}));
    let config = Value::Object(
        CONFIG_KEYS
            .iter()
            .filter_map(|k| all.get(*k).map(|v| ((*k).into(), v.clone())))
            .collect(),
    );
    Ok(Some(Session {
        columns,
        rows,
        config,
    }))
}
impl Session {
    pub fn matches(&self, subject: &str) -> bool {
        self.config["lastKnownAccountUuid"].as_str() == Some(subject)
            && self
                .rows
                .iter()
                .any(|row| self.field(row, "name") == Some("sessionKey"))
    }
    fn field<'a>(&self, row: &'a [Cell], name: &str) -> Option<&'a str> {
        row.get(self.columns.iter().position(|s| s == name)?)?
            .text()
    }
    fn cookie(&self, row: &[Cell]) -> Result<String> {
        if let Some(value) = self.field(row, "value").filter(|s| !s.is_empty()) {
            return Ok(value.into());
        }
        let index = self
            .columns
            .iter()
            .position(|s| s == "encrypted_value")
            .ok_or(Error::VerifyFailed)?;
        let Cell::Blob(encoded) = &row[index] else {
            return Err(Error::VerifyFailed);
        };
        let bytes = STANDARD.decode(encoded).map_err(|_| Error::VerifyFailed)?;
        let mut plain = decrypt(&bytes)?;
        // Chromium schema >=24 prefixes the value with SHA-256(host_key).
        use sha2::{Digest, Sha256};
        let host = self.field(row, "host_key").ok_or(Error::VerifyFailed)?;
        let hash = Sha256::digest(host.as_bytes());
        if plain.starts_with(&hash) {
            plain.drain(..32);
        }
        String::from_utf8(plain).map_err(|_| Error::VerifyFailed)
    }
    fn cookie_header(&self, org: &str) -> Result<(reqwest::header::HeaderValue, bool)> {
        let now = chrono::Utc::now().timestamp();
        let expiry = self.columns.iter().position(|s| s == "expires_utc");
        let mut cookies = Vec::new();
        let mut active_org = false;
        for row in &self.rows {
            let Some(name) = self.field(row, "name") else {
                continue;
            };
            if name != "lastActiveOrg" && !name.starts_with("sessionKey") {
                continue;
            }
            if let Some(Cell::Integer(at)) = expiry.and_then(|i| row.get(i))
                && *at > 0
                && *at / 1_000_000 - 11_644_473_600 <= now
            {
                continue;
            }
            let value = self.cookie(row)?;
            if name == "lastActiveOrg" {
                if value != org {
                    return Err(Error::DesktopSignInRequired);
                }
                active_org = true;
            }
            if value.contains(['\r', '\n', ';']) {
                return Err(Error::VerifyFailed);
            }
            cookies.push(format!("{name}={value}"));
        }
        if !cookies.iter().any(|s| s.starts_with("sessionKey=")) {
            return Err(Error::DesktopSignInRequired);
        }
        let mut header = reqwest::header::HeaderValue::from_str(&cookies.join("; "))
            .map_err(|_| Error::VerifyFailed)?;
        header.set_sensitive(true);
        Ok((header, active_org))
    }
    async fn verified_cookies(
        &self,
        http: &reqwest::Client,
        subject: &str,
        org: &str,
    ) -> Result<reqwest::header::HeaderValue> {
        if subject.is_empty() || org.is_empty() || !self.matches(subject) {
            return Err(Error::DesktopSignInRequired);
        }
        let (header, active_org) = self.cookie_header(org)?;
        let response = http
            .get("https://claude.ai/api/bootstrap")
            .header(reqwest::header::COOKIE, &header)
            .send()
            .await?;
        if matches!(response.status().as_u16(), 401 | 403) {
            return Err(Error::DesktopSignInRequired);
        }
        if !response.status().is_success() {
            return Err(Error::Http {
                endpoint: "claude/desktop-profile",
                status: response.status().as_u16(),
            });
        }
        verify_identity(&response.json().await?, subject, org, active_org)?;
        Ok(header)
    }
    pub async fn verify(&self, http: &reqwest::Client, subject: &str, org: &str) -> Result<()> {
        self.verified_cookies(http, subject, org).await.map(|_| ())
    }
    pub async fn usage(&self, http: &reqwest::Client, subject: &str, org: &str) -> Result<Value> {
        let cookies = self.verified_cookies(http, subject, org).await?;
        let mut url = url::Url::parse("https://claude.ai/api/organizations").expect("static URL");
        url.path_segments_mut()
            .expect("HTTPS URL")
            .extend([org, "usage"]);
        let response = http
            .get(url)
            .header(reqwest::header::COOKIE, cookies)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(Error::Http {
                endpoint: "claude/desktop-usage",
                status: response.status().as_u16(),
            });
        }
        Ok(response.json().await?)
    }
}

fn verify_identity(body: &Value, subject: &str, org: &str, active_org: bool) -> Result<()> {
    if body["account"]["uuid"].as_str() != Some(subject) {
        return Err(Error::VerifyFailed);
    }
    if let Some(orgs) = body["organizations"].as_array() {
        if !orgs.iter().any(|entry| entry["uuid"].as_str() == Some(org)) {
            return Err(Error::VerifyFailed);
        }
        if active_org || orgs.len() == 1 {
            return Ok(());
        }
    } else if active_org {
        return Ok(());
    }
    Err(Error::DesktopSignInRequired)
}

#[cfg(target_os = "macos")]
fn decrypt(bytes: &[u8]) -> Result<Vec<u8>> {
    use cbc::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
    if !bytes.starts_with(b"v10") {
        return Err(Error::Invalid(
            "unsupported desktop cookie encryption".into(),
        ));
    }
    let password = secret_store::get("Claude Safe Storage", "Claude Key")?
        .ok_or(Error::DesktopSignInRequired)?;
    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(password.as_bytes(), b"saltysalt", 1003, &mut key);
    cbc::Decryptor::<aes::Aes128>::new(&key.into(), &[b' '; 16].into())
        .decrypt_padded_vec_mut::<Pkcs7>(&bytes[3..])
        .map_err(|_| Error::VerifyFailed)
}
#[cfg(windows)]
fn decrypt(bytes: &[u8]) -> Result<Vec<u8>> {
    use aes_gcm::{
        Aes256Gcm, Nonce,
        aead::{Aead, KeyInit},
    };
    fn unprotect(bytes: &[u8]) -> Result<Vec<u8>> {
        use windows_sys::Win32::{
            Foundation::LocalFree,
            Security::Cryptography::{
                CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptUnprotectData,
            },
        };
        let input = CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let ok = unsafe {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err(Error::VerifyFailed);
        }
        let result =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
        unsafe {
            LocalFree(output.pbData as _);
        };
        Ok(result)
    }
    if bytes.starts_with(b"v20") {
        return Err(Error::Invalid(
            "desktop uses app-bound cookie encryption".into(),
        ));
    }
    if !bytes.starts_with(b"v10") && !bytes.starts_with(b"v11") {
        return unprotect(bytes);
    }
    let state =
        storage::read_json(&directory()?.join("Local State"))?.ok_or(Error::VerifyFailed)?;
    let encrypted = STANDARD
        .decode(
            state["os_crypt"]["encrypted_key"]
                .as_str()
                .ok_or(Error::VerifyFailed)?,
        )
        .map_err(|_| Error::VerifyFailed)?;
    let key = unprotect(
        encrypted
            .strip_prefix(b"DPAPI")
            .ok_or(Error::VerifyFailed)?,
    )?;
    if bytes.len() < 31 {
        return Err(Error::VerifyFailed);
    }
    Aes256Gcm::new_from_slice(&key)
        .map_err(|_| Error::VerifyFailed)?
        .decrypt(Nonce::from_slice(&bytes[3..15]), &bytes[15..])
        .map_err(|_| Error::VerifyFailed)
}
#[cfg(not(any(target_os = "macos", windows)))]
fn decrypt(_: &[u8]) -> Result<Vec<u8>> {
    Err(Error::Invalid(
        "desktop cookie decryption is unsupported on this platform".into(),
    ))
}
pub fn apply(session: &Session) -> Result<()> {
    apply_at(&directory()?, session)
}
fn apply_at(dir: &Path, session: &Session) -> Result<()> {
    let mut db = Connection::open(database(dir)).map_err(sql_error)?;
    if columns(&db)? != session.columns {
        return Err(Error::Invalid(
            "desktop cookie schema changed; import the account again".into(),
        ));
    }
    if session
        .columns
        .iter()
        .any(|s| !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    {
        return Err(Error::VerifyFailed);
    }
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(sql_error)?;
    tx.execute(&format!("DELETE FROM cookies WHERE {}", filter()), [])
        .map_err(sql_error)?;
    let sql = format!(
        "INSERT INTO cookies ({}) VALUES ({})",
        session.columns.join(","),
        vec!["?"; session.columns.len()].join(",")
    );
    for row in &session.rows {
        let values = row.iter().map(Cell::sql).collect::<Result<Vec<_>>>()?;
        tx.execute(&sql, rusqlite::params_from_iter(values))
            .map_err(sql_error)?;
    }
    tx.commit().map_err(sql_error)?;
    let path = dir.join("config.json");
    let mut config = storage::read_json(&path)?.unwrap_or_else(|| serde_json::json!({}));
    for key in CONFIG_KEYS {
        config
            .as_object_mut()
            .ok_or(Error::VerifyFailed)?
            .remove(*key);
        if let Some(value) = session.config.get(*key) {
            config[*key] = value.clone();
        }
    }
    // Rebuild UI caches from the restored identity; never persist another account's chat previews.
    for key in ["bootSideSessions", "bootSidebarPlaceholder"] {
        config
            .as_object_mut()
            .ok_or(Error::VerifyFailed)?
            .remove(key);
    }
    storage::write_json(&path, &config)?;
    let check = capture_at(dir)?.ok_or(Error::VerifyFailed)?;
    if check.rows != session.rows || check.config != session.config {
        return Err(Error::VerifyFailed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_identity_requires_the_account_and_an_unambiguous_organization() {
        let body = serde_json::json!({"account":{"uuid":"user"},"organizations":[{"uuid":"one"},{"uuid":"two"}]});
        assert!(verify_identity(&body, "user", "one", true).is_ok());
        assert!(verify_identity(&body, "other", "one", true).is_err());
        assert!(verify_identity(&body, "user", "other", true).is_err());
        assert!(verify_identity(&body, "user", "one", false).is_err());
        let single =
            serde_json::json!({"account":{"uuid":"user"},"organizations":[{"uuid":"one"}]});
        assert!(verify_identity(&single, "user", "one", false).is_ok());
        let account_only = serde_json::json!({"account":{"uuid":"user"}});
        assert!(verify_identity(&account_only, "user", "one", false).is_err());
        assert!(verify_identity(&account_only, "user", "one", true).is_ok());
    }

    #[test]
    fn desktop_requests_use_only_current_sign_in_cookies_for_the_expected_org() {
        let row = |name: &str, value: &str, expiry: i64| {
            vec![
                Cell::Text(name.into()),
                Cell::Text(value.into()),
                Cell::Integer(expiry),
            ]
        };
        let mut session = Session {
            columns: vec!["name".into(), "value".into(), "expires_utc".into()],
            rows: vec![
                row("sessionKey", "test", 0),
                row("lastActiveOrg", "one", 0),
                row("sessionKeyV3", "expired", 1),
                row("__cf_bm", "unrelated", 0),
            ],
            config: serde_json::json!({"lastKnownAccountUuid":"user"}),
        };
        let (header, active_org) = session.cookie_header("one").unwrap();
        assert!(active_org && header.is_sensitive());
        assert_eq!(
            header.to_str().unwrap(),
            "sessionKey=test; lastActiveOrg=one"
        );
        assert!(session.cookie_header("two").is_err());
        session.rows[0] = row("sessionKey", "expired", 1);
        assert!(session.cookie_header("one").is_err());
    }
    #[test]
    fn replacing_and_restoring_auth_keeps_other_cookies_and_preferences() {
        let dir = tempfile::tempdir().unwrap();
        let db = Connection::open(dir.path().join("Cookies")).unwrap();
        db.execute_batch("CREATE TABLE cookies(host_key TEXT, name TEXT, path TEXT, value TEXT, encrypted_value BLOB); INSERT INTO cookies VALUES('.claude.ai','sessionKey','/','original',X''),('.example.com','unrelated','/','keep',X'');").unwrap();
        let config = serde_json::json!({"lastKnownAccountUuid":"account-a", "theme":"dark", "windowBounds":{"width":900}});
        storage::write_json(&dir.path().join("config.json"), &config).unwrap();
        let previous = capture_at(dir.path()).unwrap().unwrap();
        let mut target = previous.clone();
        target.config["lastKnownAccountUuid"] = "account-b".into();
        let value = target.columns.iter().position(|c| c == "value").unwrap();
        target.rows[0][value] = Cell::Text("target".into());
        apply_at(dir.path(), &target).unwrap();
        let stored = storage::read_json(&dir.path().join("config.json"))
            .unwrap()
            .unwrap();
        assert_eq!(stored["theme"], "dark");
        assert_eq!(stored["windowBounds"]["width"], 900);
        let unrelated: String = db
            .query_row(
                "SELECT value FROM cookies WHERE name='unrelated'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(unrelated, "keep");
        apply_at(dir.path(), &previous).unwrap();
        assert!(capture_at(dir.path()).unwrap().unwrap().rows == previous.rows);
        assert_eq!(
            storage::read_json(&dir.path().join("config.json"))
                .unwrap()
                .unwrap(),
            config
        );
    }
    #[test]
    fn schema_drift_stops_before_deleting_any_cookie() {
        let dir = tempfile::tempdir().unwrap();
        let db = Connection::open(dir.path().join("Cookies")).unwrap();
        db.execute_batch("CREATE TABLE cookies(host_key TEXT, name TEXT, path TEXT, value TEXT); INSERT INTO cookies VALUES('.claude.ai','sessionKey','/','keep');").unwrap();
        let mut target = capture_at(dir.path()).unwrap().unwrap();
        target.columns.push("future_column".into());
        assert!(apply_at(dir.path(), &target).is_err());
        let count: i64 = db
            .query_row("SELECT count(*) FROM cookies", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
