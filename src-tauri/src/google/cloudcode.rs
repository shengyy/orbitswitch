//! Cloud Code endpoints Antigravity uses for quota and subscription tier.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::antigravity::install::Install;
use crate::error::{Error, Result};

/// Cloud Code endpoint passed to Antigravity's language server (`--cloud_code_endpoint`).
const BASE: &str = "https://daily-cloudcode-pa.googleapis.com/v1internal";

pub use crate::model::{GroupKey, Plan, PlanKind, QuotaGroup, QuotaWindow};

#[derive(Deserialize)]
struct SummaryResponse {
    #[serde(default)]
    groups: Vec<RawGroup>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGroup {
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    buckets: Vec<RawBucket>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBucket {
    #[serde(default)]
    window: String,
    /// proto3 JSON omits zero values, so a missing fraction means exhausted.
    #[serde(default)]
    remaining_fraction: f64,
    #[serde(default)]
    reset_time: Option<DateTime<Utc>>,
}

/// Cloud Code client carrying the installed Antigravity version.
pub struct Client<'a> {
    http: &'a reqwest::Client,
    user_agent: String,
    base: &'a str,
}

impl<'a> Client<'a> {
    pub fn new(http: &'a reqwest::Client, install: &Install) -> Result<Self> {
        Ok(Self {
            http,
            user_agent: format!("{} (antigravity/{})", super::USER_AGENT, install.version()?),
            base: BASE,
        })
    }

    async fn post<T: for<'de> Deserialize<'de>>(
        &self,
        access_token: &str,
        method: &'static str,
        body: serde_json::Value,
    ) -> Result<T> {
        let response = self
            .http
            .post(format!("{}:{method}", self.base))
            .header(reqwest::header::USER_AGENT, &self.user_agent)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(Error::Http {
                endpoint: method,
                status: status.as_u16(),
            });
        }
        Ok(response.json().await?)
    }

    /// The 5-hour and weekly buckets of every model group.
    pub async fn quota(&self, access_token: &str) -> Result<Vec<QuotaGroup>> {
        let summary: SummaryResponse = self
            .post(
                access_token,
                "retrieveUserQuotaSummary",
                serde_json::json!({}),
            )
            .await?;
        Ok(summary.groups.into_iter().map(parse_group).collect())
    }

    /// The Google AI subscription behind the account.
    pub async fn plan(&self, access_token: &str) -> Result<Plan> {
        let load: LoadResponse = self
            .post(
                access_token,
                "loadCodeAssist",
                serde_json::json!({ "metadata": { "ideType": "ANTIGRAVITY" } }),
            )
            .await?;
        Ok(parse_plan(load))
    }
}

fn parse_group(group: RawGroup) -> QuotaGroup {
    let lower = group.display_name.to_lowercase();
    let key = if lower.contains("gemini") {
        GroupKey::Gemini
    } else if lower.contains("claude") {
        GroupKey::Claude
    } else {
        GroupKey::Other
    };
    let label = match key {
        GroupKey::Gemini => "Gemini".to_owned(),
        GroupKey::Claude => "Claude".to_owned(),
        _ => group.display_name.clone(),
    };
    // "Models within this group: Claude Opus, Claude Sonnet, GPT-OSS"
    let models = group
        .description
        .split_once(':')
        .map(|(_, list)| {
            list.split(',')
                .map(|m| m.trim().to_owned())
                .filter(|m| !m.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let window = |name: &str| {
        group
            .buckets
            .iter()
            .find(|b| b.window == name)
            .map(|b| QuotaWindow {
                remaining: b.remaining_fraction.clamp(0.0, 1.0),
                reset_at: b.reset_time,
                label: name.into(),
                duration_minutes: Some(if name == "5h" { 300 } else { 10080 }),
            })
    };
    QuotaGroup {
        key,
        label,
        models,
        five_hour: window("5h"),
        weekly: window("weekly"),
        extra: group
            .buckets
            .iter()
            .filter(|b| b.window != "5h" && b.window != "weekly")
            .map(|b| QuotaWindow {
                remaining: b.remaining_fraction.clamp(0.0, 1.0),
                reset_at: b.reset_time,
                label: b.window.clone(),
                duration_minutes: None,
            })
            .collect(),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoadResponse {
    #[serde(default)]
    paid_tier: Option<Tier>,
    #[serde(default)]
    current_tier: Option<Tier>,
}

#[derive(Deserialize)]
struct Tier {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
}

fn parse_plan(load: LoadResponse) -> Plan {
    match load.paid_tier.or(load.current_tier) {
        Some(tier) if tier.id.contains("ultra") => Plan {
            kind: PlanKind::Ultra,
            name: tier.name,
        },
        Some(tier) if tier.id.contains("pro") => Plan {
            kind: PlanKind::Pro,
            name: tier.name,
        },
        _ => Plan {
            kind: PlanKind::Free,
            name: "Free".into(),
        },
    }
}

#[cfg(test)]
mod tests {
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
                    let groups = if ua.contains(&format!("antigravity/{version}")) {
                        serde_json::json!([{"displayName":"Gemini Models"}, {"displayName":"Claude and GPT models"}])
                    } else {
                        serde_json::json!([{"displayName":"Claude and GPT models"}])
                    };
                    let body = if method == "retrieveUserQuotaSummary" {
                        serde_json::json!({"groups": groups})
                    } else {
                        assert!(ua.contains(&format!("antigravity/{version}")));
                        serde_json::json!({"paidTier":{"id":"g1-pro-tier","name":"Google AI Pro"}})
                    }
                    .to_string();
                    stream.write_all(format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()
                    ).as_bytes()).await.unwrap();
                }
            });
            let groups = client.quota("synthetic-token").await.unwrap();
            assert_eq!(groups.len(), 2);
            assert_eq!(groups[0].key, GroupKey::Gemini);
            assert_eq!(groups[1].key, GroupKey::Claude);
            assert_eq!(
                client.plan("synthetic-token").await.unwrap().kind,
                PlanKind::Pro
            );
            server.await.unwrap();
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
            serde_json::from_str(r#"{"currentTier":{"id":"free-tier","name":"Antigravity"}}"#)
                .unwrap();
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
}
