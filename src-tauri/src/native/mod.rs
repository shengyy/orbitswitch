//! Shared orchestration for Codex and Claude; credentials never enter snapshots.
pub mod api;
pub mod desktop;
pub mod install;
pub mod live;
pub mod process;
pub mod storage;
mod transaction;
use crate::{
    error::{Error, Result},
    model::{Operation, QuotaState, SwitchStep},
    provider::Provider,
    registry::{self, Account},
    state::Orbit,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::sync::oneshot;
#[derive(Clone, Serialize, Deserialize)]
struct Session {
    credential: Value,
    profile: Value,
    desktop: Option<desktop::Session>,
}
impl Session {
    fn identity(&self, provider: Provider) -> Result<(String, String, String, Option<String>)> {
        api::identity(provider, &self.credential, &self.profile)
    }
    fn id(&self, provider: Provider) -> Result<String> {
        let (s, o, _, _) = self.identity(provider)?;
        Ok(format!("{}:{s}:{o}", provider.key()))
    }
    fn save(&self, id: &str) -> Result<()> {
        storage::set(id, &serde_json::to_string(self)?)
    }
    fn keep_desktop(mut self, saved: Option<&Session>) -> Self {
        if self.desktop.is_none() {
            self.desktop = saved.and_then(|session| session.desktop.clone());
        }
        self
    }
}
fn load(id: &str) -> Result<Session> {
    serde_json::from_str(&storage::get(id)?.ok_or(Error::AccountNotFound)?).map_err(Error::from)
}
fn fingerprint(value: &Value) -> String {
    registry::fingerprint(&value.to_string())
}
async fn read(
    orbit: &Orbit,
    provider: Provider,
    location: &live::Location,
) -> Result<Option<Session>> {
    let Some(credential) = location.read()? else {
        return Ok(None);
    };
    api::access(provider, &credential)?;
    let profile = api::profile(&orbit.http, provider, &credential).await?;
    let (subject, org, _, _) = api::identity(provider, &credential, &profile)?;
    let desktop = if provider == Provider::Claude {
        match desktop::capture()? {
            Some(session) if session.matches(&subject) => {
                if session.verify(&orbit.http, &subject, &org).await.is_ok() {
                    Some(session)
                } else {
                    None
                }
            }
            _ => None,
        }
    } else {
        None
    };
    Ok(Some(Session {
        credential,
        profile,
        desktop,
    }))
}
fn save_account(orbit: &Orbit, provider: Provider, session: &Session) -> Result<String> {
    let id = session.id(provider)?;
    let (_, _, email, name) = session.identity(provider)?;
    let saved = session.clone().keep_desktop(load(&id).ok().as_ref());
    saved.save(&id)?;
    orbit.update(|state| {
        state.registry.upsert(Account {
            provider,
            desktop_ready: provider == Provider::Codex || saved.desktop.is_some(),
            id: id.clone(),
            email,
            name,
            picture: None,
            plan: api::plan(provider, &session.profile, None),
            added_at: Utc::now(),
        })
    })?;
    Ok(id)
}
pub async fn sync_live(orbit: &Arc<Orbit>, provider: Provider) -> Result<()> {
    let Ok(_sync) = orbit.sync_lock.try_lock() else {
        return Ok(());
    };
    if !orbit.is_idle() {
        return Ok(());
    }
    let desktop_installed = orbit
        .state()
        .native_clients
        .get(&provider)
        .is_some_and(|(client, _)| client.app.is_some());
    let (location, raw) = match live::locate(provider).and_then(|location| {
        location.check_switch_configuration(desktop_installed)?;
        location.read().map(|raw| (location, raw))
    }) {
        Ok(value) => value,
        Err(error) => {
            orbit.update(|s| {
                s.native_active.insert(provider, None);
                s.native_aligned.insert(provider, false);
            });
            return Err(error);
        }
    };
    let Some(raw) = raw else {
        orbit.update(|s| {
            s.native_active.insert(provider, None);
            s.native_aligned.insert(provider, false);
        });
        return Ok(());
    };
    let fp = fingerprint(&raw);
    let unchanged = {
        let state = orbit.state();
        state.registry.native_fingerprint(provider) == Some(fp.as_str())
            && state.native_active.contains_key(&provider)
    };
    if unchanged && provider == Provider::Codex {
        return Ok(());
    }
    let session = match read(orbit, provider, &location).await {
        Ok(Some(session)) => session,
        result => {
            orbit.update(|s| {
                s.native_active.insert(provider, None);
                s.native_aligned.insert(provider, false);
            });
            return result.map(|_| ());
        }
    };
    let id = session.id(provider)?;
    let should_adopt = !unchanged || provider == Provider::Claude;
    if should_adopt && !orbit.state().registry.ignores_native(provider, &id) {
        save_account(orbit, provider, &session)?;
    }
    orbit.update(|s| {
        s.native_aligned.insert(
            provider,
            provider == Provider::Codex || session.desktop.is_some(),
        );
        s.native_active.insert(provider, Some(id));
        s.registry.set_native_fingerprint(provider, fp)
    })?;
    Ok(())
}
pub async fn import(
    orbit: &Arc<Orbit>,
    provider: Provider,
    expected_id: Option<&str>,
) -> Result<String> {
    let guard = orbit.begin(Operation::Adding { provider })?;
    let sync = orbit.sync_lock.lock().await;
    let location = live::locate(provider)?;
    location.check_switch_configuration(install::locate(provider).app.is_some())?;
    let session = read(orbit, provider, &location)
        .await?
        .ok_or(Error::SubscriptionLoginRequired)?;
    if let Some(desktop) = &session.desktop {
        let (subject, org, _, _) = session.identity(provider)?;
        desktop.verify(&orbit.http, &subject, &org).await?;
    }
    let id = session.id(provider)?;
    if expected_id.is_some_and(|expected| expected != id) {
        return Err(Error::VerifyFailed);
    }
    if expected_id.is_some() && session.desktop.is_none() {
        return Err(Error::DesktopSignInRequired);
    }
    save_account(orbit, provider, &session)?;
    orbit.update(|s| {
        s.native_aligned.insert(
            provider,
            provider == Provider::Codex || session.desktop.is_some(),
        );
        s.native_active.insert(provider, Some(id.clone()));
        s.registry
            .set_native_fingerprint(provider, fingerprint(&session.credential))
    })?;
    drop(sync);
    drop(guard);
    orbit.refresh_status();
    crate::quota::refresh_all(orbit).await;
    Ok(id)
}
struct LoginDir(live::Location);
impl Drop for LoginDir {
    fn drop(&mut self) {
        live::cleanup(&self.0);
    }
}
pub async fn add(orbit: &Arc<Orbit>, provider: Provider) -> Result<String> {
    let guard = orbit.begin(Operation::Adding { provider })?;
    let sync = orbit.sync_lock.lock().await;
    let cli = install::locate(provider).cli.ok_or(Error::ClientNotFound)?;
    let root = std::env::temp_dir().join(format!(
        "orbitswitch-login-{}",
        crate::google::oauth::random_token(16)?
    ));
    std::fs::create_dir(&root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))?;
    }
    let dir = LoginDir(live::location(provider, root.clone(), true)?);
    let mut cmd = tokio::process::Command::new(cli);
    cmd.kill_on_drop(true)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    for key in [
        "OPENAI_API_KEY",
        "CODEX_ACCESS_TOKEN",
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "CLAUDE_CODE_OAUTH_TOKEN",
        "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
        "CLAUDE_CODE_USE_BEDROCK",
        "CLAUDE_CODE_USE_VERTEX",
        "CLAUDE_CODE_USE_FOUNDRY",
    ] {
        cmd.env_remove(key);
    }
    if provider == Provider::Codex {
        cmd.env("CODEX_HOME", &root)
            .args(["login", "-c", "cli_auth_credentials_store=\"file\""]);
    } else {
        cmd.env("CLAUDE_CONFIG_DIR", &root)
            .env("CLAUDE_SECURESTORAGE_CONFIG_DIR", &root)
            .args(["auth", "login"]);
    }
    let mut child = cmd.spawn()?;
    let (tx, rx) = oneshot::channel();
    orbit.set_pending_auth(tx);
    let status = tokio::select! {s=child.wait()=>s?,_=rx=>{let _=child.kill().await;return Err(Error::OAuthCancelled);},_=tokio::time::sleep(Duration::from_secs(300))=>{let _=child.kill().await;return Err(Error::OAuthTimeout);}};
    if !status.success() {
        return Err(Error::OAuthDenied(
            "the official client did not complete sign-in".into(),
        ));
    }
    let session = read(orbit, provider, &dir.0)
        .await?
        .ok_or(Error::SubscriptionLoginRequired)?;
    let id = save_account(orbit, provider, &session)?;
    drop(dir);
    drop(sync);
    drop(guard);
    orbit.refresh_status();
    crate::quota::refresh_all(orbit).await;
    Ok(id)
}
pub async fn remove(orbit: &Arc<Orbit>, id: &str) -> Result<()> {
    let provider = orbit
        .state()
        .registry
        .get(id)
        .ok_or(Error::AccountNotFound)?
        .provider;
    let _guard = orbit.begin(Operation::Adding { provider })?;
    let _sync = orbit.sync_lock.lock().await;
    orbit.update(|s| s.registry.ignore_native(provider, id))?;
    storage::delete(id)?;
    orbit.update(|s| {
        s.quotas.remove(id);
        s.registry.remove(id)
    })
}
pub async fn refresh(orbit: &Arc<Orbit>, provider: Provider, id: &str) {
    let result = async {
        let mut session = load(id)?;
        let active = orbit
            .state()
            .native_active
            .get(&provider)
            .and_then(|s| s.as_deref())
            == Some(id);
        if api::expires(provider, &session.credential)
            .is_some_and(|at| at <= Utc::now().timestamp() + 60)
        {
            if active {
                return Err(Error::LiveTokenExpired);
            }
            // Resolve the current native identity again immediately before consuming an inactive refresh token.
            // External sign-ins can occur between the scheduler pass and this request.
            let location = live::locate(provider)?;
            if let Some(live) = location.read()? {
                let profile = api::profile(&orbit.http, provider, &live)
                    .await
                    .map_err(|_| Error::LiveTokenExpired)?;
                let (subject, org, _, _) = api::identity(provider, &live, &profile)?;
                if format!("{}:{subject}:{org}", provider.key()) == id {
                    return Err(Error::LiveTokenExpired);
                }
            }
            api::renew(&orbit.http, provider, &mut session.credential).await?;
            if session.id(provider)? != id {
                return Err(Error::VerifyFailed);
            }
            session.save(id)?;
        }
        let (profile, usage) = tokio::join!(
            api::profile(&orbit.http, provider, &session.credential),
            api::usage(&orbit.http, provider, &session.credential)
        );
        let profile = profile.unwrap_or_else(|_| session.profile.clone());
        let observed = api::identity(provider, &session.credential, &profile)?;
        let saved = session.identity(provider)?;
        if observed.0 != saved.0 || observed.1 != saved.1 {
            return Err(Error::VerifyFailed);
        }
        if let Ok(u) = &usage {
            api::verify_usage_identity(provider, &session.credential, u)?;
        }
        let plan = api::plan(provider, &profile, usage.as_ref().ok());
        orbit.update(|s| {
            if let Some(plan) = plan
                && s.registry.get(id).is_some()
            {
                let _ = s.registry.set_plan(id, plan);
            }
        });
        usage.map(|u| api::groups(provider, &u))
    }
    .await;
    orbit.update(|s| {
        if s.registry.get(id).is_none() {
            return;
        }
        match result {
            Ok(groups) => {
                s.quotas.insert(
                    id.into(),
                    QuotaState::Ready {
                        groups,
                        fetched_at: Utc::now(),
                        stale: false,
                    },
                );
            }
            Err(Error::TokenRevoked) => {
                s.quotas.insert(id.into(), QuotaState::Reauth);
            }
            Err(e) => {
                if let Some(QuotaState::Ready { stale, .. }) = s.quotas.get_mut(id) {
                    *stale = true;
                } else {
                    s.quotas.insert(
                        id.into(),
                        QuotaState::Failed {
                            code: e.code().into(),
                            message: e.to_string(),
                        },
                    );
                }
            }
        }
    });
}
pub async fn switch(orbit: &Arc<Orbit>, provider: Provider, id: &str) -> Result<()> {
    let step = |step| Operation::Switching {
        provider,
        target_id: id.into(),
        step,
    };
    let guard = orbit.begin(step(SwitchStep::Preparing))?;
    let sync = orbit.sync_lock.lock().await;
    if orbit
        .state()
        .registry
        .get(id)
        .is_none_or(|a| a.provider != provider)
    {
        return Err(Error::AccountNotFound);
    }
    let install = install::locate(provider);
    if install.cli.is_none() && install.app.is_none() {
        return Err(Error::ClientNotFound);
    }
    let location = live::locate(provider)?;
    location.check_switch_configuration(install.app.is_some())?;
    // Re-read the native sign-in before deciding who owns refresh-token rotation.
    let current = read(orbit, provider, &location).await?;
    let current_id = current.as_ref().map(|s| s.id(provider)).transpose()?;
    let mut target = if current_id.as_deref() == Some(id) {
        // Keep the live CLI grant, but permit a saved desktop pair to repair an external mismatch.
        current.clone().unwrap().keep_desktop(Some(&load(id)?))
    } else {
        load(id)?
    };
    if provider == Provider::Claude && install.app.is_some() && target.desktop.is_none() {
        return Err(Error::DesktopSignInRequired);
    }
    if api::expires(provider, &target.credential)
        .is_some_and(|at| at <= Utc::now().timestamp() + 60)
    {
        if current_id.as_deref() == Some(id) {
            return Err(Error::LiveTokenExpired);
        }
        api::renew(&orbit.http, provider, &mut target.credential).await?;
        target.save(id)?; // A rotated refresh token must survive later network failures.
    }
    target.profile = api::profile(&orbit.http, provider, &target.credential).await?;
    if target.id(provider)? != id {
        return Err(Error::VerifyFailed);
    }
    let usage = api::usage(&orbit.http, provider, &target.credential).await?;
    api::verify_usage_identity(provider, &target.credential, &usage)?;
    if let Some(desktop) = &target.desktop {
        let (subject, org, _, _) = target.identity(provider)?;
        desktop.verify(&orbit.http, &subject, &org).await?;
    }
    let was_running = process::is_running(provider, &install);
    orbit.set_operation(step(SwitchStep::Closing));
    process::stop(provider, &install).await?;
    // Capture after every writer has stopped. Any failure before writing reopens the original app.
    let prepared = async {
        let backup = live::Backup::take(&location)?;
        let previous_desktop = if provider == Provider::Claude {
            desktop::capture()?
        } else {
            None
        };
        if target.desktop.is_some() && previous_desktop.is_none() {
            return Err(Error::DesktopSignInRequired);
        }
        if let Some(previous) = read(orbit, provider, &location).await? {
            let old_id = previous.id(provider)?;
            if orbit.state().registry.get(&old_id).is_some() {
                previous
                    .keep_desktop(load(&old_id).ok().as_ref())
                    .save(&old_id)?;
            }
        }
        Ok((backup, previous_desktop))
    }
    .await;
    let (backup, previous_desktop) = match prepared {
        Ok(value) => value,
        Err(error) => {
            if was_running {
                let _ = process::launch(&install).await;
            }
            return Err(error);
        }
    };
    orbit.set_operation(step(SwitchStep::Writing));
    let result = transaction::apply(
        || {
            location.write(&target.credential, &target.profile)?;
            if let Some(desktop) = &target.desktop {
                desktop::apply(desktop)?;
            }
            let written = location.read()?.ok_or(Error::VerifyFailed)?;
            if api::access(provider, &written)? != api::access(provider, &target.credential)? {
                return Err(Error::VerifyFailed);
            }
            if api::identity(provider, &written, &target.profile)? != target.identity(provider)? {
                return Err(Error::VerifyFailed);
            }
            target.save(id)?;
            orbit.update(|s| {
                s.registry
                    .set_native_fingerprint(provider, fingerprint(&target.credential))
            })?;
            Ok(())
        },
        || {
            // Attempt both restorations even when one fails. Do not launch on a partial rollback.
            let cli = backup.restore();
            let desktop = previous_desktop.as_ref().map(desktop::apply).transpose();
            cli.and(desktop.map(|_| ()))
        },
    );
    if let Err(error) = result {
        if !matches!(error, Error::RollbackFailed) && was_running {
            let _ = process::launch(&install).await;
        }
        return Err(error);
    }
    orbit.update(|s| {
        s.native_aligned.insert(provider, true);
        s.native_active.insert(provider, Some(id.into()));
    });
    orbit.set_operation(step(SwitchStep::Launching));
    let result = process::launch(&install).await.map_err(Error::after_switch);
    drop(sync);
    drop(guard);
    orbit.refresh_status();
    crate::quota::refresh_all(orbit).await;
    result
}
pub async fn control(orbit: &Arc<Orbit>, provider: Provider, restart: bool) -> Result<()> {
    let _guard = orbit.begin(if restart {
        Operation::Restarting { provider }
    } else {
        Operation::Stopping { provider }
    })?;
    let _sync = orbit.sync_lock.lock().await;
    let install = install::locate(provider);
    process::stop(provider, &install).await?;
    let result = if restart {
        process::launch(&install).await
    } else {
        Ok(())
    };
    orbit.refresh_status();
    result
}

#[cfg(test)]
mod system_tests;
