use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn sends_installed_version_and_refreshes_it_after_an_upgrade() {
    let dir = tempfile::tempdir().unwrap();
    let install = Install {
        root: dir.path().into(),
        main_exe: dir.path().join("Antigravity"),
        bin_dir: dir.path().join("bin"),
    };
    let http = reqwest::Client::builder()
        .user_agent(super::super::USER_AGENT)
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    for version in ["2.23.1", "2.24.0"] {
        let package = serde_json::json!({"version": version}).to_string();
        let header = serde_json::json!({"files": {"package.json": {
            "offset": "0", "size": package.len()
        }}})
        .to_string();
        let padded = (header.len() + 3) & !3;
        let size = (8 + padded) as u32;
        let mut archive = Vec::new();
        for word in [4, size, size - 4, header.len() as u32] {
            archive.extend(word.to_le_bytes());
        }
        archive.extend(header.as_bytes());
        archive.resize(8 + size as usize, 0);
        archive.extend(package.as_bytes());
        std::fs::write(dir.path().join("app.asar"), archive).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1internal", listener.local_addr().unwrap());
        let mut client = Client::new(&http, &install).unwrap();
        client.base = &base;
        let server = tokio::spawn(async move {
            for method in ["retrieveUserQuotaSummary", "loadCodeAssist"] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut byte = [0];
                while !request.ends_with(b"\r\n\r\n") {
                    stream.read_exact(&mut byte).await.unwrap();
                    request.push(byte[0]);
                    assert!(request.len() < 8192);
                }
                let request = String::from_utf8(request).unwrap();
                assert!(request.starts_with(&format!("POST /v1internal:{method} ")));
                let length: usize = request
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                let mut body = vec![0; length];
                stream.read_exact(&mut body).await.unwrap();
                let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(
                    body,
                    if method == "retrieveUserQuotaSummary" {
                        serde_json::json!({})
                    } else {
                        serde_json::json!({"metadata":{"ideType":"ANTIGRAVITY"}})
                    }
                );
                let ua = request
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("user-agent")
                            .then(|| value.trim())
                    })
                    .unwrap();
                assert!(ua.starts_with("OrbitSwitch/"));
                // The upstream gate returns HTTP 200 with only Claude if
                // the vendor product/version token is missing or stale.
                let groups = if ua.contains(&format!("antigravity/hub/{version}")) {
                    serde_json::json!([
                        {"displayName":"Gemini Models"},
                        {"displayName":"Claude and GPT models"},
                        {"displayName":"Gemini 4 Argon","buckets":[{
                            "bucketId":"gemini-argon-wtu-weekly","window":"weekly",
                            "remainingFraction":0.75,"resetTime":"2026-11-01T12:00:00Z"
                        }]}
                    ])
                } else {
                    serde_json::json!([{"displayName":"Claude and GPT models"}])
                };
                let body = if method == "retrieveUserQuotaSummary" {
                    serde_json::json!({"groups": groups})
                } else {
                    assert!(ua.contains(&format!("antigravity/hub/{version}")));
                    serde_json::json!({"paidTier":{"id":"g1-pro-tier","name":"Google AI Pro"}})
                }
                .to_string();
                stream.write_all(format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()
                ).as_bytes()).await.unwrap();
            }
        });
        let groups = client.quota("synthetic-token").await.unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].key, GroupKey::Gemini);
        assert_eq!(groups[1].key, GroupKey::Claude);
        assert_eq!(groups[2].key, GroupKey::Argon);
        assert_eq!(
            client.plan("synthetic-token").await.unwrap().kind,
            PlanKind::Pro
        );
        server.await.unwrap();
    }
}

#[test]
fn preserves_shared_argon_weekly_quota_when_summary_reports_separate_pool() {
    // Given a Pro summary with separate Flash and shared Argon pools.
    for fraction in [Some(0.375), None] {
        let mut bucket = serde_json::json!({
            "bucketId":"gemini-argon-wtu-weekly",
            "displayName":"Gemini 4 Argon Weekly Limit",
            "window":"weekly","resetTime":"2026-11-01T12:00:00Z"
        });
        if let Some(remaining) = fraction {
            bucket["remainingFraction"] = serde_json::json!(remaining);
        }
        let summary: SummaryResponse = serde_json::from_value(serde_json::json!({"groups":[
            {"displayName":"Gemini Models","buckets":[
                {"window":"weekly","remainingFraction":0.9}
            ]},
            {"displayName":"Gemini 4 Argon",
             "description":"For paid Pro plans, Argon quota is separate.",
             "buckets":[bucket]}
        ]}))
        .unwrap();

        // When the summary is converted to the frontend snapshot.
        let groups: Vec<_> = summary.groups.into_iter().map(parse_group).collect();

        // Then Argon stays one weekly row, distinct from Gemini, with its reset.
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].key, GroupKey::Gemini);
        let argon = &groups[1];
        assert_eq!(argon.key, GroupKey::Argon);
        assert_eq!(argon.label, "Argon");
        assert!(argon.models.is_empty());
        assert!(argon.five_hour.is_none());
        assert!(argon.extra.is_empty());
        let weekly = argon.weekly.as_ref().unwrap();
        assert!((weekly.remaining - fraction.unwrap_or(0.0)).abs() < f64::EPSILON);
        assert_eq!(weekly.duration_minutes, Some(10080));
        assert_eq!(
            weekly.reset_at,
            Some("2026-11-01T12:00:00Z".parse().unwrap())
        );
    }
}

#[test]
fn parses_quota_summary() {
    let raw = r#"{"groups":[
      {"displayName":"Gemini Models","description":"Models within this group: Gemini Flash, Gemini Pro",
       "buckets":[{"window":"weekly","remainingFraction":0.96,"resetTime":"2026-10-10T09:23:35Z"},
                  {"window":"5h","remainingFraction":0.5,"resetTime":"2026-10-03T14:23:35Z"}]},
      {"displayName":"Claude and GPT models","description":"Models within this group: Claude Opus, Claude Sonnet, GPT-OSS",
       "buckets":[{"window":"5h","resetTime":"2026-10-03T14:23:35Z"}]}]}"#;
    let summary: SummaryResponse = serde_json::from_str(raw).unwrap();
    let groups: Vec<_> = summary.groups.into_iter().map(parse_group).collect();
    assert_eq!(groups[0].key, GroupKey::Gemini);
    assert_eq!(groups[0].five_hour.as_ref().unwrap().remaining, 0.5);
    assert_eq!(groups[0].weekly.as_ref().unwrap().remaining, 0.96);
    assert_eq!(groups[1].key, GroupKey::Claude);
    assert_eq!(
        groups[1].models,
        ["Claude Opus", "Claude Sonnet", "GPT-OSS"]
    );
    // proto3 omitted the zero fraction
    assert_eq!(groups[1].five_hour.as_ref().unwrap().remaining, 0.0);
    assert!(groups[1].weekly.is_none());
}

#[test]
fn parses_plan_tiers() {
    let pro: LoadResponse =
        serde_json::from_str(r#"{"currentTier":{"id":"free-tier"},"paidTier":{"id":"g1-pro-tier","name":"Google AI Pro"}}"#)
            .unwrap();
    assert_eq!(
        parse_plan(pro),
        Plan {
            kind: PlanKind::Pro,
            name: "Google AI Pro".into()
        }
    );
    let free: LoadResponse =
        serde_json::from_str(r#"{"currentTier":{"id":"free-tier","name":"Antigravity"}}"#).unwrap();
    assert_eq!(parse_plan(free).kind, PlanKind::Free);
}

#[test]
fn parses_starter_paid_tier_as_free() {
    let starter = serde_json::from_str(
        r#"{"currentTier":{"id":"free-tier","name":"Antigravity"},
            "paidTier":{"id":"free-tier","name":"Antigravity Starter Quota"}}"#,
    )
    .unwrap();
    assert_eq!(
        parse_plan(starter),
        Plan {
            kind: PlanKind::Free,
            name: "Free".into(),
        }
    );
}
