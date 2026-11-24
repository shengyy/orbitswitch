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
    buckets: Option<Vec<RawBucket>>,
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
            user_agent: format!(
                "{} (antigravity/hub/{})",
                super::USER_AGENT,
                install.version()?
            ),
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
        Ok(summary.groups.into_iter().filter_map(parse_group).collect())
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

fn parse_group(group: RawGroup) -> Option<QuotaGroup> {
    let buckets = group.buckets?;
    let window = |name: &str| {
        buckets
            .iter()
            .find(|bucket| bucket.window == name)
            .map(|bucket| QuotaWindow {
                remaining: bucket.remaining_fraction.clamp(0.0, 1.0),
                reset_at: bucket.reset_time,
                label: name.into(),
                duration_minutes: Some(if name == "5h" { 300 } else { 10080 }),
            })
    };
    let five_hour = window("5h");
    let weekly = window("weekly");
    let extra: Vec<_> = buckets
        .into_iter()
        .filter(|bucket| {
            !bucket.window.trim().is_empty() && !matches!(bucket.window.as_str(), "5h" | "weekly")
        })
        .map(|bucket| QuotaWindow {
            remaining: bucket.remaining_fraction.clamp(0.0, 1.0),
            reset_at: bucket.reset_time,
            label: bucket.window,
            duration_minutes: None,
        })
        .collect();
    if five_hour.is_none() && weekly.is_none() && extra.is_empty() {
        return None;
    }

    let (key, label) = match group.display_name.trim().to_ascii_lowercase().as_str() {
        "gemini models" => (GroupKey::Gemini, "Gemini".to_owned()),
        "claude and gpt models" => (GroupKey::Claude, "Claude".to_owned()),
        _ => (GroupKey::Other, group.display_name),
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
    Some(QuotaGroup {
        key,
        label,
        models,
        five_hour,
        weekly,
        extra,
    })
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
#[path = "cloudcode_tests.rs"]
mod tests;
