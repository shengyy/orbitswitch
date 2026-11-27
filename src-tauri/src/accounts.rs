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
use crate::google::oauth::{self, Pkce, Profile};
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
    let profile =
        oauth::profile(&orbit.http, &grant.access_token, grant.id_token.as_deref()).await?;
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
    let refresh_token = &bundle.token.refresh_token;
    let fingerprint = registry::fingerprint(refresh_token);
    let is_new_sign_in = !refresh_token.is_empty()
        && orbit.state().registry.live_fingerprint() != Some(fingerprint.as_str());
    let claims = bundle.claims();
    if !is_new_sign_in && let Some(claims) = &claims {
        orbit.update(|state| state.active_id = Some(claims.sub.clone()));
        return Ok(());
    }
    if claims.is_none() && !refresh_token.is_empty() {
        let account_ids = orbit
            .state()
            .registry
            .accounts()
            .iter()
            .filter(|account| account.provider == Provider::Antigravity)
            .map(|account| account.id.clone())
            .collect();
        let token = refresh_token.clone();
        let matched = tokio::task::spawn_blocking(move || {
            match_saved_refresh_token(account_ids, &token, vault::get)
        })
        .await
        .map_err(|err| Error::Invalid(err.to_string()))?;
        match matched {
            Ok(Some(id)) => {
                return orbit.update(|state| {
                    state.registry.set_live_fingerprint(fingerprint)?;
                    state.active_id = Some(id);
                    Ok(())
                });
            }
            Ok(None) => {}
            Err(error) => log::warn!("could not compare saved Antigravity grants: {error}"),
        }
    }
    let profile = match bundle.profile(&orbit.http).await {
        Ok(profile) => profile,
        Err(error) => {
            orbit.update(|state| state.active_id = None);
            return Err(error);
        }
    };

    if is_new_sign_in {
        vault::set(&profile.id, refresh_token)?;
        orbit.tokens.forget(&profile.id);
        orbit.update(|state| -> Result<()> {
            state.registry.upsert(account_from(&profile))?;
            state.registry.set_live_fingerprint(fingerprint)
        })?;
        log::info!("adopted the account signed in to Antigravity");
    }
    orbit.update(|state| state.active_id = Some(profile.id));
    Ok(())
}

/// Saved grants are keyed by a previously verified Google account ID.
pub(crate) fn match_saved_refresh_token(
    account_ids: Vec<String>,
    refresh_token: &str,
    read_token: impl Fn(&str) -> Result<Option<String>>,
) -> Result<Option<String>> {
    let mut matched = None;
    for id in account_ids {
        if read_token(&id)?.as_deref() == Some(refresh_token) {
            if matched.is_some() {
                return Ok(None);
            }
            matched = Some(id);
        }
    }
    Ok(matched)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_a_saved_sign_in_by_its_exact_refresh_token() {
        let ids = || vec!["other".into(), "current".into(), "missing".into()];
        let read = |id: &str| {
            Ok(match id {
                "other" => Some("other-token".to_owned()),
                "current" => Some("current-token".to_owned()),
                _ => None,
            })
        };
        assert_eq!(
            match_saved_refresh_token(ids(), "current-token", read).unwrap(),
            Some("current".into())
        );
        assert_eq!(
            match_saved_refresh_token(ids(), "new-token", read).unwrap(),
            None
        );
    }

    #[test]
    fn refuses_an_ambiguous_refresh_token_match() {
        let ids = vec!["first".into(), "second".into()];
        let matched =
            match_saved_refresh_token(ids, "same-token", |_| Ok(Some("same-token".to_owned())));
        assert_eq!(matched.unwrap(), None);
    }
}
