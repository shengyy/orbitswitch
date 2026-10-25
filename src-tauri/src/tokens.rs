//! Short-lived access tokens per account, kept only in memory.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Refresh window before access token expiry.
const MARGIN: Duration = Duration::from_secs(120);

#[derive(Default)]
pub struct TokenCache {
    tokens: Mutex<HashMap<String, (String, Instant)>>,
}

impl TokenCache {
    pub fn get(&self, account_id: &str) -> Option<String> {
        let tokens = self.tokens.lock().expect("token cache poisoned");
        tokens
            .get(account_id)
            .filter(|(_, expires)| *expires > Instant::now() + MARGIN)
            .map(|(token, _)| token.clone())
    }

    pub fn put(&self, account_id: &str, token: String, expires_in_secs: i64) {
        let expires = Instant::now() + Duration::from_secs(expires_in_secs.max(0) as u64);
        self.tokens
            .lock()
            .expect("token cache poisoned")
            .insert(account_id.to_owned(), (token, expires));
    }

    pub fn forget(&self, account_id: &str) {
        self.tokens
            .lock()
            .expect("token cache poisoned")
            .remove(account_id);
    }
}
