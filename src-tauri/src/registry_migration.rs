//! Consume the pre-0.2.1 account store before normal startup. All later reads
//! and writes use the current registry and vault; clients' own stores are untouched.

use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::{secret_store, vault};

use super::{FILE, File, Provider, Registry, write_file};

const PREVIOUS_IDENTIFIER: &str = "io.github.shengyy.agyorbit";
const PREVIOUS_SERVICE: &str = "agyorbit";
const JOURNAL: &str = "accounts-migration.json";

#[derive(Default, Serialize, Deserialize)]
struct Journal {
    // Hashes only. This identifies staged copies after a crash without putting
    // credentials in a file or mistaking a failed copy for a pre-existing token.
    copies: BTreeMap<String, String>,
}

fn digest(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

trait Credentials {
    fn get(&self, service: &str, id: &str) -> Result<Option<String>>;
    fn set(&self, service: &str, id: &str, value: &str) -> Result<()>;
    fn delete(&self, service: &str, id: &str) -> Result<()>;
}

struct SystemCredentials;

impl Credentials for SystemCredentials {
    fn get(&self, service: &str, id: &str) -> Result<Option<String>> {
        secret_store::get(service, id)
    }

    fn set(&self, service: &str, id: &str, value: &str) -> Result<()> {
        secret_store::set(service, id, value)
    }

    fn delete(&self, service: &str, id: &str) -> Result<()> {
        secret_store::delete(service, id)
    }
}

pub(super) fn upgrade(current: &mut Registry) -> Result<()> {
    let previous = current
        .path
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| Error::Invalid("account data directory has no parent".into()))?
        .join(PREVIOUS_IDENTIFIER);
    migrate(current, &previous, &SystemCredentials)
}

fn merge(current: &File, previous: &File) -> Result<File> {
    let mut merged = current.clone();
    for account in &previous.accounts {
        if current
            .ignored_native
            .get(&account.provider)
            .is_some_and(|ids| ids.contains(&account.id))
        {
            continue;
        }
        if let Some(existing) = merged.accounts.iter().find(|item| item.id == account.id) {
            if existing.provider != account.provider {
                return Err(Error::Invalid(
                    "account provider conflict during upgrade".into(),
                ));
            }
        } else {
            merged.accounts.push(account.clone());
        }
    }
    if merged.live_fingerprint.is_none() {
        merged.live_fingerprint = previous.live_fingerprint.clone();
    }
    for (provider, fingerprint) in &previous.native_fingerprints {
        merged
            .native_fingerprints
            .entry(*provider)
            .or_insert_with(|| fingerprint.clone());
    }
    for (provider, ignored) in &previous.ignored_native {
        merged
            .ignored_native
            .entry(*provider)
            .or_default()
            .extend(ignored.iter().cloned());
    }
    for account in &merged.accounts {
        if let Some(ignored) = merged.ignored_native.get_mut(&account.provider) {
            ignored.remove(&account.id);
        }
    }
    Ok(merged)
}

fn migrate(
    current: &mut Registry,
    previous_dir: &Path,
    credentials: &impl Credentials,
) -> Result<()> {
    let previous_path = previous_dir.join(FILE);
    let journal_path = current.path.with_file_name(JOURNAL);
    // Read bytes first so an absent source is a no-op, while malformed or
    // inaccessible sources stop startup instead of publishing an empty list.
    let source_bytes = match std::fs::read(&previous_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // A crash after source removal can leave only the completed journal.
            match std::fs::remove_file(journal_path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    if previous_path == current.path {
        return Err(Error::Invalid(
            "account migration source equals destination".into(),
        ));
    }
    let previous = Registry::read(previous_dir)?;
    let merged = merge(&current.file, &previous.file)?;
    let google_ids: Vec<_> = previous
        .accounts()
        .iter()
        .filter(|account| account.provider == Provider::Antigravity)
        .map(|account| account.id.as_str())
        .collect();

    let mut journal: Journal = match std::fs::read(&journal_path) {
        Ok(bytes) => serde_json::from_slice(&bytes)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Journal::default(),
        Err(error) => return Err(error.into()),
    };
    // Record which credentials did not exist in the destination before writing
    // any of them. A retry can repair a partial copy, but never overwrites 0.2.1's
    // existing grants. Native sessions already use the current shared service.
    for id in &google_ids {
        if (journal.copies.contains_key(*id) || credentials.get(vault::SERVICE, id)?.is_none())
            && let Some(value) = credentials.get(PREVIOUS_SERVICE, id)?
        {
            journal.copies.insert((*id).to_owned(), digest(&value));
        }
    }
    write_file(&journal_path, &serde_json::to_vec(&journal)?)?;
    for (id, expected) in &journal.copies {
        if credentials
            .get(vault::SERVICE, id)?
            .as_deref()
            .map(digest)
            .as_ref()
            != Some(expected)
        {
            let value = credentials
                .get(PREVIOUS_SERVICE, id)?
                .filter(|value| digest(value) == *expected)
                .ok_or(Error::VerifyFailed)?;
            credentials.set(vault::SERVICE, id, &value)?;
            if credentials.get(vault::SERVICE, id)?.as_deref() != Some(value.as_str()) {
                return Err(Error::VerifyFailed);
            }
        }
    }
    if std::fs::read(&previous_path)? != source_bytes {
        return Err(Error::Invalid(
            "previous account store changed; close the older app and retry".into(),
        ));
    }
    let old_file = std::mem::replace(&mut current.file, merged);
    if let Err(error) = current.save() {
        current.file = old_file;
        return Err(error);
    }
    let readback = Registry::read(current.path.parent().unwrap())?;
    if serde_json::to_vec(&readback.file)? != serde_json::to_vec(&current.file)? {
        return Err(Error::VerifyFailed);
    }

    // A still-running older version might rotate a source grant. Detect this
    // before deleting anything; the next attempt stages its latest value.
    for id in &google_ids {
        if let Some(source) = credentials.get(PREVIOUS_SERVICE, id)? {
            let changed = match journal.copies.get(*id) {
                Some(expected) => digest(&source) != *expected,
                None => credentials.get(vault::SERVICE, id)?.is_none(),
            };
            if changed {
                return Err(Error::Invalid(
                    "previous authorization changed; close the older app and retry".into(),
                ));
            }
        }
    }

    // Commit is durable and verified before consuming the source. Keep its
    // registry until every old vault item is removed, so cleanup is resumable.
    for id in google_ids {
        credentials.delete(PREVIOUS_SERVICE, id)?;
    }
    std::fs::remove_file(previous_path)?;
    std::fs::remove_file(journal_path)?;
    // Preserve unrelated files, including WebView data, if the directory is not empty.
    match std::fs::remove_dir(previous_dir) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
        Err(error) => return Err(error.into()),
    }
    log::info!(
        "account storage upgraded ({} accounts)",
        current.accounts().len()
    );
    Ok(())
}

#[cfg(test)]
#[path = "registry_migration_tests.rs"]
mod tests;
