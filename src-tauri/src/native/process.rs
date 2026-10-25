//! Process changes are called only by confirmed account/process commands.
use super::install::{Install, contains};
use crate::{
    antigravity::process::{ProcessKind, RelatedProcess},
    error::{Error, Result},
    provider::Provider,
};
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
fn system() -> System {
    let mut s = System::new();
    s.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
    );
    s
}
fn kind(_provider: Provider, install: &Install, exe: &std::path::Path) -> Option<ProcessKind> {
    if install.executable.as_deref() == Some(exe) {
        return Some(ProcessKind::App);
    }
    if install.app.as_ref().is_some_and(|root| contains(root, exe)) {
        return Some(ProcessKind::Helper);
    }
    if install.cli.as_deref() == Some(exe) {
        return Some(ProcessKind::Cli);
    }
    None
}
pub fn scan(provider: Provider, install: &Install) -> Vec<RelatedProcess> {
    system()
        .processes()
        .iter()
        .filter_map(|(pid, p)| {
            kind(provider, install, p.exe()?).map(|kind| RelatedProcess {
                pid: pid.as_u32(),
                kind,
            })
        })
        .collect()
}
pub fn is_running(provider: Provider, install: &Install) -> bool {
    !scan(provider, install).is_empty()
}
pub async fn stop(provider: Provider, install: &Install) -> Result<()> {
    let s = system();
    for p in s.processes().values().filter(|p| {
        p.exe()
            .is_some_and(|e| kind(provider, install, e).is_some())
    }) {
        #[cfg(unix)]
        {
            p.kill_with(sysinfo::Signal::Term);
        }
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &p.pid().as_u32().to_string(), "/T"])
                .status();
        }
    }
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if scan(provider, install).is_empty() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    // Do not force-kill a coding agent whose work may not be saved.
    Err(Error::ProcessStuck(scan(provider, install).len()))
}
pub async fn launch(install: &Install) -> Result<()> {
    let Some(app) = &install.app else {
        return Ok(());
    };
    #[cfg(target_os = "macos")]
    {
        let status = tokio::process::Command::new("/usr/bin/open")
            .arg(app)
            .status()
            .await
            .map_err(|_| Error::Launch("could not open the client".into()))?;
        if !status.success() {
            return Err(Error::Launch(
                "the system rejected opening the client".into(),
            ));
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        tokio::process::Command::new(install.executable.as_ref().unwrap_or(app))
            .spawn()
            .map_err(|_| Error::Launch("could not open the client".into()))?;
    }
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if system()
            .processes()
            .values()
            .any(|p| p.exe() == install.executable.as_deref())
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Err(Error::Launch("the main process did not appear".into()))
}
