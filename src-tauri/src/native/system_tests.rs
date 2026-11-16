//! Opt-in read-only probes. Never refresh tokens, launch/stop clients or write credentials.
use super::{api, codex_projects, desktop, install, live, storage};
use crate::provider::Provider;

#[tokio::test]
#[ignore = "reads the signed-in official clients and their usage endpoints"]
async fn current_subscription_contracts() {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    for provider in [Provider::Codex, Provider::Claude] {
        let credential = live::locate(provider)
            .unwrap()
            .read()
            .unwrap()
            .expect("sign in with the official CLI first");
        let profile = api::profile(&http, provider, &credential).await.unwrap();
        api::identity(provider, &credential, &profile).unwrap();
        let usage = api::usage(&http, provider, &credential).await.unwrap();
        let groups = api::groups(provider, &usage);
        assert!(
            !groups.is_empty(),
            "the account should expose at least one rate-limit window"
        );
        api::verify_usage_identity(provider, &credential, &usage).unwrap();
        println!(
            "{}: identity and {} quota groups verified (read-only)",
            provider.key(),
            groups.len()
        );
    }
}

#[tokio::test]
#[ignore = "reads Claude Desktop cookies and verifies the same identity, without modifying them"]
async fn current_claude_desktop_binding() {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let credential = live::locate(Provider::Claude)
        .unwrap()
        .read()
        .unwrap()
        .expect("CLI login required");
    let profile = api::profile(&http, Provider::Claude, &credential)
        .await
        .unwrap();
    let (subject, org, _, _) = api::identity(Provider::Claude, &credential, &profile).unwrap();
    let session = desktop::capture().unwrap().expect("desktop login required");
    assert!(
        session.matches(&subject),
        "Claude Desktop is not signed into the same account as the CLI"
    );
    session.verify(&http, &subject, &org).await.unwrap();
    println!("Claude Desktop and CLI identity match; session verified (read-only)");
}

#[test]
#[ignore = "plans Codex local-project recovery in memory; never writes files or changes processes"]
fn current_codex_local_projects() {
    let location = live::locate(Provider::Codex).unwrap();
    let credential = location.read().unwrap().expect("Codex CLI login required");
    let (user, account, _, _) =
        api::identity(Provider::Codex, &credential, &serde_json::Value::Null).unwrap();
    let id = format!("codex:{user}:{account}");
    #[cfg(target_os = "macos")]
    let data = std::env::home_dir()
        .unwrap()
        .join("Library/Application Support");
    #[cfg(windows)]
    let data = std::path::PathBuf::from(std::env::var_os("APPDATA").unwrap());
    #[cfg(not(any(target_os = "macos", windows)))]
    let data = std::env::home_dir().unwrap().join(".local/share");
    let registry = storage::read_json(&data.join("io.github.shengyy.orbitswitch/accounts.json"))
        .unwrap()
        .expect("OrbitSwitch saved accounts required");
    let ids: Vec<_> = registry["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|account| account["provider"] == "codex")
        .filter_map(|account| account["id"].as_str().map(str::to_owned))
        .collect();
    if let Some(plan) = codex_projects::prepare(&location.root, &id, &ids, Some(&id)).unwrap() {
        let [
            (projects_before, assignments_before),
            (projects_after, assignments_after),
        ] = plan.planned_counts(&id).unwrap();
        assert!(projects_after >= projects_before && assignments_after >= assignments_before);
        println!(
            "Codex local recovery plan (read-only): {projects_before} -> {projects_after} projects; {assignments_before} -> {assignments_after} thread assignments"
        );
    } else {
        println!("Codex local-project state needs no recovery (read-only)");
    }
}

#[test]
#[ignore = "verifies installed CLI discovery and runtime PATH resolution under minimal GUI PATH (read-only --version)"]
fn installed_cli_resolves_under_minimal_gui_path() {
    for provider in [Provider::Codex, Provider::Claude] {
        let cli = install::locate(provider)
            .cli
            .unwrap_or_else(|| panic!("{} CLI not found", provider.key()));
        let output = std::process::Command::new(&cli.program)
            .arg("--version")
            .env("PATH", &cli.path_env)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} --version failed under resolved PATH",
            provider.key()
        );
        println!(
            "{} ({}): {}",
            provider.key(),
            cli.program.display(),
            String::from_utf8_lossy(&output.stdout).trim()
        );
    }
    #[cfg(unix)]
    if std::path::Path::new("/opt/homebrew/bin/node").is_file()
        || std::path::Path::new("/usr/local/bin/node").is_file()
    {
        let cli = install::locate(Provider::Codex).cli.unwrap();
        let node = std::process::Command::new("/usr/bin/env")
            .args(["node", "--version"])
            .env("PATH", &cli.path_env)
            .output()
            .unwrap();
        assert!(
            node.status.success(),
            "node should resolve via cli.path_env"
        );
        println!(
            "node via cli.path_env: {}",
            String::from_utf8_lossy(&node.stdout).trim()
        );
    }
}
