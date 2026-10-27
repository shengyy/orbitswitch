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
    let key = if lower.contains("argon") {
        GroupKey::Argon
    } else if lower.contains("gemini") {
        GroupKey::Gemini
    } else if lower.contains("claude") {
        GroupKey::Claude
    } else {
        GroupKey::Other
    };
    let label = match key {
        GroupKey::Gemini => "Gemini".to_owned(),
        GroupKey::Argon => "Argon".to_owned(),
        GroupKey::Claude => "Claude".to_owned(),
        GroupKey::Other
        | GroupKey::Codex
        | GroupKey::Review
        | GroupKey::Sonnet
        | GroupKey::Opus => group.display_name.clone(),
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
#[path = "cloudcode_tests.rs"]
mod tests;
