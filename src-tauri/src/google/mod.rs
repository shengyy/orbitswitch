//! Google APIs: OAuth sign-in and the Cloud Code quota endpoints.

pub mod cloudcode;
pub mod loopback;
pub mod oauth;

/// OrbitSwitch identifies itself on every Google request. Cloud Code also needs
/// the installed Antigravity's version; its client adds that product token.
pub const USER_AGENT: &str = concat!("OrbitSwitch/", env!("CARGO_PKG_VERSION"));
