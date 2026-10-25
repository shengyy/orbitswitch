//! Placeholder for platforms OrbitSwitch does not support yet.

use crate::error::{Error, Result};

fn unsupported() -> Error {
    Error::SecretStore("this platform has no supported credential store".into())
}

pub fn get(_service: &str, _account: &str) -> Result<Option<String>> {
    Err(unsupported())
}

pub fn set(_service: &str, _account: &str, _secret: &str) -> Result<()> {
    Err(unsupported())
}

pub fn delete(_service: &str, _account: &str) -> Result<()> {
    Err(unsupported())
}
