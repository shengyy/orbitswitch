//! Background upkeep: follow Antigravity's sign-in and keep quotas fresh.

use std::sync::Arc;
use std::time::Duration;

use crate::state::Orbit;
use crate::{accounts, quota, updater};

const INTERVAL: Duration = Duration::from_secs(5 * 60);

pub fn start(orbit: Arc<Orbit>) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::join!(tick(&orbit), updater::check_background(&orbit));
            tokio::time::sleep(INTERVAL).await;
        }
    });
}

/// One upkeep pass; skipped while a switch or sign-in is running.
pub async fn tick(orbit: &Arc<Orbit>) {
    if !orbit.is_idle() {
        return;
    }
    let status_orbit = orbit.clone();
    let _ = tokio::task::spawn_blocking(move || status_orbit.refresh_status()).await;
    if let Err(err) = accounts::sync_live(orbit).await {
        log::warn!("could not read Antigravity's sign-in: {err}");
    }
    for provider in [
        crate::provider::Provider::Codex,
        crate::provider::Provider::Claude,
    ] {
        if let Err(err) = crate::native::sync_live(orbit, provider).await {
            log::warn!("could not follow {} sign-in: {err}", provider.key());
        }
    }
    quota::refresh_all(orbit).await;
}
