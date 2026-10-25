//! Finding, stopping and launching Antigravity processes.
//!
//! Every process that may hold the signed-in credential has to exit before a
//! switch: the IDE, its helpers and language server, and the `agy` CLI. Any
//! of them could otherwise write the previous account back when it refreshes
//! its token.

use std::path::Path;
use std::time::{Duration, Instant};

use sysinfo::{Pid, Process, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::antigravity::install::Install;
use crate::error::{Error, Result};
pub use crate::model::{ProcessKind, RelatedProcess};

const GRACEFUL_TIMEOUT: Duration = Duration::from_secs(8);
const FORCE_TIMEOUT: Duration = Duration::from_secs(3);
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(15);
const POLL: Duration = Duration::from_millis(250);

fn snapshot() -> System {
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
    );
    system
}

fn classify(install: Option<&Install>, process: &Process) -> Option<ProcessKind> {
    let exe = process.exe()?;
    let file_name = exe.file_name()?.to_string_lossy().to_ascii_lowercase();
    if file_name == "agy" || file_name == "agy.exe" {
        return Some(ProcessKind::Cli);
    }
    let install = install?;
    if !install.contains(exe) {
        return None;
    }
    Some(if exe == install.main_exe {
        ProcessKind::App
    } else if file_name.starts_with("language_server") {
        ProcessKind::LanguageServer
    } else {
        ProcessKind::Helper
    })
}

fn related(system: &System, install: Option<&Install>) -> Vec<(Pid, ProcessKind)> {
    system
        .processes()
        .iter()
        .filter_map(|(pid, process)| classify(install, process).map(|kind| (*pid, kind)))
        .collect()
}

/// The Antigravity processes running right now.
pub fn scan(install: Option<&Install>) -> Vec<RelatedProcess> {
    related(&snapshot(), install)
        .into_iter()
        .map(|(pid, kind)| RelatedProcess {
            pid: pid.as_u32(),
            kind,
        })
        .collect()
}

pub fn is_app_running(install: Option<&Install>) -> bool {
    scan(install).iter().any(|p| p.kind == ProcessKind::App)
}

/// Asks the IDE and CLI to quit, then force-kills whatever is left.
pub async fn stop_all(install: Option<&Install>) -> Result<()> {
    let system = snapshot();
    for (pid, kind) in related(&system, install) {
        if matches!(kind, ProcessKind::App | ProcessKind::Cli)
            && let Some(process) = system.process(pid)
        {
            request_exit(process);
        }
    }
    if wait_until(GRACEFUL_TIMEOUT, || scan(install).is_empty()).await {
        return Ok(());
    }
    let system = snapshot();
    let survivors = related(&system, install);
    log::warn!("force-killing {} Antigravity process(es)", survivors.len());
    for (pid, _) in &survivors {
        if let Some(process) = system.process(*pid) {
            process.kill();
        }
    }
    if wait_until(FORCE_TIMEOUT, || scan(install).is_empty()).await {
        Ok(())
    } else {
        Err(Error::ProcessStuck(scan(install).len()))
    }
}

/// Launches the IDE and waits until its main process is up.
pub async fn launch(install: &Install) -> Result<()> {
    spawn_app(&install.root, &install.main_exe).map_err(|err| Error::Launch(err.to_string()))?;
    if wait_until(LAUNCH_TIMEOUT, || is_app_running(Some(install))).await {
        Ok(())
    } else {
        Err(Error::Launch("Antigravity did not start in time".into()))
    }
}

async fn wait_until(timeout: Duration, mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if done() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(POLL).await;
    }
}

#[cfg(unix)]
fn request_exit(process: &Process) {
    // SIGTERM lets Electron and the CLI shut down cleanly; an Apple Event
    // quit could block on dialogs and needs Automation permission.
    process.kill_with(sysinfo::Signal::Term);
}

#[cfg(windows)]
fn request_exit(process: &Process) {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;
    // Without /F, taskkill posts WM_CLOSE so the app can close its windows.
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &process.pid().as_u32().to_string(), "/T"])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
}

#[cfg(target_os = "macos")]
fn spawn_app(root: &Path, _main_exe: &Path) -> std::io::Result<()> {
    let status = std::process::Command::new("/usr/bin/open")
        .arg(root)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("open exited with {status}")))
    }
}

#[cfg(windows)]
fn spawn_app(root: &Path, main_exe: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS};
    std::process::Command::new(main_exe)
        .current_dir(root)
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .map(drop)
}

#[cfg(not(any(target_os = "macos", windows)))]
fn spawn_app(root: &Path, main_exe: &Path) -> std::io::Result<()> {
    std::process::Command::new(main_exe)
        .current_dir(root)
        .spawn()
        .map(drop)
}
