//! Opt-in read-only probes. Never refresh tokens, launch/stop clients or write credentials.
use super::{api, codex_projects, install, live, storage};
use crate::provider::Provider;

#[tokio::test]
#[ignore = "reads the signed-in official clients and their usage endpoints"]
async fn current_subscription_contracts() {
    let http = reqwest::Client::builder()
        .user_agent(crate::google::USER_AGENT)
        .timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    for provider in [Provider::Codex, Provider::Claude] {
        let location = live::locate(provider).unwrap();
        let credential = location
            .read()
            .unwrap()
            .expect("sign in with the official CLI first");
        let (mut session, _) = super::observe_sign_in(&http, provider, &location, credential)
            .await
            .expect("identify the current sign-in");
        let id = session.id(provider).unwrap();
        let groups = super::subscription_quota(&http, provider, &id, true, &mut session)
            .await
            .expect("read the current subscription")
            .groups
            .expect("read current usage");
        assert!(
            !groups.is_empty(),
            "the account should expose at least one rate-limit window"
        );
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
    let location = live::locate(Provider::Claude).unwrap();
    let credential = location.read().unwrap().expect("CLI login required");
    let (session, oauth_verified) =
        super::observe_sign_in(&http, Provider::Claude, &location, credential)
            .await
            .expect("observe the live CLI account");
    assert!(
        session.desktop.is_some(),
        "Claude Desktop is not signed into the same account as the CLI"
    );
    println!(
        "Claude Desktop and CLI identity match (read-only). Fresh OAuth profile: {oauth_verified}"
    );
}

#[tokio::test]
#[ignore = "reads Claude Desktop subscription limits for the signed-in saved account"]
async fn current_claude_desktop_usage() {
    let provider = Provider::Claude;
    let location = live::locate(provider).unwrap();
    let metadata = storage::read_json(location.metadata.as_ref().unwrap())
        .unwrap()
        .unwrap();
    let subject = metadata["oauthAccount"]["accountUuid"].as_str().unwrap();
    let org = metadata["oauthAccount"]["organizationUuid"]
        .as_str()
        .unwrap();
    let id = format!("claude:{subject}:{org}");
    let mut saved = super::load(&id).expect("saved account");
    let cli = location.read().unwrap().expect("CLI login required");
    let same_access = api::access(provider, &cli)
        .is_ok_and(|token| api::access(provider, &saved.credential).ok() == Some(token));
    let has_refresh = cli["claudeAiOauth"]["refreshToken"]
        .as_str()
        .is_some_and(|token| !token.is_empty());
    let http = reqwest::Client::builder()
        .user_agent(crate::google::USER_AGENT)
        .timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let expired = api::expires(provider, &saved.credential)
        .is_some_and(|at| at <= chrono::Utc::now().timestamp());
    if !expired {
        // Exercise the fallback on hosts with fresh tokens using only this in-memory copy.
        saved.credential["claudeAiOauth"]["expiresAt"] = 0.into();
    }
    let groups = super::subscription_quota(&http, provider, &id, true, &mut saved)
        .await
        .expect("subscription quota")
        .groups
        .expect("desktop usage");
    let five_hour = groups.iter().any(|group| group.five_hour.is_some());
    let weekly = groups.iter().any(|group| group.weekly.is_some());
    assert!(
        five_hour || weekly,
        "expected a reported subscription usage window"
    );
    println!(
        "Saved access token already expired: {expired}. CLI access matches saved: {same_access}. CLI refresh credential present: {has_refresh}. Desktop windows: five-hour={five_hour}, weekly={weekly}"
    );
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
