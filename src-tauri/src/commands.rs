//! Tauri commands: the frontend's only entry points. Each one delegates to
//! the module that owns the behavior.

use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::antigravity::process;
use crate::error::{Error, Result};
use crate::model::{RelatedProcess, Snapshot, UpdateInfo};
use crate::state::Orbit;
use crate::{accounts, autostart, quota, scheduler, shell, switcher, updater};
use crate::{native, provider::Provider};

type Shared<'a> = State<'a, Arc<Orbit>>;

/// Quotas younger than this are not refetched when the surface opens.
const STALE_AFTER: Duration = Duration::from_secs(60);

#[tauri::command]
pub fn get_snapshot(orbit: Shared<'_>) -> Snapshot {
    orbit.snapshot()
}

#[tauri::command]
pub async fn refresh(orbit: Shared<'_>, force: bool) -> Result<()> {
    if force {
        scheduler::tick(&orbit).await;
    } else {
        quota::refresh_if_stale(&orbit, STALE_AFTER).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn add_account(orbit: Shared<'_>, provider: Provider) -> Result<String> {
    if provider == Provider::Antigravity {
        accounts::add(&orbit).await
    } else {
        native::add(&orbit, provider).await
    }
}

#[tauri::command]
pub fn cancel_add_account(orbit: Shared<'_>) {
    accounts::cancel_add(&orbit);
}

#[tauri::command]
pub fn copy_sign_in_link(orbit: Shared<'_>) -> Result<()> {
    accounts::copy_sign_in_link(&orbit)
}

#[tauri::command]
pub async fn remove_account(orbit: Shared<'_>, id: String) -> Result<()> {
    let provider = orbit
        .state()
        .registry
        .get(&id)
        .ok_or(Error::AccountNotFound)?
        .provider;
    if provider == Provider::Antigravity {
        accounts::remove(&orbit, &id).await
    } else {
        native::remove(&orbit, &id).await
    }
}

#[tauri::command]
pub async fn switch_account(orbit: Shared<'_>, id: String) -> Result<()> {
    let provider = orbit
        .state()
        .registry
        .get(&id)
        .ok_or(Error::AccountNotFound)?
        .provider;
    if provider == Provider::Antigravity {
        switcher::switch(&orbit, &id).await
    } else {
        native::switch(&orbit, provider, &id).await
    }
}

#[tauri::command]
pub fn running_processes(orbit: Shared<'_>, provider: Provider) -> Vec<RelatedProcess> {
    if provider == Provider::Antigravity {
        process::scan(orbit.install().ok().as_ref())
    } else {
        native::process::scan(provider, &native::install::locate(provider))
    }
}

#[tauri::command]
pub async fn stop_client(orbit: Shared<'_>, provider: Provider) -> Result<()> {
    if provider == Provider::Antigravity {
        switcher::stop(&orbit).await
    } else {
        native::control(&orbit, provider, false).await
    }
}

#[tauri::command]
pub async fn restart_client(orbit: Shared<'_>, provider: Provider) -> Result<()> {
    if provider == Provider::Antigravity {
        switcher::restart(&orbit).await
    } else {
        native::control(&orbit, provider, true).await
    }
}

#[tauri::command]
pub fn resize_panel(app: AppHandle, height: f64) {
    shell::resize(&app, height);
}

#[tauri::command]
pub fn reveal_logs(app: AppHandle) -> Result<()> {
    let dir = app
        .path()
        .app_log_dir()
        .map_err(|err| Error::Invalid(err.to_string()))?;
    std::fs::create_dir_all(&dir)?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|err| Error::Invalid(err.to_string()))
}

#[tauri::command]
pub fn autostart_enabled() -> Result<bool> {
    autostart::is_enabled()
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<()> {
    autostart::set_enabled(enabled)
}

#[tauri::command]
pub async fn check_update(orbit: Shared<'_>) -> Result<Option<UpdateInfo>> {
    let result = updater::check(&orbit, true).await;
    if let Err(err) = &result {
        log::warn!("could not check for app updates: {err}");
    }
    result
}

#[tauri::command]
pub async fn install_update(orbit: Shared<'_>, version: String) -> Result<()> {
    let result = updater::install(&orbit, &version).await;
    if let Err(err) = &result {
        log::warn!("could not install app update: {err}");
    }
    result
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub async fn import_account(
    orbit: Shared<'_>,
    provider: Provider,
    expected_id: Option<String>,
) -> Result<String> {
    if provider == Provider::Antigravity {
        accounts::sync_live(&orbit).await?;
        orbit
            .state()
            .active_id
            .clone()
            .ok_or(Error::AccountNotFound)
    } else {
        native::import(&orbit, provider, expected_id.as_deref()).await
    }
}
