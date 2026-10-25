//! Opt-in read-only probes. Never refresh tokens, launch/stop clients or write credentials.
use super::{api, desktop, live};
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
