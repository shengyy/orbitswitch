//! Native credential locations and lossless rollback. No secret reaches IPC.
use super::storage;
use crate::{
    error::{Error, Result},
    provider::Provider,
    secret_store,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use unicode_normalization::UnicodeNormalization;
#[derive(Clone)]
pub struct Location {
    pub provider: Provider,
    pub root: PathBuf,
    pub file: PathBuf,
    pub metadata: Option<PathBuf>,
    pub keyring: Option<(String, String)>,
    pub keyring_only: bool,
}
fn home() -> Result<PathBuf> {
    std::env::home_dir().ok_or_else(|| Error::Invalid("home directory unavailable".into()))
}
pub fn locate(provider: Provider) -> Result<Location> {
    let root = match provider {
        Provider::Codex => std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or(home()?.join(".codex")),
        Provider::Claude => std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or(home()?.join(".claude")),
        _ => return Err(Error::Invalid("not a native CLI provider".into())),
    };
    location(provider, root, false)
}
pub fn location(provider: Provider, root: PathBuf, isolated: bool) -> Result<Location> {
    if provider == Provider::Codex {
        let store = if isolated {
            "file".to_owned()
        } else {
            match std::fs::read_to_string(root.join("config.toml")) {
                Ok(raw) => {
                    let config: toml::Value = toml::from_str(&raw).map_err(|_| {
                        Error::Invalid("could not parse Codex configuration".into())
                    })?;
                    let store = config
                        .get("cli_auth_credentials_store")
                        .and_then(toml::Value::as_str)
                        .unwrap_or("file");
                    if store != "file"
                        && config
                            .get("features")
                            .and_then(|v| v.get("secret_auth_storage"))
                            .and_then(toml::Value::as_bool)
                            == Some(true)
                    {
                        return Err(Error::Invalid("encrypted Codex secret storage is not supported; use file or direct keyring authentication".into()));
                    }
                    store.to_owned()
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => "file".into(),
                Err(e) => return Err(e.into()),
            }
        };
        if !matches!(store.as_str(), "file" | "keyring" | "auto") {
            return Err(Error::Invalid(
                "unsupported Codex credential storage mode".into(),
            ));
        }
        let canonical = root.canonicalize().unwrap_or(root.clone());
        let hash = hex(&Sha256::digest(canonical.to_string_lossy().as_bytes()));
        return Ok(Location {
            provider,
            file: root.join("auth.json"),
            metadata: None,
            keyring: matches!(store.as_str(), "keyring" | "auto")
                .then(|| ("Codex Auth".into(), format!("cli|{}", &hash[..16]))),
            keyring_only: store == "keyring",
            root,
        });
    }
    let custom = isolated || std::env::var_os("CLAUDE_CONFIG_DIR").is_some();
    let storage_dir = if isolated {
        Some(root.to_string_lossy().into_owned())
    } else {
        std::env::var("CLAUDE_SECURESTORAGE_CONFIG_DIR")
            .ok()
            .or_else(|| std::env::var("CLAUDE_CONFIG_DIR").ok())
    };
    let suffix = storage_dir
        .filter(|s| !s.is_empty())
        .map(|s| {
            format!(
                "-{}",
                &hex(&Sha256::digest(s.nfc().collect::<String>().as_bytes()))[..8]
            )
        })
        .unwrap_or_default();
    let account = std::env::var("USER").unwrap_or_else(|_| "claude-code-user".into());
    Ok(Location {
        provider,
        file: root.join(".credentials.json"),
        metadata: Some(if custom {
            root.join(".claude.json")
        } else {
            home()?.join(".claude.json")
        }),
        keyring: cfg!(target_os = "macos")
            .then(|| (format!("Claude Code-credentials{suffix}"), account)),
        keyring_only: false,
        root,
    })
}
impl Location {
    fn secret_get(&self, service: &str, account: &str) -> Result<Option<String>> {
        #[cfg(windows)]
        if self.provider == Provider::Codex {
            let entry = keyring::Entry::new(service, account)
                .map_err(|_| Error::SecretStore("could not open Codex keyring entry".into()))?;
            return match entry.get_password() {
                Ok(value) => Ok(Some(value)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(_) => Err(Error::SecretStore(
                    "could not read Codex keyring entry".into(),
                )),
            };
        }
        secret_store::get(service, account)
    }
    fn secret_set(&self, service: &str, account: &str, value: &str) -> Result<()> {
        #[cfg(windows)]
        if self.provider == Provider::Codex {
            return keyring::Entry::new(service, account)
                .and_then(|entry| entry.set_password(value))
                .map_err(|_| Error::SecretStore("could not write Codex keyring entry".into()));
        }
        secret_store::set(service, account, value)
    }
    fn secret_delete(&self, service: &str, account: &str) -> Result<()> {
        #[cfg(windows)]
        if self.provider == Provider::Codex {
            return match keyring::Entry::new(service, account)
                .and_then(|entry| entry.delete_credential())
            {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(_) => Err(Error::SecretStore(
                    "could not remove Codex keyring entry".into(),
                )),
            };
        }
        secret_store::delete(service, account)
    }
    pub fn check_switch_configuration(&self, desktop_installed: bool) -> Result<()> {
        let overrides: &[&str] = if self.provider == Provider::Codex {
            &["OPENAI_API_KEY", "CODEX_ACCESS_TOKEN"]
        } else {
            &[
                "ANTHROPIC_API_KEY",
                "ANTHROPIC_AUTH_TOKEN",
                "CLAUDE_CODE_OAUTH_TOKEN",
                "CLAUDE_CODE_USE_BEDROCK",
                "CLAUDE_CODE_USE_VERTEX",
                "CLAUDE_CODE_USE_FOUNDRY",
            ]
        };
        if overrides
            .iter()
            .any(|key| std::env::var(key).is_ok_and(|value| !value.is_empty() && value != "0"))
        {
            return Err(Error::Invalid("an environment override controls client authentication; remove it before subscription switching".into()));
        }
        if self.provider == Provider::Codex {
            if let Ok(raw) = std::fs::read_to_string(self.root.join("config.toml")) {
                let config: toml::Value = toml::from_str(&raw)
                    .map_err(|_| Error::Invalid("invalid Codex configuration".into()))?;
                if config
                    .get("model_provider")
                    .and_then(toml::Value::as_str)
                    .is_some_and(|p| p != "openai")
                {
                    return Err(Error::Invalid(
                        "Codex is configured for a custom API provider".into(),
                    ));
                }
            }
            if desktop_installed
                && self.root.canonicalize().ok() != home()?.join(".codex").canonicalize().ok()
            {
                return Err(Error::Invalid("desktop switching requires the default Codex home; custom CLI profiles remain independent".into()));
            }
        } else if let Some(settings) = storage::read_json(&self.root.join("settings.json"))?
            && overrides.iter().any(|key| {
                settings["env"][*key]
                    .as_str()
                    .is_some_and(|v| !v.is_empty() && v != "0")
            })
        {
            return Err(Error::Invalid(
                "Claude settings override subscription authentication".into(),
            ));
        }
        Ok(())
    }
    pub fn read(&self) -> Result<Option<Value>> {
        if let Some((service, account)) = &self.keyring {
            match self.secret_get(service, account) {
                Ok(Some(raw)) => return Ok(Some(serde_json::from_str(&raw)?)),
                Err(e) => return Err(e),
                _ => {}
            }
        }
        if self.keyring_only {
            return Ok(None);
        }
        storage::read_json(&self.file)
    }
    pub fn write(&self, credential: &Value, profile: &Value) -> Result<()> {
        let mut value = if self.provider == Provider::Claude {
            self.read()?.unwrap_or_else(|| serde_json::json!({}))
        } else {
            credential.clone()
        };
        if self.provider == Provider::Claude {
            value["claudeAiOauth"] = credential["claudeAiOauth"].clone();
        }
        let mut in_keyring = false;
        if let Some((service, account)) = &self.keyring {
            match self.secret_set(service, account, &serde_json::to_string(&value)?) {
                Ok(()) => in_keyring = true,
                Err(e) => return Err(e),
            }
        }
        if self.provider == Provider::Codex && in_keyring {
            storage::restore_file(&self.file, None)?;
        } else if !in_keyring || self.file.exists() {
            storage::write_json(&self.file, &value)?;
        }
        if let Some(path) = &self.metadata {
            let mut metadata = storage::read_json(path)?.unwrap_or_else(|| serde_json::json!({}));
            let a = &profile["account"];
            let o = &profile["organization"];
            metadata["oauthAccount"] = serde_json::json!({"accountUuid":a["uuid"],"emailAddress":a["email"],"organizationUuid":o["uuid"],"displayName":a["display_name"],"fullName":a["full_name"],"organizationName":o["name"],"organizationType":o["organization_type"],"organizationRateLimitTier":o["rate_limit_tier"],"hasExtraUsageEnabled":o["has_extra_usage_enabled"]});
            storage::write_json(path, &metadata)?;
        }
        Ok(())
    }
}
pub struct Backup {
    location: Location,
    keyring: Option<String>,
    file: Option<Value>,
    metadata: Option<Value>,
}
impl Backup {
    pub fn take(location: &Location) -> Result<Self> {
        Ok(Self {
            location: location.clone(),
            keyring: location
                .keyring
                .as_ref()
                .map(|(s, a)| location.secret_get(s, a))
                .transpose()?
                .flatten(),
            file: storage::read_json(&location.file)?,
            metadata: location
                .metadata
                .as_ref()
                .map(|p| storage::read_json(p))
                .transpose()?
                .flatten(),
        })
    }
    pub fn restore(&self) -> Result<()> {
        let mut result = Ok(());
        if let Some((s, a)) = &self.location.keyring {
            result = match &self.keyring {
                Some(raw) => self.location.secret_set(s, a, raw),
                None => self.location.secret_delete(s, a),
            };
        }
        let file = storage::restore_file(&self.location.file, self.file.as_ref());
        let metadata = self
            .location
            .metadata
            .as_ref()
            .map(|path| storage::restore_file(path, self.metadata.as_ref()))
            .transpose();
        result.and(file).and(metadata.map(|_| ()))
    }
}
pub fn cleanup(location: &Location) {
    if let Some((s, a)) = &location.keyring {
        let _ = location.secret_delete(s, a);
    }
    let _ = std::fs::remove_dir_all(&location.root);
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_file_rollback_preserves_unrelated_settings() {
        let dir = tempfile::tempdir().unwrap();
        let location = Location {
            provider: Provider::Claude,
            root: dir.path().into(),
            file: dir.path().join(".credentials.json"),
            metadata: Some(dir.path().join(".claude.json")),
            keyring: None,
            keyring_only: false,
        };
        let old =
            serde_json::json!({"claudeAiOauth":{"accessToken":"fixture-a"},"unrelated":"keep"});
        let meta = serde_json::json!({"oauthAccount":{"accountUuid":"a"},"projects":{"sample":{"allowed":true}}});
        storage::write_json(&location.file, &old).unwrap();
        storage::write_json(location.metadata.as_ref().unwrap(), &meta).unwrap();
        let backup = Backup::take(&location).unwrap();
        location.write(&serde_json::json!({"claudeAiOauth":{"accessToken":"fixture-b"}}), &serde_json::json!({"account":{"uuid":"b","email":"b@example.com"},"organization":{"uuid":"org-b"}})).unwrap();
        assert_eq!(location.read().unwrap().unwrap()["unrelated"], "keep");
        assert_eq!(
            storage::read_json(location.metadata.as_ref().unwrap())
                .unwrap()
                .unwrap()["projects"],
            meta["projects"]
        );
        backup.restore().unwrap();
        assert_eq!(location.read().unwrap().unwrap(), old);
        assert_eq!(
            storage::read_json(location.metadata.as_ref().unwrap())
                .unwrap()
                .unwrap(),
            meta
        );
    }
    #[test]
    fn unsupported_codex_config_cannot_fall_back_to_another_store() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "cli_auth_credentials_store = 'future-store'\n",
        )
        .unwrap();
        assert!(location(Provider::Codex, dir.path().into(), false).is_err());
    }
}
