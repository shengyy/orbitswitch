//! Read-only identity and quota sources for a saved Claude subscription.
use super::{Session, api, desktop, live, load, storage};
use crate::error::{Error, Result};
use crate::google::cloudcode::QuotaGroup;
use crate::provider::Provider;
use serde_json::Value;

pub(super) fn saved_cli_profile(location: &live::Location, live: &Value) -> Result<Option<Value>> {
    let Some(path) = &location.metadata else {
        return Ok(None);
    };
    let Some(metadata) = storage::read_json(path)? else {
        return Ok(None);
    };
    match_saved_profile(&metadata["oauthAccount"], live, load)
}

fn match_saved_profile(
    metadata: &Value,
    live: &Value,
    load_session: impl FnOnce(&str) -> Result<Session>,
) -> Result<Option<Value>> {
    let (Some(subject), Some(org)) = (
        metadata["accountUuid"].as_str().filter(|s| !s.is_empty()),
        metadata["organizationUuid"]
            .as_str()
            .filter(|s| !s.is_empty()),
    ) else {
        return Ok(None);
    };
    let id = format!("claude:{subject}:{org}");
    let saved = match load_session(&id) {
        Ok(saved) => saved,
        Err(Error::AccountNotFound) => return Ok(None),
        Err(error) => return Err(error),
    };
    let refresh = live["claudeAiOauth"]["refreshToken"]
        .as_str()
        .filter(|s| !s.is_empty());
    let matches = match refresh {
        Some(token) => saved.credential["claudeAiOauth"]["refreshToken"].as_str() == Some(token),
        None => api::access(Provider::Claude, live).is_ok_and(|token| {
            api::access(Provider::Claude, &saved.credential).ok() == Some(token)
        }),
    };
    Ok((matches && saved.id(Provider::Claude)? == id).then_some(saved.profile))
}

pub(super) async fn desktop_quota(
    http: &reqwest::Client,
    saved: &Session,
) -> Result<Option<Vec<QuotaGroup>>> {
    let (subject, org, _, _) = saved.identity(Provider::Claude)?;
    let Some(desktop) = desktop::capture()? else {
        return Ok(None);
    };
    match desktop.usage(http, &subject, &org).await {
        Ok(usage) => {
            let groups = api::groups(Provider::Claude, &usage);
            if groups.is_empty()
                && ![
                    "five_hour",
                    "seven_day",
                    "limits",
                    "seven_day_sonnet",
                    "seven_day_opus",
                ]
                .iter()
                .any(|key| usage.get(key).is_some())
            {
                return Err(Error::Invalid(
                    "unrecognized Claude Desktop usage response".into(),
                ));
            }
            Ok(Some(groups))
        }
        Err(Error::DesktopSignInRequired) => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn saved() -> Session {
        Session {
            credential: json!({"claudeAiOauth":{"refreshToken":"grant-a","accessToken":"old-access"}}),
            profile: json!({"account":{"uuid":"user-a"},"organization":{"uuid":"org-a"}}),
            desktop: None,
        }
    }

    #[test]
    fn recognizes_only_the_saved_grant_for_the_same_account_and_workspace() {
        let metadata = json!({"accountUuid":"user-a","organizationUuid":"org-a"});
        let live = json!({"claudeAiOauth":{"refreshToken":"grant-a","accessToken":"new-access"}});
        let matched = match_saved_profile(&metadata, &live, |id| {
            assert_eq!(id, "claude:user-a:org-a");
            Ok(saved())
        })
        .unwrap();
        assert_eq!(matched, Some(saved().profile));
        let access_only = json!({"claudeAiOauth":{"accessToken":"old-access"}});
        assert_eq!(
            match_saved_profile(&metadata, &access_only, |_| Ok(saved())).unwrap(),
            Some(saved().profile)
        );
        for other in [
            json!({"accountUuid":"user-a","organizationUuid":"org-b"}),
            json!({"accountUuid":"user-b","organizationUuid":"org-a"}),
            json!({"accountUuid":"user-a"}),
        ] {
            assert!(
                match_saved_profile(&other, &live, |_| Ok(saved()))
                    .unwrap()
                    .is_none()
            );
        }
        for other in [
            json!({"claudeAiOauth":{"refreshToken":"other","accessToken":"old-access"}}),
            json!({"claudeAiOauth":{"refreshToken":"","accessToken":""}}),
            json!({"claudeAiOauth":{"accessToken":"unknown-access"}}),
        ] {
            assert!(
                match_saved_profile(&metadata, &other, |_| Ok(saved()))
                    .unwrap()
                    .is_none()
            );
        }
        assert!(
            match_saved_profile(&metadata, &live, |_| Err(Error::AccountNotFound))
                .unwrap()
                .is_none()
        );
    }
}
