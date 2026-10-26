//! Per-account refresh tokens, kept in the platform credential store under
//! service `orbitswitch` and keyed by the Google account's stable `sub`.

use crate::error::Result;
use crate::secret_store;

pub(crate) const SERVICE: &str = "orbitswitch";

pub fn get(account_id: &str) -> Result<Option<String>> {
    secret_store::get(SERVICE, account_id)
}

pub fn set(account_id: &str, refresh_token: &str) -> Result<()> {
    secret_store::set(SERVICE, account_id, refresh_token)
}

pub fn delete(account_id: &str) -> Result<()> {
    secret_store::delete(SERVICE, account_id)
}
