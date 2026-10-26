//! Account metadata, persisted as `accounts.json` in the app data directory.
//! Holds no secrets: refresh tokens live in the vault.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::Result;
use crate::google::cloudcode::Plan;
use crate::provider::Provider;
use std::collections::{BTreeMap, BTreeSet};

#[path = "registry_migration.rs"]
mod migration;

const FILE: &str = "accounts.json";
const VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    #[serde(default)]
    pub provider: Provider,
    #[serde(default)]
    pub desktop_ready: bool,
    /// Google `sub`, or a provider/user/workspace identity for native clients.
    pub id: String,
    pub email: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub picture: Option<String>,
    #[serde(default)]
    pub plan: Option<Plan>,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct File {
    version: u32,
    accounts: Vec<Account>,
    /// Fingerprint of the IDE refresh token last seen, so a new sign-in made
    /// inside Antigravity is adopted once and a removed account stays removed.
    #[serde(default)]
    live_fingerprint: Option<String>,
    #[serde(default)]
    native_fingerprints: BTreeMap<Provider, String>,
    #[serde(default)]
    ignored_native: BTreeMap<Provider, BTreeSet<String>>,
}

pub struct Registry {
    path: PathBuf,
    file: File,
}

/// Short, non-reversible fingerprint of a refresh token.
pub fn fingerprint(refresh_token: &str) -> String {
    Sha256::digest(refresh_token.as_bytes())[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl Registry {
    pub fn load(dir: &Path) -> Result<Registry> {
        let mut registry = Self::read(dir)?;
        migration::upgrade(&mut registry)?;
        Ok(registry)
    }

    fn read(dir: &Path) -> Result<Registry> {
        let path = dir.join(FILE);
        let file: File = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => File {
                version: VERSION,
                ..File::default()
            },
            Err(err) => return Err(err.into()),
        };
        if file.version != VERSION {
            return Err(crate::error::Error::Invalid(
                "unsupported account registry version".into(),
            ));
        }
        Ok(Registry { path, file })
    }

    pub fn accounts(&self) -> &[Account] {
        &self.file.accounts
    }

    pub fn get(&self, id: &str) -> Option<&Account> {
        self.file.accounts.iter().find(|a| a.id == id)
    }

    /// Inserts a new account or refreshes an existing one in place, keeping
    /// its position, `added_at` and known plan.
    pub fn upsert(&mut self, account: Account) -> Result<()> {
        let previous = self.file.clone();
        if let Some(ignored) = self.file.ignored_native.get_mut(&account.provider) {
            ignored.remove(&account.id);
        }
        match self.file.accounts.iter_mut().find(|a| a.id == account.id) {
            Some(existing) => {
                existing.email = account.email;
                existing.desktop_ready = account.desktop_ready;
                existing.name = account.name.or(existing.name.take());
                existing.picture = account.picture.or(existing.picture.take());
                existing.plan = account.plan.or(existing.plan.take());
            }
            None => self.file.accounts.push(account),
        }
        if let Err(error) = self.save() {
            self.file = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn set_plan(&mut self, id: &str, plan: Plan) -> Result<()> {
        match self.file.accounts.iter_mut().find(|a| a.id == id) {
            Some(account) if account.plan.as_ref() != Some(&plan) => {
                account.plan = Some(plan);
                self.save()
            }
            _ => Ok(()),
        }
    }

    pub fn remove(&mut self, id: &str) -> Result<()> {
        self.file.accounts.retain(|a| a.id != id);
        self.save()
    }

    pub fn live_fingerprint(&self) -> Option<&str> {
        self.file.live_fingerprint.as_deref()
    }

    pub fn set_live_fingerprint(&mut self, fingerprint: String) -> Result<()> {
        if self.file.live_fingerprint.as_deref() == Some(fingerprint.as_str()) {
            return Ok(());
        }
        self.file.live_fingerprint = Some(fingerprint);
        self.save()
    }

    pub fn ignores_native(&self, provider: Provider, id: &str) -> bool {
        self.file
            .ignored_native
            .get(&provider)
            .is_some_and(|values| values.contains(id))
    }
    pub fn ignore_native(&mut self, provider: Provider, id: &str) -> Result<()> {
        let previous = self.file.ignored_native.clone();
        self.file
            .ignored_native
            .entry(provider)
            .or_default()
            .insert(id.into());
        if let Err(error) = self.save() {
            self.file.ignored_native = previous;
            return Err(error);
        }
        Ok(())
    }
    pub fn native_fingerprint(&self, provider: Provider) -> Option<&str> {
        self.file
            .native_fingerprints
            .get(&provider)
            .map(String::as_str)
    }
    pub fn set_native_fingerprint(&mut self, provider: Provider, value: String) -> Result<()> {
        let previous = self.file.native_fingerprints.clone();
        self.file.native_fingerprints.insert(provider, value);
        if let Err(error) = self.save() {
            self.file.native_fingerprints = previous;
            return Err(error);
        }
        Ok(())
    }

    fn save(&self) -> Result<()> {
        write_file(&self.path, &serde_json::to_vec_pretty(&self.file)?)
    }
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;

    let dir = path.parent().expect("registry path has a parent");
    fs::create_dir_all(dir)?;
    let tmp = path.with_extension("json.tmp");
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, path)?;
        #[cfg(unix)]
        fs::File::open(dir)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(id: &str, name: Option<&str>) -> Account {
        Account {
            provider: Provider::Antigravity,
            desktop_ready: true,
            id: id.into(),
            email: format!("{id}@example.com"),
            name: name.map(Into::into),
            picture: None,
            plan: None,
            added_at: Utc::now(),
        }
    }

    #[test]
    fn upsert_keeps_order_and_known_fields() {
        let dir = std::env::temp_dir().join(format!("orbitswitch-registry-{}", std::process::id()));
        let mut registry = Registry::load(&dir).unwrap();
        registry.upsert(account("a", Some("Ada"))).unwrap();
        registry.upsert(account("b", None)).unwrap();
        registry.upsert(account("a", None)).unwrap();
        let reloaded = Registry::load(&dir).unwrap();
        let ids: Vec<_> = reloaded.accounts().iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(reloaded.get("a").unwrap().name.as_deref(), Some("Ada"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn removing_another_account_does_not_readopt_a_forgotten_live_signin() {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = Registry::load(dir.path()).unwrap();
        registry.ignore_native(Provider::Claude, "a").unwrap();
        registry.ignore_native(Provider::Claude, "b").unwrap();
        let mut imported = account("b", None);
        imported.provider = Provider::Claude;
        registry.upsert(imported).unwrap();
        let reloaded = Registry::load(dir.path()).unwrap();
        assert!(reloaded.ignores_native(Provider::Claude, "a"));
        assert!(!reloaded.ignores_native(Provider::Claude, "b"));
    }
}
