//! Canonical label definitions for Rodgers.
//! All required labels are defined here so init/doctor share the same source of truth.

use serde::{Deserialize, Serialize};

/// A Rodgers-required GitHub label.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct LabelDefinition {
    pub name: &'static str,
    pub color: &'static str,
    pub description: &'static str,
}

/// All labels Rodgers expects in a managed repository.
#[allow(dead_code)]
pub const RODGERS_REQUIRED_LABELS: &[LabelDefinition] = &[
    LabelDefinition {
        name: "bug",
        color: "d73a4a",
        description: "A bug report",
    },
    LabelDefinition {
        name: "feature",
        color: "a2eeef",
        description: "A feature request",
    },
    LabelDefinition {
        name: "question",
        color: "d876e3",
        description: "A question from the community",
    },
    LabelDefinition {
        name: "needs-information",
        color: "PaleGreen",
        description: "Rodgers has asked for clarification from the requestor",
    },
    LabelDefinition {
        name: "needs-documentation",
        color: "DBAB79",
        description: "Rodgers has determined the question lacks a documentation answer",
    },
    LabelDefinition {
        name: "ready-for-review",
        color: "fbca04",
        description: "Rodgers has triaged this; awaiting human decision",
    },
    LabelDefinition {
        name: "will-not-do",
        color: "ff4444",
        description: "Human has decided this will not be worked",
    },
    LabelDefinition {
        name: "ready-for-work",
        color: "238636",
        description: "Human has approved this for implementation",
    },
    LabelDefinition {
        name: "in-progress",
        color: "1a7f37",
        description: "Work is underway",
    },
];

/// Labels Rodgers creates programmatically.
#[allow(dead_code)]
pub const RODGERS_AUTO_LABELS: &[LabelDefinition] = RODGERS_REQUIRED_LABELS;

/// All labels Rodgers should NOT conflict with (optional warning list).
#[allow(dead_code)]
pub const RODGERS_RESERVED_LABELS: &[&str] = &[
    "bug",
    "feature",
    "question",
    "needs-information",
    "needs-documentation",
    "ready-for-review",
    "will-not-do",
    "ready-for-work",
    "in-progress",
];

/// Returns true if the given label name is a Rodgers-reserved label.
#[allow(dead_code)]
pub fn is_rodgers_reserved(name: &str) -> bool {
    RODGERS_RESERVED_LABELS.contains(&name)
}
