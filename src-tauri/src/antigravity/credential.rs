//! The credential bundle Antigravity persists after a sign-in.
//!
//! Observed on Antigravity 2.19.1:
//!
//! ```json
//! {"token":{"access_token":"…","token_type":"Bearer","refresh_token":"…","expiry":"2026-10-03T17:52:30.07+08:00"},
//!  "auth_method":"consumer","id_token":"…"}
//! ```
//!
//! `token` is golang.org/x/oauth2's `Token`. Unknown fields are kept so a
//! newer Antigravity can add fields without OrbitSwitch dropping them.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{Local, SecondsFormat, TimeDelta};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::google::oauth::{IdClaims, TokenGrant};

/// Prefix go-keyring adds on macOS before base64-encoding the value.
const KEYRING_PREFIX: &str = "go-keyring-base64:";
/// What Antigravity records for a personal Google account sign-in.
pub const CONSUMER_AUTH_METHOD: &str = "consumer";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub token: Token,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub access_token: String,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Bundle {
    /// Builds the bundle Antigravity expects from a fresh token grant.
    pub fn from_grant(grant: &TokenGrant, refresh_token: &str, auth_method: &str) -> Bundle {
        let expiry = Local::now() + TimeDelta::seconds(grant.expires_in);
        Bundle {
            token: Token {
                access_token: grant.access_token.clone(),
                token_type: "Bearer".into(),
                refresh_token: refresh_token.to_owned(),
                expiry: Some(expiry.to_rfc3339_opts(SecondsFormat::AutoSi, false)),
                extra: Map::new(),
            },
            auth_method: Some(auth_method.to_owned()),
            id_token: grant.id_token.clone(),
            extra: Map::new(),
        }
    }

    /// Parses either the keyring encoding or plain JSON.
    pub fn decode(raw: &str) -> Result<Bundle> {
        let raw = raw.trim();
        match raw.strip_prefix(KEYRING_PREFIX) {
            Some(encoded) => {
                let bytes = STANDARD
                    .decode(encoded)
                    .map_err(|err| Error::Invalid(format!("keyring value is not base64: {err}")))?;
                Ok(serde_json::from_slice(&bytes)?)
            }
            None => Ok(serde_json::from_str(raw)?),
        }
    }

    /// Plain JSON, as written to the fallback file.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// The value go-keyring would store on this platform.
    pub fn to_keyring_value(&self) -> Result<String> {
        let json = self.to_json()?;
        if cfg!(target_os = "macos") {
            Ok(format!("{KEYRING_PREFIX}{}", STANDARD.encode(json)))
        } else {
            Ok(json)
        }
    }

    pub fn claims(&self) -> Option<IdClaims> {
        self.id_token.as_deref().and_then(IdClaims::parse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"token":{"access_token":"a","token_type":"Bearer","refresh_token":"r","expiry":"2026-10-03T17:52:30.076579+08:00","future":1},"auth_method":"consumer","id_token":"x.y.z","other":true}"#;

    #[test]
    fn round_trips_keyring_encoding_and_keeps_unknown_fields() {
        let bundle = Bundle::decode(SAMPLE).unwrap();
        let again = Bundle::decode(&bundle.to_keyring_value().unwrap()).unwrap();
        assert_eq!(again.token.refresh_token, "r");
        assert_eq!(again.token.extra["future"], 1);
        assert_eq!(again.extra["other"], true);
        assert_eq!(again.auth_method.as_deref(), Some(CONSUMER_AUTH_METHOD));
    }

    #[test]
    fn from_grant_matches_go_time_format() {
        let grant = TokenGrant {
            access_token: "a".into(),
            expires_in: 3599,
            refresh_token: None,
            id_token: None,
        };
        let bundle = Bundle::from_grant(&grant, "r", CONSUMER_AUTH_METHOD);
        let expiry = bundle.token.expiry.unwrap();
        assert!(chrono::DateTime::parse_from_rfc3339(&expiry).is_ok());
        assert!(
            !expiry.ends_with('Z'),
            "Go writes the local offset: {expiry}"
        );
    }
}
