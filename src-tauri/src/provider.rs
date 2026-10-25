//! Stable provider identities; product labels are frontend presentation.
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    #[default]
    Antigravity,
    Codex,
    Claude,
}
impl Provider {
    pub const ALL: [Self; 3] = [Self::Antigravity, Self::Codex, Self::Claude];
    pub fn key(self) -> &'static str {
        match self {
            Self::Antigravity => "antigravity",
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }
}
