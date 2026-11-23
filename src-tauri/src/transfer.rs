//! Export and import account bundles, including platform credentials and metadata.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::native::storage;
use crate::provider::Provider;
use crate::registry::Account;
use crate::state::Orbit;
use crate::vault;

const BUNDLE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedAccount {
    pub account: Account,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    pub accounts: Vec<ExportedAccount>,
}

#[cfg(target_os = "macos")]
async fn pick_save_path(default_name: &str) -> Result<Option<std::path::PathBuf>> {
    tokio::task::spawn_blocking({
        let default_name = default_name.to_owned();
        move || {
            let script = format!(
                r#"tell application "System Events"
    activate
    set filePath to choose file name with prompt "Export Accounts" default name "{default_name}"
end tell
POSIX path of filePath"#
            );
            let output = std::process::Command::new("/usr/bin/osascript")
                .args(["-e", &script])
                .output()?;
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                if path_str.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(std::path::PathBuf::from(path_str)))
                }
            } else {
                Ok(None)
            }
        }
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))?
}

#[cfg(not(target_os = "macos"))]
async fn pick_save_path(default_name: &str) -> Result<Option<std::path::PathBuf>> {
    let file_handle = rfd::AsyncFileDialog::new()
        .set_title("Export Accounts")
        .set_file_name(default_name)
        .add_filter("JSON", &["json"])
        .save_file()
        .await;
    Ok(file_handle.map(|h| h.path().to_path_buf()))
}

#[cfg(target_os = "macos")]
async fn pick_open_path() -> Result<Option<std::path::PathBuf>> {
    tokio::task::spawn_blocking(|| {
        let script = r#"tell application "System Events"
    activate
    set filePath to choose file of type {"public.json", "json"} with prompt "Import Accounts"
end tell
POSIX path of filePath"#;
        let output = std::process::Command::new("/usr/bin/osascript")
            .args(["-e", script])
            .output()?;
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if path_str.is_empty() {
                Ok(None)
            } else {
                Ok(Some(std::path::PathBuf::from(path_str)))
            }
        } else {
            Ok(None)
        }
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))?
}

#[cfg(not(target_os = "macos"))]
async fn pick_open_path() -> Result<Option<std::path::PathBuf>> {
    let file_handle = rfd::AsyncFileDialog::new()
        .set_title("Import Accounts")
        .add_filter("JSON", &["json"])
        .pick_file()
        .await;
    Ok(file_handle.map(|h| h.path().to_path_buf()))
}

/// Prompt user to choose a save destination and export all registered accounts.
pub async fn export_all(orbit: &Arc<Orbit>) -> Result<Option<usize>> {
    let accounts = orbit.state().registry.accounts().to_vec();
    if accounts.is_empty() {
        return Ok(Some(0));
    }

    let default_name = format!(
        "orbitswitch-accounts-{}.json",
        Utc::now().format("%Y%m%d-%H%M%S")
    );

    let path = pick_save_path(&default_name).await?;
    let Some(path) = path else {
        #[cfg(target_os = "macos")]
        crate::shell::show(&orbit.app);
        return Ok(None);
    };

    let mut exported = Vec::with_capacity(accounts.len());
    for account in accounts {
        let token = match account.provider {
            Provider::Antigravity => vault::get(&account.id).ok().flatten(),
            Provider::Codex | Provider::Claude => storage::get(&account.id).ok().flatten(),
        };
        exported.push(ExportedAccount { account, token });
    }

    let bundle = ExportBundle {
        version: BUNDLE_VERSION,
        exported_at: Utc::now(),
        accounts: exported,
    };

    let json_bytes = serde_json::to_vec_pretty(&bundle)?;
    std::fs::write(&path, &json_bytes)?;

    #[cfg(target_os = "macos")]
    crate::shell::show(&orbit.app);

    Ok(Some(bundle.accounts.len()))
}

/// Prompt user to select an export file and import accounts into the registry and vaults.
pub async fn import_from_file(orbit: &Arc<Orbit>) -> Result<Option<usize>> {
    let path = pick_open_path().await?;
    let Some(path) = path else {
        #[cfg(target_os = "macos")]
        crate::shell::show(&orbit.app);
        return Ok(None);
    };

    let bytes = std::fs::read(&path)?;
    let bundle: ExportBundle = serde_json::from_slice(&bytes)
        .map_err(|err| Error::Invalid(format!("invalid account export format: {err}")))?;

    if bundle.version == 0 || bundle.version > BUNDLE_VERSION {
        #[cfg(target_os = "macos")]
        crate::shell::show(&orbit.app);
        return Err(Error::Invalid(format!(
            "unsupported export bundle version: {}",
            bundle.version
        )));
    }

    let count = bundle.accounts.len();
    if count == 0 {
        #[cfg(target_os = "macos")]
        crate::shell::show(&orbit.app);
        return Ok(Some(0));
    }

    for item in bundle.accounts {
        if item.account.id.trim().is_empty() {
            continue;
        }

        // Restore secrets to vault or session store if provided
        if let Some(token) = item.token {
            match item.account.provider {
                Provider::Antigravity => {
                    vault::set(&item.account.id, &token)?;
                }
                Provider::Codex | Provider::Claude => {
                    storage::set(&item.account.id, &token)?;
                }
            }
        }

        orbit.update(|state| state.registry.upsert(item.account))?;
    }

    // Refresh quotas and broadcast snapshot to update the UI
    crate::quota::refresh_all(orbit).await;

    #[cfg(target_os = "macos")]
    crate::shell::show(&orbit.app);

    Ok(Some(count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_bundle_serialization() {
        let account = Account {
            provider: Provider::Antigravity,
            desktop_ready: true,
            id: "1001".into(),
            email: "test@example.com".into(),
            name: Some("Test User".into()),
            picture: None,
            plan: None,
            added_at: Utc::now(),
        };

        let bundle = ExportBundle {
            version: 1,
            exported_at: Utc::now(),
            accounts: vec![ExportedAccount {
                account: account.clone(),
                token: Some("sample-refresh-token".into()),
            }],
        };

        let json = serde_json::to_string(&bundle).unwrap();
        let parsed: ExportBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.accounts.len(), 1);
        assert_eq!(parsed.accounts[0].account.email, "test@example.com");
        assert_eq!(
            parsed.accounts[0].token.as_deref(),
            Some("sample-refresh-token")
        );
    }
}
