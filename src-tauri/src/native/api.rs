//! Subscription and quota contracts used by the installed official clients.
use crate::{
    error::{Error, Result},
    google::cloudcode::{GroupKey, Plan, PlanKind, QuotaGroup, QuotaWindow},
    provider::Provider,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use serde_json::Value;
const CODEX_CLIENT: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
// Public OAuth client identifier, not a secret.
const CLAUDE_CLIENT: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
pub fn claims(jwt: &str) -> Result<Value> {
    let payload = jwt.split('.').nth(1).ok_or(Error::VerifyFailed)?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .map_err(|_| Error::VerifyFailed)?;
    Ok(serde_json::from_slice(&bytes)?)
}
pub fn access(provider: Provider, credential: &Value) -> Result<&str> {
    let value = match provider {
        Provider::Codex => &credential["tokens"]["access_token"],
        _ => &credential["claudeAiOauth"]["accessToken"],
    };
    value
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or(Error::SubscriptionLoginRequired)
}
pub fn expires(provider: Provider, credential: &Value) -> Option<i64> {
    if provider == Provider::Codex {
        claims(access(provider, credential).ok()?).ok()?["exp"].as_i64()
    } else {
        credential["claudeAiOauth"]["expiresAt"]
            .as_i64()
            .map(|n| n / 1000)
    }
}
pub fn identity(
    provider: Provider,
    credential: &Value,
    profile: &Value,
) -> Result<(String, String, String, Option<String>)> {
    if provider == Provider::Codex {
        let token = claims(
            credential["tokens"]["id_token"]
                .as_str()
                .ok_or(Error::SubscriptionLoginRequired)?,
        )?;
        let subject = token["https://api.openai.com/auth"]["chatgpt_user_id"]
            .as_str()
            .or(token["sub"].as_str())
            .ok_or(Error::VerifyFailed)?;
        let org = credential["tokens"]["account_id"]
            .as_str()
            .ok_or(Error::VerifyFailed)?;
        Ok((
            subject.into(),
            org.into(),
            token["email"].as_str().unwrap_or_default().into(),
            token["name"].as_str().map(Into::into),
        ))
    } else {
        let a = &profile["account"];
        let org = profile["organization"]["uuid"]
            .as_str()
            .ok_or(Error::VerifyFailed)?;
        Ok((
            a["uuid"].as_str().ok_or(Error::VerifyFailed)?.into(),
            org.into(),
            a["email"].as_str().unwrap_or_default().into(),
            a["display_name"]
                .as_str()
                .or(a["full_name"].as_str())
                .map(Into::into),
        ))
    }
}
async fn json(response: reqwest::Response, endpoint: &'static str) -> Result<Value> {
    let status = response.status();
    if status.as_u16() == 401 {
        return Err(Error::TokenRevoked);
    }
    if !status.is_success() {
        return Err(Error::Http {
            endpoint,
            status: status.as_u16(),
        });
    }
    Ok(response.json().await?)
}
pub async fn profile(
    http: &reqwest::Client,
    provider: Provider,
    credential: &Value,
) -> Result<Value> {
    if provider == Provider::Codex {
        return claims(
            credential["tokens"]["id_token"]
                .as_str()
                .ok_or(Error::SubscriptionLoginRequired)?,
        );
    }
    json(
        http.get("https://api.anthropic.com/api/oauth/profile")
            .bearer_auth(access(provider, credential)?)
            .header("anthropic-beta", "oauth-2025-04-20")
            .send()
            .await?,
        "claude/profile",
    )
    .await
}
pub async fn usage(
    http: &reqwest::Client,
    provider: Provider,
    credential: &Value,
) -> Result<Value> {
    let request = if provider == Provider::Codex {
        http.get("https://chatgpt.com/backend-api/wham/usage")
            .header(
                "ChatGPT-Account-Id",
                credential["tokens"]["account_id"]
                    .as_str()
                    .ok_or(Error::VerifyFailed)?,
            )
    } else {
        http.get("https://api.anthropic.com/api/oauth/usage")
            .header("anthropic-beta", "oauth-2025-04-20")
    };
    json(
        request
            .bearer_auth(access(provider, credential)?)
            .send()
            .await?,
        "subscription/usage",
    )
    .await
}
pub fn verify_usage_identity(provider: Provider, credential: &Value, usage: &Value) -> Result<()> {
    if provider == Provider::Codex
        && usage["account_id"].as_str() != credential["tokens"]["account_id"].as_str()
    {
        return Err(Error::VerifyFailed);
    }
    if provider == Provider::Codex {
        let profile = claims(
            credential["tokens"]["id_token"]
                .as_str()
                .ok_or(Error::VerifyFailed)?,
        )?;
        if let Some(user) = usage["user_id"].as_str()
            && profile["https://api.openai.com/auth"]["chatgpt_user_id"].as_str() != Some(user)
        {
            return Err(Error::VerifyFailed);
        }
    }
    Ok(())
}
pub async fn renew(
    http: &reqwest::Client,
    provider: Provider,
    credential: &mut Value,
) -> Result<()> {
    let (url, client, refresh) = if provider == Provider::Codex {
        (
            "https://auth.openai.com/oauth/token",
            CODEX_CLIENT,
            credential["tokens"]["refresh_token"].as_str(),
        )
    } else {
        (
            "https://platform.claude.com/v1/oauth/token",
            CLAUDE_CLIENT,
            credential["claudeAiOauth"]["refreshToken"].as_str(),
        )
    };
    let response=http.post(url).json(&serde_json::json!({"grant_type":"refresh_token","client_id":client,"refresh_token":refresh.ok_or(Error::TokenRevoked)?})).send().await?;
    let status = response.status();
    let data: Value = response.json().await?;
    if !status.is_success() {
        let code = data["error"]
            .as_str()
            .or(data["error"]["type"].as_str())
            .or(data["error"]["code"].as_str());
        if matches!(
            code,
            Some(
                "invalid_grant"
                    | "refresh_token_reused"
                    | "refresh_token_expired"
                    | "refresh_token_invalidated"
            )
        ) {
            return Err(Error::TokenRevoked);
        }
        return Err(Error::Http {
            endpoint: "oauth/refresh",
            status: status.as_u16(),
        });
    }
    let access = data["access_token"].as_str().ok_or(Error::VerifyFailed)?;
    if provider == Provider::Codex {
        credential["tokens"]["access_token"] = access.into();
        for key in ["refresh_token", "id_token"] {
            if let Some(value) = data.get(key) {
                credential["tokens"][key] = value.clone();
            }
        }
        credential["last_refresh"] = Utc::now().to_rfc3339().into();
    } else {
        let oauth = &mut credential["claudeAiOauth"];
        oauth["accessToken"] = access.into();
        if let Some(value) = data.get("refresh_token") {
            oauth["refreshToken"] = value.clone();
        }
        oauth["expiresAt"] =
            ((Utc::now().timestamp() + data["expires_in"].as_i64().unwrap_or(3600)) * 1000).into();
    }
    Ok(())
}
pub fn plan(provider: Provider, profile: &Value, usage: Option<&Value>) -> Option<Plan> {
    let value = if provider == Provider::Codex {
        usage
            .and_then(|u| u["plan_type"].as_str())
            .or(profile["https://api.openai.com/auth"]["chatgpt_plan_type"].as_str())
    } else {
        profile["organization"]["organization_type"].as_str()
    };
    value.map(|s| {
        let name = s.trim_start_matches("claude_");
        let kind = match name {
            "free" => PlanKind::Free,
            "plus" => PlanKind::Plus,
            "pro" => PlanKind::Pro,
            "max" => PlanKind::Max,
            "team" => PlanKind::Team,
            "business" => PlanKind::Business,
            "enterprise" => PlanKind::Enterprise,
            _ => PlanKind::Unknown,
        };
        Plan {
            kind,
            name: name.replace('_', " "),
        }
    })
}
fn time(value: &Value) -> Option<DateTime<Utc>> {
    value
        .as_i64()
        .and_then(|n| DateTime::from_timestamp(n, 0))
        .or_else(|| {
            value
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|t| t.with_timezone(&Utc))
        })
}
fn window(
    value: &Value,
    percent_key: &str,
    label: &str,
    duration: Option<u64>,
) -> Option<QuotaWindow> {
    let used = value[percent_key].as_f64().filter(|n| n.is_finite())?;
    Some(QuotaWindow {
        remaining: (1.0 - used / 100.0).clamp(0.0, 1.0),
        reset_at: time(&value["resets_at"]).or_else(|| time(&value["reset_at"])),
        label: label.into(),
        duration_minutes: duration,
    })
}
fn group(key: GroupKey, label: &str, windows: Vec<QuotaWindow>) -> QuotaGroup {
    let mut group = QuotaGroup {
        key,
        label: label.into(),
        models: vec![],
        five_hour: None,
        weekly: None,
        extra: vec![],
    };
    for w in windows {
        match w.duration_minutes {
            Some(300) if group.five_hour.is_none() => group.five_hour = Some(w),
            Some(10080) if group.weekly.is_none() => group.weekly = Some(w),
            _ => group.extra.push(w),
        }
    }
    group
}
pub fn groups(provider: Provider, usage: &Value) -> Vec<QuotaGroup> {
    if provider == Provider::Claude {
        let mut result = vec![group(
            GroupKey::Claude,
            "Claude",
            [("five_hour", "5h", 300), ("seven_day", "weekly", 10080)]
                .into_iter()
                .filter_map(|(k, l, d)| window(&usage[k], "utilization", l, Some(d)))
                .collect(),
        )];
        let scoped = usage["limits"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|l| l["kind"] == "weekly_scoped" && l["is_active"].as_bool() != Some(false))
            .filter_map(|l| {
                let name = l["scope"]["model"]["display_name"].as_str()?;
                Some(group(
                    GroupKey::Other,
                    name,
                    vec![window(l, "percent", "weekly", Some(10080))?],
                ))
            })
            .collect::<Vec<_>>();
        if scoped.is_empty() {
            for (field, key, label) in [
                ("seven_day_sonnet", GroupKey::Sonnet, "Sonnet"),
                ("seven_day_opus", GroupKey::Opus, "Opus"),
            ] {
                if let Some(w) = window(&usage[field], "utilization", "weekly", Some(10080)) {
                    result.push(group(key, label, vec![w]));
                }
            }
        } else {
            result.extend(scoped);
        }
        return result
            .into_iter()
            .filter(|g| g.five_hour.is_some() || g.weekly.is_some() || !g.extra.is_empty())
            .collect();
    }
    let parse = |value: &Value, key, label: &str| {
        let windows = ["primary_window", "secondary_window"]
            .into_iter()
            .filter_map(|field| {
                let w = &value[field];
                let minutes = w["limit_window_seconds"].as_u64().map(|n| n / 60);
                let label = match minutes {
                    Some(300) => "5h".into(),
                    Some(10080) => "weekly".into(),
                    Some(n) => format!("{}m", n),
                    None => "Limit".into(),
                };
                window(w, "used_percent", &label, minutes)
            })
            .collect();
        group(key, label, windows)
    };
    let mut result = vec![
        parse(&usage["rate_limit"], GroupKey::Codex, "Codex"),
        parse(
            &usage["code_review_rate_limit"],
            GroupKey::Review,
            "Code review",
        ),
    ];
    for bucket in usage["additional_rate_limits"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let name = bucket["limit_name"]
            .as_str()
            .or(bucket["limit_id"].as_str())
            .unwrap_or("Additional limit");
        result.push(parse(&bucket["rate_limit"], GroupKey::Other, name));
    }
    result
        .into_iter()
        .filter(|g| g.five_hour.is_some() || g.weekly.is_some() || !g.extra.is_empty())
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_is_not_exhausted() {
        assert!(
            groups(
                Provider::Codex,
                &serde_json::json!({"rate_limit":{"primary_window":{"used_percent":null}}})
            )
            .is_empty()
        );
    }
    #[test]
    fn codex_arbitrary_window_and_remaining() {
        let g = groups(
            Provider::Codex,
            &serde_json::json!({"rate_limit":{"primary_window":{"used_percent":25,"limit_window_seconds":900,"reset_at":1800000000}}}),
        );
        assert_eq!(g[0].extra[0].remaining, 0.75);
        assert_eq!(g[0].extra[0].duration_minutes, Some(15));
    }
    #[test]
    fn claude_scoped_limits_do_not_duplicate_legacy() {
        let g = groups(
            Provider::Claude,
            &serde_json::json!({"seven_day_opus":{"utilization":50},"limits":[{"kind":"weekly_scoped","scope":{"model":{"display_name":"Opus"}},"percent":30,"is_active":true}]}),
        );
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].weekly.as_ref().unwrap().remaining, 0.7);
    }
}
