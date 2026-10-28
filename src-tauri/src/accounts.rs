//! Antigravity account sign-in, re-authorization, removal and live adoption.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::oneshot;

use crate::antigravity::live;
use crate::error::{Error, Result};
use crate::google::loopback::Loopback;
use crate::google::oauth::{self, IdClaims, Pkce, Profile};
use crate::model::Operation;
use crate::provider::Provider;
use crate::registry::{self, Account};
use crate::state::Orbit;
use crate::{quota, shell, vault};

const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Runs the browser sign-in and stores the account. Signing in to an account
/// that is already listed renews its authorization. Returns the account id.
pub async fn add(orbit: &Arc<Orbit>) -> Result<String> {
    let guard = orbit.begin(Operation::Adding {
        provider: Provider::Antigravity,
        can_copy_link: false,
    })?;
    let sync = orbit.sync_lock.lock().await;
    let client = orbit.oauth_client().await?;
    let loopback = Loopback::bind().await?;
    let redirect_uri = loopback.redirect_uri.clone();
    let pkce = Pkce::new()?;
    let state = oauth::random_token(24)?;
    let url = oauth::authorization_url(&client.id, &redirect_uri, &state, &pkce.challenge);
    let (cancel_tx, cancel_rx) = oneshot::channel();
    orbit.set_pending_auth(cancel_tx, Some(url.clone()));
    if orbit.app.opener().open_url(url, None::<&str>).is_err() {
        log::warn!("could not open the browser; use Copy sign-in link to continue");
        shell::show(&orbit.app);
    }
    let code = tokio::select! {
        code = loopback.wait_for_code(&state) => code,
        _ = cancel_rx => Err(Error::OAuthCancelled),
        _ = tokio::time::sleep(SIGN_IN_TIMEOUT) => Err(Error::OAuthTimeout),
    }?;
    // Retire the link before token exchange so it cannot be copied after use.
    orbit.cancel_pending_auth();

    let grant =
        oauth::exchange_code(&orbit.http, &client, &code, &pkce.verifier, &redirect_uri).await?;
    let refresh_token = grant
        .refresh_token
        .clone()
        .ok_or_else(|| Error::Invalid("Google returned no refresh token".into()))?;
    let profile = profile(orbit, &grant.access_token, grant.id_token.as_deref()).await?;
    vault::set(&profile.id, &refresh_token)?;
    orbit
        .tokens
        .put(&profile.id, grant.access_token, grant.expires_in);
    orbit.update(|state| state.registry.upsert(account_from(&profile)))?;
    log::info!("account added");

    drop(sync);
    drop(guard);
    shell::show(&orbit.app);
    quota::refresh_one(orbit, &profile.id).await;
    Ok(profile.id)
}

pub fn cancel_add(orbit: &Orbit) {
    orbit.cancel_pending_auth();
}

pub fn copy_sign_in_link(orbit: &Orbit) -> Result<()> {
    orbit
        .app
        .clipboard()
        .write_text(orbit.sign_in_url()?)
        .map_err(|_| Error::Invalid("could not copy the sign-in link".into()))
}

/// Forgets the account. Antigravity stays signed in if it was the active one.
pub async fn remove(orbit: &Arc<Orbit>, account_id: &str) -> Result<()> {
    let _guard = orbit.begin(Operation::Removing {
        provider: Provider::Antigravity,
    })?;
    let _sync = orbit.sync_lock.lock().await;
    vault::delete(account_id)?;
    orbit.tokens.forget(account_id);
    orbit.update(|state| {
        state.quotas.remove(account_id);
        state.registry.remove(account_id)
    })
}

/// Follows the account Antigravity is signed in with. A sign-in that OrbitSwitch
/// has not seen yet (for example one made inside the IDE) is adopted once.
pub async fn sync_live(orbit: &Arc<Orbit>) -> Result<()> {
    let Ok(_sync) = orbit.sync_lock.try_lock() else {
        return Ok(());
    };
    if !orbit.is_idle() {
        return Ok(());
    }
    let Some(bundle) = tokio::task::spawn_blocking(live::read)
        .await
        .map_err(|err| Error::Invalid(err.to_string()))??
    else {
        orbit.update(|state| state.active_id = None);
        return Ok(());
    };
    let Some(claims) = bundle.claims() else {
        log::warn!("signed-in credential has no ID token; active account unknown");
        return Ok(());
    };
    let refresh_token = bundle.token.refresh_token;
    let fingerprint = registry::fingerprint(&refresh_token);
    let is_new_sign_in = !refresh_token.is_empty()
        && orbit.state().registry.live_fingerprint() != Some(fingerprint.as_str());

    if is_new_sign_in {
        vault::set(&claims.sub, &refresh_token)?;
        orbit.tokens.forget(&claims.sub);
        let profile = match orbit.access_token(&claims.sub).await {
            Ok(token) => profile(orbit, &token, bundle.id_token.as_deref()).await?,
            Err(_) => profile_from_claims(&claims),
        };
        orbit.update(|state| -> Result<()> {
            state.registry.upsert(account_from(&profile))?;
            state.registry.set_live_fingerprint(fingerprint)
        })?;
        log::info!("adopted the account signed in to Antigravity");
        let orbit = orbit.clone();
        let id = claims.sub.clone();
        tauri::async_runtime::spawn(async move { quota::refresh_one(&orbit, &id).await });
    }
    orbit.update(|state| state.active_id = Some(claims.sub));
    Ok(())
}

/// Identity from Google's userinfo, falling back to the ID token claims.
async fn profile(orbit: &Orbit, access_token: &str, id_token: Option<&str>) -> Result<Profile> {
    match oauth::userinfo(&orbit.http, access_token).await {
        Ok(profile) if !profile.id.is_empty() => Ok(profile),
        result => {
            if let Err(err) = result {
                log::warn!("userinfo failed: {err}");
            }
            id_token
                .and_then(IdClaims::parse)
                .map(|claims| profile_from_claims(&claims))
                .ok_or_else(|| Error::Invalid("could not identify the Google account".into()))
        }
    }
}

fn profile_from_claims(claims: &IdClaims) -> Profile {
    Profile {
        id: claims.sub.clone(),
        email: claims.email.clone().unwrap_or_default(),
        name: claims.name.clone(),
        picture: claims.picture.clone(),
    }
}

fn account_from(profile: &Profile) -> Account {
    Account {
        provider: Provider::Antigravity,
        desktop_ready: true,
        id: profile.id.clone(),
        email: profile.email.clone(),
        name: profile.name.clone(),
        picture: profile.picture.clone(),
        plan: None,
        added_at: Utc::now(),
    }
}
