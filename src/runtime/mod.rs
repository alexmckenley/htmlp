//! Typed model requests and runtime accounting, independent of any provider SDK.
//! Every fragment requires a source and role. Estimates, tokenizer-measured
//! document budgets, and reported usage are distinct; none implies the others.
mod request;
mod usage;
pub use request::*;
use serde::{Deserialize, Serialize};
pub use usage::*;

/// Message role; source attribution remains independent of role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

/// Opaque provider-neutral tool-call identity. The caller generates IDs;
/// HTMLP has no UUID, clock, or random-number dependency.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct ToolUseId(String);
impl ToolUseId {
    pub fn from_string(value: String) -> Self {
        Self(value)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ToolUseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
