//! Switching Antigravity to another account, plus plain stop and restart.
//!
//! Switch order: mint a fresh token for the target first (nothing is touched
//! if Google refuses), save the current sign-in back to its account, stop
//! every Antigravity process, write the credential, verify it, relaunch. If
//! writing or verifying fails, the previous credential is restored; once the
//! new one is verified the switch stands even if the relaunch fails.

use std::sync::Arc;

use crate::antigravity::credential::{Bundle, CONSUMER_AUTH_METHOD};
use crate::antigravity::{live, process};
use crate::error::{Error, Result};
use crate::google::oauth;
use crate::model::{Operation, QuotaState, SwitchStep};
use crate::provider::Provider;
use crate::registry;
use crate::state::Orbit;
use crate::{quota, vault};

pub async fn switch(orbit: &Arc<Orbit>, target_id: &str) -> Result<()> {
    let step = |step| Operation::Switching {
        provider: Provider::Antigravity,
        target_id: target_id.to_owned(),
        step,
    };
    let guard = orbit.begin(step(SwitchStep::Preparing))?;
    let sync = orbit.sync_lock.lock().await;
    let install = orbit.install()?;
    if orbit.state().registry.get(target_id).is_none() {
        return Err(Error::AccountNotFound);
    }
    let refresh_token = vault::get(target_id)?.ok_or(Error::AccountNotFound)?;
    let client = orbit.oauth_client().await?;
    let grant = match oauth::refresh(&orbit.http, &client, &refresh_token).await {
        Err(Error::TokenRevoked) => {
            orbit.update(|state| {
                state
                    .quotas
                    .insert(target_id.to_owned(), QuotaState::Reauth)
            });
            return Err(Error::TokenRevoked);
        }
        other => other?,
    };

    let previous = live::read()?;
    save_back(orbit, previous.as_ref())?;
    let auth_method = previous
        .as_ref()
        .and_then(|bundle| bundle.auth_method.clone())
        .unwrap_or_else(|| CONSUMER_AUTH_METHOD.to_owned());
    let bundle = Bundle::from_grant(&grant, &refresh_token, &auth_method);

    orbit.set_operation(step(SwitchStep::Closing));
    process::stop_all(Some(&install)).await?;

    orbit.set_operation(step(SwitchStep::Writing));
    if let Err(err) = write_and_verify(&bundle, target_id) {
        log::error!("switch failed while writing, restoring the previous sign-in: {err}");
        let restored = match &previous {
            Some(previous) => live::write(previous),
            None => live::clear(),
        };
        if let Err(restore_err) = restored {
            log::error!("restoring the previous sign-in failed: {restore_err}");
            return Err(Error::RollbackFailed);
        }
        let _ = process::launch(&install).await;
        return Err(err);
    }
    orbit
        .tokens
        .put(target_id, grant.access_token.clone(), grant.expires_in);
    orbit.update(|state| -> Result<()> {
        state.active_id = Some(target_id.to_owned());
        state
            .registry
            .set_live_fingerprint(registry::fingerprint(&refresh_token))
    })?;
    log::info!("switched Antigravity to another account");

    orbit.set_operation(step(SwitchStep::Launching));
    let launched = process::launch(&install).await;
    drop(sync);
    drop(guard);
    orbit.refresh_status();
    quota::refresh_one(orbit, target_id).await;
    launched
}

/// Antigravity may hold a newer refresh token for the account it is signed
/// in with (for example after signing in again inside the IDE). Keep it.
fn save_back(orbit: &Orbit, previous: Option<&Bundle>) -> Result<()> {
    let Some((bundle, claims)) = previous.and_then(|b| b.claims().map(|c| (b, c))) else {
        return Ok(());
    };
    let known = orbit.state().registry.get(&claims.sub).is_some();
    if known && !bundle.token.refresh_token.is_empty() {
        vault::set(&claims.sub, &bundle.token.refresh_token)?;
    }
    Ok(())
}

fn write_and_verify(bundle: &Bundle, target_id: &str) -> Result<()> {
    live::write(bundle)?;
    let written = live::read()?.ok_or(Error::VerifyFailed)?;
    let matches = written.token.refresh_token == bundle.token.refresh_token
        && written
            .claims()
            .is_none_or(|claims| claims.sub == target_id);
    if matches {
        Ok(())
    } else {
        Err(Error::VerifyFailed)
    }
}

pub async fn stop(orbit: &Arc<Orbit>) -> Result<()> {
    let _guard = orbit.begin(Operation::Stopping {
        provider: Provider::Antigravity,
    })?;
    let install = orbit.install().ok();
    let result = process::stop_all(install.as_ref()).await;
    orbit.refresh_status();
    result
}

pub async fn restart(orbit: &Arc<Orbit>) -> Result<()> {
    let _guard = orbit.begin(Operation::Restarting {
        provider: Provider::Antigravity,
    })?;
    let install = orbit.install()?;
    process::stop_all(Some(&install)).await?;
    let result = process::launch(&install).await;
    orbit.refresh_status();
    result
}
