//! Google OAuth 2.0 for installed apps: authorization URL with PKCE, code
//! exchange, refresh, and identity.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::antigravity::oauth_client::OAuthClient;
use crate::error::{Error, Result};

pub const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const USERINFO_ENDPOINT: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

/// The scopes Antigravity's own sign-in is granted (read back from Google's
/// tokeninfo for an Antigravity 2.19.1 session). Requesting the same set
/// makes OrbitSwitch's tokens equivalent to the IDE's.
pub const SCOPES: [&str; 7] = [
    "openid",
    "email",
    "profile",
    "https://www.googleapis.com/auth/cloud-platform",
    "https://www.googleapis.com/auth/aicode",
    "https://www.googleapis.com/auth/cclog",
    "https://www.googleapis.com/auth/experimentsandconfigs",
];

/// URL-safe random string with `bytes` bytes of entropy.
pub fn random_token(bytes: usize) -> Result<String> {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).map_err(|err| Error::Invalid(err.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn new() -> Result<Pkce> {
        let verifier = random_token(48)?;
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Ok(Pkce {
            verifier,
            challenge,
        })
    }
}

pub fn authorization_url(
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    challenge: &str,
) -> String {
    let mut url = url::Url::parse(AUTH_ENDPOINT).expect("static URL");
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", &SCOPES.join(" "))
        .append_pair("access_type", "offline")
        // Always show the account chooser and issue a refresh token.
        .append_pair("prompt", "select_account consent")
        .append_pair("state", state)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256");
    url.into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenGrant {
    pub access_token: String,
    pub expires_in: i64,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub id_token: Option<String>,
}

pub async fn exchange_code(
    http: &reqwest::Client,
    client: &OAuthClient,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<TokenGrant> {
    token_request(
        http,
        &[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect_uri),
            ("client_id", &client.id),
            ("client_secret", &client.secret),
        ],
    )
    .await
}

pub async fn refresh(
    http: &reqwest::Client,
    client: &OAuthClient,
    refresh_token: &str,
) -> Result<TokenGrant> {
    token_request(
        http,
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &client.id),
            ("client_secret", &client.secret),
        ],
    )
    .await
}

async fn token_request(http: &reqwest::Client, form: &[(&str, &str)]) -> Result<TokenGrant> {
    let response = http.post(TOKEN_ENDPOINT).form(form).send().await?;
    let status = response.status();
    if status.is_success() {
        return Ok(response.json().await?);
    }
    let body: serde_json::Value = response.json().await.unwrap_or_default();
    match body["error"].as_str() {
        Some("invalid_grant") => Err(Error::TokenRevoked),
        _ => Err(Error::Http {
            endpoint: "oauth2/token",
            status: status.as_u16(),
        }),
    }
}

/// Identity of a Google account.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Profile {
    #[serde(alias = "sub")]
    pub id: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub picture: Option<String>,
}

pub async fn userinfo(http: &reqwest::Client, access_token: &str) -> Result<Profile> {
    let response = http
        .get(USERINFO_ENDPOINT)
        .bearer_auth(access_token)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(Error::Http {
            endpoint: "oauth2/userinfo",
            status: status.as_u16(),
        });
    }
    Ok(response.json().await?)
}

/// Claims read from an ID token that came straight from Google's token
/// endpoint over TLS, so the signature is not re-verified.
#[derive(Debug, Clone, Deserialize)]
pub struct IdClaims {
    pub sub: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub picture: Option<String>,
}

impl IdClaims {
    pub fn parse(jwt: &str) -> Option<IdClaims> {
        let payload = jwt.split('.').nth(1)?;
        let bytes = URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
        serde_json::from_slice(&bytes).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_id_token_payload() {
        let payload = URL_SAFE_NO_PAD.encode(r#"{"sub":"42","email":"a@example.com"}"#);
        let claims = IdClaims::parse(&format!("h.{payload}.s")).unwrap();
        assert_eq!(claims.sub, "42");
        assert_eq!(claims.email.as_deref(), Some("a@example.com"));
    }

    #[test]
    fn pkce_challenge_is_s256_of_verifier() {
        let pkce = Pkce::new().unwrap();
        assert_eq!(
            pkce.challenge,
            URL_SAFE_NO_PAD.encode(Sha256::digest(pkce.verifier.as_bytes()))
        );
        assert!(pkce.verifier.len() >= 43);
    }

    #[test]
    fn authorization_url_carries_pkce_and_offline_access() {
        let url = authorization_url("id", "http://127.0.0.1:1/oauth-callback", "st", "ch");
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A1%2Foauth-callback"));
    }
}
