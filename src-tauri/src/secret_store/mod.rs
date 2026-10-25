//! Platform credential store: macOS Keychain or Windows Credential Manager.
//!
//! Items are addressed by `(service, account)`, the same scheme go-keyring
//! uses, so OrbitSwitch can read and write the item Antigravity itself owns as well
//! as its own vault items.

use crate::error::Result;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as platform;

#[cfg(not(any(target_os = "macos", windows)))]
mod unsupported;
#[cfg(not(any(target_os = "macos", windows)))]
use unsupported as platform;

/// Returns the stored secret, or `None` when the item does not exist.
pub fn get(service: &str, account: &str) -> Result<Option<String>> {
    platform::get(service, account)
}

/// Creates or replaces the item.
pub fn set(service: &str, account: &str, secret: &str) -> Result<()> {
    platform::set(service, account, secret)
}

/// Deletes the item. Deleting a missing item is not an error.
pub fn delete(service: &str, account: &str) -> Result<()> {
    platform::delete(service, account)
}
