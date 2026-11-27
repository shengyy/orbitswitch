//! Checks against the real Antigravity installation on this machine.
//!
//! Run with `cargo test system_tests -- --ignored --nocapture` on a machine
//! where Antigravity is installed and signed in. The probes verify local
//! identity and use the access token for read-only Google queries.

use crate::antigravity::install::Install;
use crate::antigravity::{live, oauth_client, process};
use crate::google::{USER_AGENT, cloudcode};

#[cfg(target_os = "macos")]
#[test]
#[ignore = "needs the signed-in Antigravity account saved in OrbitSwitch"]
fn matches_saved_antigravity_sign_in_without_userinfo() {
    use crate::accounts::match_saved_refresh_token;
    use crate::provider::Provider;
    use crate::registry::Registry;
    use crate::vault;

    let dir = std::env::home_dir()
        .unwrap()
        .join("Library/Application Support/io.github.shengyy.orbitswitch");
    let registry = Registry::read(&dir).expect("OrbitSwitch account registry");
    let ids = registry
        .accounts()
        .iter()
        .filter(|account| account.provider == Provider::Antigravity)
        .map(|account| account.id.clone())
        .collect();
    let bundle = live::read().unwrap().expect("Antigravity is signed in");
    assert!(!bundle.token.refresh_token.is_empty());
    let id = match_saved_refresh_token(ids, &bundle.token.refresh_token, vault::get)
        .expect("read saved grants")
        .expect("save the signed-in Antigravity account in OrbitSwitch first");
    if let Some(email) = bundle.extra.get("id_token_email").and_then(|v| v.as_str()) {
        assert!(
            registry.get(&id).unwrap().email.eq_ignore_ascii_case(email),
            "the matched account should agree with the live credential's email"
        );
    }
    println!("Saved Antigravity account verified without Google UserInfo");
}

#[tokio::test]
#[ignore = "needs a local Antigravity installation"]
async fn reads_installation_sign_in_and_quota() {
    let install = Install::locate().expect("Antigravity installation");
    println!(
        "Antigravity version: {}",
        install.version().expect("installed version")
    );
    println!(
        "Antigravity processes: {}",
        process::scan(Some(&install)).len()
    );

    let http = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .unwrap();
    oauth_client::discover(&http, &install)
        .await
        .expect("OAuth client");

    let bundle = live::read().unwrap().expect("Antigravity is signed in");
    bundle
        .profile(&http)
        .await
        .expect("Antigravity account identity");
    println!(
        "Antigravity account identity verified, usable ID token claims: {}",
        bundle.claims().is_some()
    );

    let cloudcode = cloudcode::Client::new(&http, &install).expect("Cloud Code identity");
    let access_token = &bundle.token.access_token;
    let groups = cloudcode.quota(access_token).await.expect("quota");
    for group in &groups {
        let pct =
            |w: &Option<cloudcode::QuotaWindow>| w.as_ref().map(|w| (w.remaining * 100.0).round());
        println!(
            "quota {:?}: 5h={:?}% weekly={:?}%",
            group.key,
            pct(&group.five_hour),
            pct(&group.weekly)
        );
    }
    assert!(!groups.is_empty());
    assert!(
        groups
            .iter()
            .any(|group| group.key == cloudcode::GroupKey::Gemini),
        "quota summary omitted Gemini; verify the installed product identity"
    );
    println!(
        "plan: {:?}",
        cloudcode.plan(access_token).await.expect("plan").kind
    );
}
