use std::{cell::RefCell, collections::BTreeMap};

use chrono::Utc;

use super::*;
use crate::registry::Account;

#[derive(Default)]
struct MemoryCredentials {
    values: RefCell<BTreeMap<(String, String), String>>,
    fail_set: RefCell<Option<String>>,
    fail_delete: RefCell<Option<String>>,
    corrupt_write: RefCell<bool>,
}

impl MemoryCredentials {
    fn put(&self, service: &str, id: &str, value: &str) {
        self.values
            .borrow_mut()
            .insert((service.into(), id.into()), value.into());
    }
}

impl Credentials for MemoryCredentials {
    fn get(&self, service: &str, id: &str) -> Result<Option<String>> {
        Ok(self
            .values
            .borrow()
            .get(&(service.into(), id.into()))
            .cloned())
    }

    fn set(&self, service: &str, id: &str, value: &str) -> Result<()> {
        if self.fail_set.borrow().as_deref() == Some(id) {
            return Err(Error::SecretStore("synthetic write failure".into()));
        }
        self.put(
            service,
            id,
            if *self.corrupt_write.borrow() {
                "corrupt"
            } else {
                value
            },
        );
        Ok(())
    }

    fn delete(&self, service: &str, id: &str) -> Result<()> {
        if self.fail_delete.borrow().as_deref() == Some(id) {
            return Err(Error::SecretStore("synthetic cleanup failure".into()));
        }
        self.values
            .borrow_mut()
            .remove(&(service.into(), id.into()));
        Ok(())
    }
}

struct Stores {
    _root: tempfile::TempDir,
    current: Registry,
    previous: Registry,
    credentials: MemoryCredentials,
}

impl Stores {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let identifier = config["identifier"].as_str().unwrap();
        assert_eq!(config["productName"], "OrbitSwitch");
        // These are persisted identities, independent of display branding.
        assert_eq!(identifier, "io.github.shengyy.orbitswitch");
        assert_eq!(vault::SERVICE, "orbitswitch");
        Self {
            current: Registry::read(&root.path().join(identifier)).unwrap(),
            previous: Registry::read(&root.path().join(PREVIOUS_IDENTIFIER)).unwrap(),
            credentials: MemoryCredentials::default(),
            _root: root,
        }
    }

    fn run(&mut self) -> Result<()> {
        migrate(
            &mut self.current,
            self.previous.path.parent().unwrap(),
            &self.credentials,
        )
    }

    fn old_google(&mut self, id: &str) {
        self.previous
            .upsert(account(id, Provider::Antigravity))
            .unwrap();
        self.credentials
            .put(PREVIOUS_SERVICE, id, &format!("old-{id}"));
    }
}

fn account(id: &str, provider: Provider) -> Account {
    Account {
        provider,
        desktop_ready: true,
        id: id.into(),
        email: format!("{id}@example.com"),
        name: Some(id.into()),
        picture: None,
        plan: None,
        added_at: Utc::now(),
    }
}

#[test]
fn fresh_install_does_not_create_a_registry_or_touch_credentials() {
    let mut stores = Stores::new();
    stores.run().unwrap();
    assert!(!stores.current.path.exists());
    assert!(stores.credentials.values.borrow().is_empty());
}

#[test]
fn original_google_schema_is_consumed_through_the_production_startup_path() {
    let stores = Stores::new();
    let mut original = account("ada", Provider::Antigravity);
    original.desktop_ready = false;
    let mut json = serde_json::to_value(&original).unwrap();
    json.as_object_mut().unwrap().remove("provider");
    json.as_object_mut().unwrap().remove("desktopReady");
    write_file(
        &stores.previous.path,
        serde_json::json!({"version": 1, "accounts": [json]})
            .to_string()
            .as_bytes(),
    )
    .unwrap();
    // Exercise real path discovery without accessing the system vault.
    // A native-only source has no Google entries to copy.
    let mut native = Registry::read(stores.previous.path.parent().unwrap()).unwrap();
    assert_eq!(native.accounts()[0].provider, Provider::Antigravity);
    native.file.accounts[0].provider = Provider::Codex;
    native.save().unwrap();
    let loaded = Registry::load(stores.current.path.parent().unwrap()).unwrap();
    assert_eq!(loaded.accounts()[0].id, "ada");
    assert!(!stores.previous.path.exists());
}

#[test]
fn merges_both_versions_preserves_new_grants_and_binding_then_runs_only_once() {
    let mut stores = Stores::new();
    stores.old_google("ada");
    stores.old_google("grace");
    stores
        .previous
        .upsert(account("codex-old", Provider::Codex))
        .unwrap();
    stores
        .previous
        .upsert(account("claude", Provider::Claude))
        .unwrap();
    stores
        .current
        .upsert(account("ada", Provider::Antigravity))
        .unwrap();
    stores
        .current
        .upsert(account("codex-new", Provider::Codex))
        .unwrap();
    stores.credentials.put(vault::SERVICE, "ada", "newer-ada");
    stores
        .credentials
        .put("orbitswitch.sessions", "claude", "paired-desktop-session");
    stores.run().unwrap();

    let loaded = Registry::read(stores.current.path.parent().unwrap()).unwrap();
    let ids: Vec<_> = loaded
        .accounts()
        .iter()
        .map(|account| account.id.as_str())
        .collect();
    assert_eq!(ids, ["ada", "codex-new", "grace", "codex-old", "claude"]);
    assert!(loaded.get("claude").unwrap().desktop_ready);
    assert_eq!(
        stores
            .credentials
            .get(vault::SERVICE, "ada")
            .unwrap()
            .as_deref(),
        Some("newer-ada")
    );
    assert_eq!(
        stores
            .credentials
            .get(vault::SERVICE, "grace")
            .unwrap()
            .as_deref(),
        Some("old-grace")
    );
    assert_eq!(
        stores
            .credentials
            .get("orbitswitch.sessions", "claude")
            .unwrap()
            .as_deref(),
        Some("paired-desktop-session")
    );
    assert!(
        stores
            .credentials
            .get(PREVIOUS_SERVICE, "ada")
            .unwrap()
            .is_none()
    );
    assert!(!stores.previous.path.exists());
    assert!(!stores.current.path.with_file_name(JOURNAL).exists());
    let before = std::fs::read(&stores.current.path).unwrap();
    stores.current.remove("grace").unwrap();
    stores.run().unwrap();
    assert!(stores.current.get("grace").is_none());
    assert_ne!(std::fs::read(&stores.current.path).unwrap(), before);
}

#[test]
fn preserves_fingerprints_explicit_removals_and_latest_account_metadata() {
    let mut stores = Stores::new();
    stores
        .previous
        .upsert(account("removed", Provider::Claude))
        .unwrap();
    stores
        .previous
        .upsert(account("kept", Provider::Claude))
        .unwrap();
    stores
        .previous
        .set_live_fingerprint("old-google".into())
        .unwrap();
    stores
        .previous
        .set_native_fingerprint(Provider::Claude, "old-claude".into())
        .unwrap();
    stores
        .previous
        .ignore_native(Provider::Codex, "forgotten")
        .unwrap();
    stores
        .previous
        .ignore_native(Provider::Claude, "kept")
        .unwrap();
    stores
        .current
        .ignore_native(Provider::Claude, "removed")
        .unwrap();
    let mut newest = account("kept", Provider::Claude);
    newest.name = Some("Latest".into());
    newest.desktop_ready = false;
    stores.current.upsert(newest).unwrap();
    stores
        .current
        .set_live_fingerprint("new-google".into())
        .unwrap();
    stores.run().unwrap();
    let current = &stores.current;
    assert_eq!(current.accounts().len(), 1);
    assert_eq!(current.get("kept").unwrap().name.as_deref(), Some("Latest"));
    assert!(!current.get("kept").unwrap().desktop_ready);
    assert_eq!(current.live_fingerprint(), Some("new-google"));
    assert_eq!(
        current.native_fingerprint(Provider::Claude),
        Some("old-claude")
    );
    assert!(current.ignores_native(Provider::Claude, "removed"));
    assert!(current.ignores_native(Provider::Codex, "forgotten"));
    assert!(!current.ignores_native(Provider::Claude, "kept"));
}

#[test]
fn token_write_failure_keeps_source_and_can_resume_after_partial_copies() {
    let mut stores = Stores::new();
    stores.old_google("a");
    stores.old_google("b");
    *stores.credentials.fail_set.borrow_mut() = Some("b".into());
    let source = std::fs::read(&stores.previous.path).unwrap();
    assert!(stores.run().is_err());
    assert_eq!(std::fs::read(&stores.previous.path).unwrap(), source);
    assert!(!stores.current.path.exists());
    assert_eq!(
        stores
            .credentials
            .get(PREVIOUS_SERVICE, "a")
            .unwrap()
            .as_deref(),
        Some("old-a")
    );
    assert!(stores.current.accounts().is_empty());
    *stores.credentials.fail_set.borrow_mut() = None;
    stores.run().unwrap();
    assert_eq!(stores.current.accounts().len(), 2);
    assert!(!stores.previous.path.exists());
}

#[test]
fn a_failed_readback_is_repaired_on_retry_instead_of_becoming_authoritative() {
    let mut stores = Stores::new();
    stores.old_google("ada");
    *stores.credentials.corrupt_write.borrow_mut() = true;
    assert!(matches!(stores.run(), Err(Error::VerifyFailed)));
    assert!(stores.previous.path.exists());
    assert!(!stores.current.path.exists());
    *stores.credentials.corrupt_write.borrow_mut() = false;
    stores.run().unwrap();
    assert_eq!(
        stores
            .credentials
            .get(vault::SERVICE, "ada")
            .unwrap()
            .as_deref(),
        Some("old-ada")
    );
    assert!(!stores.previous.path.exists());
}

#[test]
fn registry_publish_failure_leaves_source_untouched_and_can_retry() {
    let mut stores = Stores::new();
    stores.old_google("ada");
    std::fs::create_dir_all(&stores.current.path).unwrap();
    assert!(stores.run().is_err());
    assert!(stores.current.accounts().is_empty());
    assert!(stores.previous.path.exists());
    assert!(
        stores
            .credentials
            .get(PREVIOUS_SERVICE, "ada")
            .unwrap()
            .is_some()
    );
    std::fs::remove_dir(&stores.current.path).unwrap();
    stores.run().unwrap();
    assert_eq!(stores.current.accounts().len(), 1);
}

#[test]
fn interrupted_cleanup_resumes_even_after_some_old_credentials_are_gone() {
    let mut stores = Stores::new();
    stores.old_google("a");
    stores.old_google("b");
    *stores.credentials.fail_delete.borrow_mut() = Some("b".into());
    assert!(stores.run().is_err());
    assert_eq!(
        Registry::read(stores.current.path.parent().unwrap())
            .unwrap()
            .accounts()
            .len(),
        2
    );
    assert!(
        stores
            .credentials
            .get(PREVIOUS_SERVICE, "a")
            .unwrap()
            .is_none()
    );
    assert!(stores.previous.path.exists());
    *stores.credentials.fail_delete.borrow_mut() = None;
    stores.current = Registry::read(stores.current.path.parent().unwrap()).unwrap();
    stores.run().unwrap();
    assert_eq!(stores.current.accounts().len(), 2);
    assert!(!stores.previous.path.exists());
}

#[test]
fn malformed_or_future_source_does_not_modify_the_destination_or_credentials() {
    for bytes in [b"not json".as_slice(), br#"{"version":2,"accounts":[]}"#] {
        let mut stores = Stores::new();
        stores
            .current
            .upsert(account("current", Provider::Codex))
            .unwrap();
        write_file(&stores.previous.path, bytes).unwrap();
        let destination = std::fs::read(&stores.current.path).unwrap();
        assert!(stores.run().is_err());
        assert_eq!(std::fs::read(&stores.current.path).unwrap(), destination);
        assert_eq!(std::fs::read(&stores.previous.path).unwrap(), bytes);
        assert!(stores.credentials.values.borrow().is_empty());
    }
}

#[test]
fn missing_old_token_retains_the_account_and_unrelated_files_are_preserved() {
    let mut stores = Stores::new();
    stores
        .previous
        .upsert(account("reauth", Provider::Antigravity))
        .unwrap();
    let unrelated = stores.previous.path.with_file_name("unrelated.json");
    std::fs::write(&unrelated, b"preserve").unwrap();
    stores.run().unwrap();
    assert!(stores.current.get("reauth").is_some());
    assert!(
        stores
            .credentials
            .get(vault::SERVICE, "reauth")
            .unwrap()
            .is_none()
    );
    assert_eq!(std::fs::read(unrelated).unwrap(), b"preserve");
    assert!(!stores.previous.path.exists());
}

#[test]
fn a_completed_journal_left_by_a_crash_is_removed_on_next_startup() {
    let mut stores = Stores::new();
    stores
        .current
        .upsert(account("ada", Provider::Codex))
        .unwrap();
    let journal = stores.current.path.with_file_name(JOURNAL);
    write_file(&journal, b"{\"copies\":{}}").unwrap();
    stores.run().unwrap();
    assert_eq!(stores.current.accounts().len(), 1);
    assert!(!journal.exists());
}

#[test]
fn conflicting_provider_identity_aborts_before_writing_any_credentials() {
    let mut stores = Stores::new();
    stores.old_google("ada");
    stores
        .current
        .upsert(account("ada", Provider::Codex))
        .unwrap();
    assert!(stores.run().is_err());
    assert!(
        stores
            .credentials
            .get(vault::SERVICE, "ada")
            .unwrap()
            .is_none()
    );
    assert!(stores.previous.path.exists());
}
