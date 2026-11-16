//! Isolated official-CLI browser sign-in subprocess and secret-free failure classification.

use super::install::Cli;
use crate::{
    error::{Error, Result},
    provider::Provider,
};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, sync::oneshot};

const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);
const STDERR_CAPTURE_LIMIT: usize = 4096;

const REMOVED_ENV_KEYS: &[&str] = &[
    "OPENAI_API_KEY",
    "CODEX_ACCESS_TOKEN",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
];

async fn drain_stderr(stderr: Option<tokio::process::ChildStderr>) -> Vec<u8> {
    let Some(mut stderr) = stderr else {
        return Vec::new();
    };
    let mut captured = Vec::new();
    let mut scratch = [0u8; 1024];
    loop {
        match stderr.read(&mut scratch).await {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                let remaining = STDERR_CAPTURE_LIMIT.saturating_sub(captured.len());
                if remaining > 0 {
                    captured.extend_from_slice(&scratch[..read.min(remaining)]);
                }
            }
        }
    }
    captured
}

fn mentions_missing_binary(exit_code: Option<i32>, lower: &str, name: &str) -> bool {
    if matches!(exit_code, Some(127 | 9009)) && lower.contains(name) {
        return true;
    }
    [
        format!("env: {name}:"),
        format!("env: '{name}':"),
        format!("env: ‘{name}’:"),
        format!("{name}: command not found"),
        format!("{name}: not found"),
        format!("'{name}' is not recognized"),
    ]
    .iter()
    .any(|pattern| lower.contains(pattern))
}

/// Maps a non-zero CLI exit to a stable error without echoing raw `stderr` (which may hold URLs or tokens).
fn classify_failure(exit_code: Option<i32>, stderr: &[u8]) -> Error {
    let text = String::from_utf8_lossy(stderr);
    let lower = text.to_ascii_lowercase();

    if mentions_missing_binary(exit_code, &lower, "node") {
        return Error::CliRuntime("missing Node.js runtime required by the installed CLI".into());
    }
    if mentions_missing_binary(exit_code, &lower, "bun") {
        return Error::CliRuntime("missing Bun runtime required by the installed CLI".into());
    }
    if text.contains("ERR_MODULE_NOT_FOUND") || text.contains("Cannot find module") {
        return Error::CliRuntime("the installed CLI package is incomplete".into());
    }
    if exit_code == Some(127) {
        return Error::CliRuntime(
            "required CLI runtime or executable was not found (exit 127)".into(),
        );
    }
    if exit_code == Some(126)
        || lower.contains("dyld[")
        || lower.contains("library not loaded:")
        || lower.contains("exec format error")
    {
        return Error::CliRuntime("the installed CLI could not be executed".into());
    }

    Error::OAuthDenied("the official client did not complete sign-in".into())
}

pub async fn run(
    provider: Provider,
    cli: &Cli,
    root: &Path,
    cancel: oneshot::Receiver<()>,
) -> Result<()> {
    run_with_timeout(provider, cli, root, cancel, LOGIN_TIMEOUT).await
}

async fn run_with_timeout(
    provider: Provider,
    cli: &Cli,
    root: &Path,
    cancel: oneshot::Receiver<()>,
    timeout: Duration,
) -> Result<()> {
    let mut cmd = tokio::process::Command::new(&cli.program);
    cmd.kill_on_drop(true)
        .current_dir(root)
        .env("PATH", &cli.path_env)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    for key in REMOVED_ENV_KEYS {
        cmd.env_remove(key);
    }
    if provider == Provider::Codex {
        cmd.env("CODEX_HOME", root)
            .args(["login", "-c", "cli_auth_credentials_store=\"file\""]);
    } else {
        cmd.env("CLAUDE_CONFIG_DIR", root)
            .env("CLAUDE_SECURESTORAGE_CONFIG_DIR", root)
            .args(["auth", "login"]);
    }
    let mut child = cmd.spawn().map_err(|err| {
        Error::CliRuntime(format!(
            "could not start the installed CLI ({:?})",
            err.kind()
        ))
    })?;
    let stderr_task = tokio::spawn(drain_stderr(child.stderr.take()));
    let status = tokio::select! {
        status = child.wait() => status.map_err(|err| {
            Error::CliRuntime(format!("failed while waiting for the CLI ({:?})", err.kind()))
        })?,
        _ = cancel => {
            let _ = child.kill().await;
            stderr_task.abort();
            return Err(Error::OAuthCancelled);
        }
        _ = tokio::time::sleep(timeout) => {
            let _ = child.kill().await;
            stderr_task.abort();
            return Err(Error::OAuthTimeout);
        }
    };
    if status.success() {
        stderr_task.abort();
        return Ok(());
    }
    let stderr = tokio::time::timeout(Duration::from_millis(200), stderr_task)
        .await
        .ok()
        .and_then(std::result::Result::ok)
        .unwrap_or_default();
    Err(classify_failure(status.code(), &stderr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_node_error_is_classified_as_cli_runtime_and_redacts_stderr() {
        let stderr = b"env: node: No such file or directory\nhttps://example.com/oauth?state=SECRET_TOKEN&email=alice@example.com";
        let err = classify_failure(Some(127), stderr);
        assert_eq!(err.code(), "cli_runtime");
        let msg = err.to_string();
        assert!(msg.contains("Node.js"));
        assert!(!msg.contains("SECRET_TOKEN"));
        assert!(!msg.contains("alice@example.com"));
        assert!(!msg.contains("https://"));
    }

    #[test]
    fn broken_packages_and_non_executable_launchers_map_to_cli_runtime() {
        for (code, raw) in [
            (
                Some(1),
                "Error [ERR_MODULE_NOT_FOUND]: Cannot find module '/Users/private/pkg'",
            ),
            (
                Some(126),
                "zsh: permission denied: /Users/private/bin/codex",
            ),
            (Some(127), "sh: unknown-runtime: not found"),
            (
                Some(9009),
                "'node' is not recognized as an internal or external command",
            ),
        ] {
            let err = classify_failure(code, raw.as_bytes());
            assert_eq!(err.code(), "cli_runtime", "failed for {raw}");
            assert!(!err.to_string().contains("/Users/private"));
        }
    }

    #[test]
    fn regular_sign_in_exit_remains_oauth_denied_without_leaking_stderr() {
        let stderr = b"Sign-in failed for bob@example.com with code= SENSITIVE_GRANT";
        let err = classify_failure(Some(1), stderr);
        assert_eq!(err.code(), "oauth_denied");
        let msg = err.to_string();
        assert!(!msg.contains("bob@example.com"));
        assert!(!msg.contains("SENSITIVE_GRANT"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn npm_env_node_launcher_succeeds_when_node_is_only_in_cli_path_env() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let node_dir = temp.path().join("homebrew/bin");
        let pkg_dir = temp
            .path()
            .join("homebrew/lib/node_modules/@openai/codex/bin");
        let login_root = temp.path().join("login-home");
        std::fs::create_dir_all(&node_dir).unwrap();
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::create_dir_all(&login_root).unwrap();

        let fake_node = node_dir.join("node");
        std::fs::write(&fake_node, "#!/bin/sh\nset -eu\nexec \"/bin/sh\" \"$@\"\n").unwrap();
        std::fs::set_permissions(&fake_node, std::fs::Permissions::from_mode(0o755)).unwrap();

        let codex_js = pkg_dir.join("codex.js");
        std::fs::write(
            &codex_js,
            "#!/usr/bin/env node\nset -eu\ntest \"$1\" = \"login\"\ntest -z \"${OPENAI_API_KEY:-}\"\npwd > \"$CODEX_HOME/cwd.txt\"\necho '{\"ok\":true}' > \"$CODEX_HOME/auth.json\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&codex_js, std::fs::Permissions::from_mode(0o755)).unwrap();

        let symlink = node_dir.join("codex");
        std::os::unix::fs::symlink(&codex_js, &symlink).unwrap();

        let path_env = std::env::join_paths([
            std::path::Path::new("/usr/bin"),
            std::path::Path::new("/bin"),
            node_dir.as_path(),
        ])
        .unwrap();
        let cli = Cli {
            program: symlink,
            canonical: codex_js.canonicalize().unwrap(),
            path_env,
        };

        let (_tx, rx) = oneshot::channel();
        run(Provider::Codex, &cli, &login_root, rx).await.unwrap();
        assert!(login_root.join("auth.json").is_file());
        let recorded_cwd = std::fs::read_to_string(login_root.join("cwd.txt")).unwrap();
        assert_eq!(
            std::path::PathBuf::from(recorded_cwd.trim())
                .canonicalize()
                .unwrap(),
            login_root.canonicalize().unwrap()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn missing_env_runtime_surfaces_cli_runtime_instead_of_oauth_denied() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let launcher = temp.path().join("codex");
        let login_root = temp.path().join("login-home");
        std::fs::create_dir_all(&login_root).unwrap();
        std::fs::write(&launcher, "#!/usr/bin/env orbitswitch_missing_node_test\n").unwrap();
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755)).unwrap();

        // Include "node" in the missing interpreter message the exact way `/usr/bin/env node` fails
        // when `node` is absent from `PATH`.
        let wrapper = temp.path().join("codex-wrapper");
        std::fs::write(
            &wrapper,
            "#!/bin/sh\nPATH=/usr/bin:/bin /usr/bin/env node --version\n",
        )
        .unwrap();
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();

        for prog in [&launcher, &wrapper] {
            let cli = Cli {
                program: prog.clone(),
                canonical: prog.clone(),
                path_env: std::ffi::OsString::from("/usr/bin:/bin"),
            };
            let (_tx, rx) = oneshot::channel();
            let err = run(Provider::Codex, &cli, &login_root, rx)
                .await
                .unwrap_err();
            assert_eq!(err.code(), "cli_runtime");
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn large_stderr_and_cancellation_do_not_deadlock_or_leak() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let login_root = temp.path().join("login-home");
        std::fs::create_dir_all(&login_root).unwrap();

        // 1) Chatty stderr > 128 KiB must drain without blocking pipe buffers or leaking output.
        let chatty = temp.path().join("chatty");
        std::fs::write(
            &chatty,
            "#!/bin/sh\nawk 'BEGIN { for (i = 0; i < 4000; i++) print \"SECRET_STDERR_PAYLOAD_1234567890\" }' >&2\nexit 1\n",
        )
        .unwrap();
        std::fs::set_permissions(&chatty, std::fs::Permissions::from_mode(0o755)).unwrap();
        let cli = Cli {
            program: chatty.clone(),
            canonical: chatty,
            path_env: std::ffi::OsString::from("/usr/bin:/bin"),
        };
        let (_tx, rx) = oneshot::channel();
        let err = run_with_timeout(
            Provider::Claude,
            &cli,
            &login_root,
            rx,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err();
        assert_eq!(err.code(), "oauth_denied");
        assert!(!err.to_string().contains("SECRET_STDERR_PAYLOAD"));

        // 2) Cancellation terminates a waiting CLI promptly.
        let sleeper = temp.path().join("sleeper");
        std::fs::write(&sleeper, "#!/bin/sh\nsleep 60\n").unwrap();
        std::fs::set_permissions(&sleeper, std::fs::Permissions::from_mode(0o755)).unwrap();
        let cli = Cli {
            program: sleeper.clone(),
            canonical: sleeper,
            path_env: std::ffi::OsString::from("/usr/bin:/bin"),
        };
        let (tx, rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            run_with_timeout(
                Provider::Codex,
                &cli,
                &login_root,
                rx,
                Duration::from_secs(30),
            )
            .await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        let _ = tx.send(());
        let err = task.await.unwrap().unwrap_err();
        assert!(matches!(err, Error::OAuthCancelled));
    }
}
