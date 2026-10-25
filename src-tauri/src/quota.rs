//! Keeping each account's quota and plan current.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::task::JoinSet;

use crate::error::{Error, Result};
use crate::google::cloudcode::{self, Plan, QuotaGroup};
use crate::model::QuotaState;
use crate::state::{Inner, Orbit};

/// Refreshes every account concurrently. A refresh already in flight wins.
pub async fn refresh_all(orbit: &Arc<Orbit>) {
    let Ok(_sync) = orbit.sync_lock.try_lock() else {
        return;
    };
    if !orbit.is_idle() {
        return;
    }
    let ids = {
        let mut state = orbit.state();
        if state.refreshing {
            return;
        }
        state.refreshing = true;
        state
            .registry
            .accounts()
            .iter()
            .map(|a| a.id.clone())
            .collect::<Vec<_>>()
    };
    orbit.emit();
    let mut tasks = JoinSet::new();
    for id in ids {
        let orbit = orbit.clone();
        tasks.spawn(async move { refresh_one(&orbit, &id).await });
    }
    while tasks.join_next().await.is_some() {}
    orbit.update(|state| {
        state.refreshing = false;
        state.refreshed_at = Some(Utc::now());
    });
}

/// Refreshes when the last full refresh is older than `max_age`.
pub async fn refresh_if_stale(orbit: &Arc<Orbit>, max_age: Duration) {
    let stale = orbit
        .state()
        .refreshed_at
        .is_none_or(|at| (Utc::now() - at).to_std().unwrap_or_default() > max_age);
    if stale {
        crate::scheduler::tick(orbit).await;
    }
}

struct Refresh {
    groups: Result<Vec<QuotaGroup>>,
    plan: Result<Plan>,
}

pub async fn refresh_one(orbit: &Arc<Orbit>, account_id: &str) {
    let provider = orbit.state().registry.get(account_id).map(|a| a.provider);
    if let Some(provider) = provider.filter(|p| *p != crate::provider::Provider::Antigravity) {
        crate::native::refresh(orbit, provider, account_id).await;
        return;
    }
    let result = async {
        let token = orbit.access_token(account_id).await?;
        let cloudcode = cloudcode::Client::new(&orbit.http, &orbit.install()?)?;
        let (groups, plan) = tokio::join!(cloudcode.quota(&token), cloudcode.plan(&token));
        Ok(Refresh { groups, plan })
    }
    .await;
    orbit.update(|state| apply_refresh(state, account_id, result));
}

fn apply_refresh(state: &mut Inner, account_id: &str, result: Result<Refresh>) {
    // A quota failure must not discard a successful subscription lookup.
    let groups = match result {
        Ok(Refresh { groups, plan }) => {
            match plan {
                Ok(plan) => {
                    if let Err(err) = state.registry.set_plan(account_id, plan) {
                        log::warn!("could not save plan: {err}");
                    }
                }
                Err(err) => log::warn!("plan lookup failed: {err}"),
            }
            groups
        }
        Err(err) => Err(err),
    };
    let quota = match groups {
        Ok(groups) => QuotaState::Ready {
            groups,
            fetched_at: Utc::now(),
            stale: false,
        },
        Err(Error::TokenRevoked) => QuotaState::Reauth,
        Err(err) => {
            log::warn!("quota refresh failed: {err}");
            QuotaState::Failed {
                code: err.code().into(),
                message: err.to_string(),
            }
        }
    };
    // Keep the account's last good numbers visible through a transient error.
    let keep_previous = matches!(quota, QuotaState::Failed { .. })
        && matches!(state.quotas.get(account_id), Some(QuotaState::Ready { .. }));
    if keep_previous && let Some(QuotaState::Ready { stale, .. }) = state.quotas.get_mut(account_id)
    {
        *stale = true;
    }
    if !keep_previous && state.registry.get(account_id).is_some() {
        state.quotas.insert(account_id.to_owned(), quota);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::google::cloudcode::PlanKind;
    use crate::model::Operation;
    use crate::registry::{Account, Registry};

    fn state() -> (tempfile::TempDir, Inner) {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = Registry::load(dir.path()).unwrap();
        registry
            .upsert(Account {
                provider: crate::provider::Provider::Antigravity,
                desktop_ready: true,
                id: "sample".into(),
                email: "sample@example.com".into(),
                name: None,
                picture: None,
                plan: Some(Plan {
                    kind: PlanKind::Pro,
                    name: "Google AI Pro".into(),
                }),
                added_at: Utc::now(),
            })
            .unwrap();
        (
            dir,
            Inner {
                registry,
                quotas: Default::default(),
                active_id: None,
                native_active: Default::default(),
                native_aligned: Default::default(),
                native_clients: Default::default(),
                operation: Operation::Idle,
                refreshing: false,
                refreshed_at: None,
                install: None,
                running: false,
                update: Default::default(),
            },
        )
    }

    fn free_plan() -> Result<Plan> {
        Ok(Plan {
            kind: PlanKind::Free,
            name: "Free".into(),
        })
    }

    #[test]
    fn quota_denied_still_saves_subscription_downgrade() {
        for has_previous_quota in [false, true] {
            let (dir, mut state) = state();
            let previous_fetch = Utc::now() - chrono::Duration::minutes(5);
            if has_previous_quota {
                state.quotas.insert(
                    "sample".into(),
                    QuotaState::Ready {
                        groups: Vec::new(),
                        fetched_at: previous_fetch,
                        stale: false,
                    },
                );
            }
            apply_refresh(
                &mut state,
                "sample",
                Ok(Refresh {
                    groups: Err(Error::Http {
                        endpoint: "retrieveUserQuotaSummary",
                        status: 403,
                    }),
                    plan: free_plan(),
                }),
            );
            assert_eq!(state.registry.get("sample").unwrap().plan, free_plan().ok());
            let saved = Registry::load(dir.path()).unwrap();
            assert_eq!(saved.get("sample").unwrap().plan, free_plan().ok());
            match state.quotas.get("sample").unwrap() {
                QuotaState::Ready { fetched_at, .. } if has_previous_quota => {
                    assert_eq!(*fetched_at, previous_fetch);
                }
                QuotaState::Failed { .. } if !has_previous_quota => {}
                other => panic!("unexpected quota state: {other:?}"),
            }
        }
    }

    #[test]
    fn plan_failure_keeps_saved_plan_and_updates_quota() {
        let (_dir, mut state) = state();
        let previous_plan = state.registry.get("sample").unwrap().plan.clone();
        apply_refresh(
            &mut state,
            "sample",
            Ok(Refresh {
                groups: Ok(Vec::new()),
                plan: Err(Error::Http {
                    endpoint: "loadCodeAssist",
                    status: 503,
                }),
            }),
        );
        assert_eq!(state.registry.get("sample").unwrap().plan, previous_plan);
        assert!(matches!(state.quotas["sample"], QuotaState::Ready { .. }));
    }

    #[test]
    fn revoked_token_replaces_cached_quota_with_reauth() {
        let (_dir, mut state) = state();
        state.quotas.insert(
            "sample".into(),
            QuotaState::Ready {
                groups: Vec::new(),
                fetched_at: Utc::now(),
                stale: false,
            },
        );
        apply_refresh(&mut state, "sample", Err(Error::TokenRevoked));
        assert!(matches!(state.quotas["sample"], QuotaState::Reauth));
    }

    #[test]
    fn completed_refresh_does_not_restore_removed_account() {
        let (dir, mut state) = state();
        state.registry.remove("sample").unwrap();
        apply_refresh(
            &mut state,
            "sample",
            Ok(Refresh {
                groups: Ok(Vec::new()),
                plan: free_plan(),
            }),
        );
        assert!(!state.quotas.contains_key("sample"));
        assert!(Registry::load(dir.path()).unwrap().accounts().is_empty());
    }
}
