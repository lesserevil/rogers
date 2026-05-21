#![allow(dead_code)]

//! Canonical label definitions for Rodgers.
//! All required labels are defined here so init/doctor share the same source of truth.

use serde::{Deserialize, Serialize};

/// A Rodgers-required GitHub label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelDefinition {
    pub name: &'static str,
    pub color: &'static str,
    pub description: &'static str,
}

/// All labels Rodgers expects in a managed repository.
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
pub const RODGERS_AUTO_LABELS: &[LabelDefinition] = &[
    // Triage classification labels
    LabelDefinition {
        name: "rodgers:bug",
        color: "ff9900",
        description: "Bug routed to feature-bug workflow with severity assessment",
    },
    LabelDefinition {
        name: "rodgers:question",
        color: "e99695",
        description: "Question routed to question-routing workflow",
    },
    // Severity labels (applied with bug routing)
    LabelDefinition {
        name: "severity: critical",
        color: "ff0000",
        description: "Critical severity - crash, data loss, security vulnerability",
    },
    LabelDefinition {
        name: "severity: high",
        color: "d73a4a",
        description: "High severity - broken feature, major functionality impaired",
    },
    LabelDefinition {
        name: "severity: medium",
        color: "fbca04",
        description: "Medium severity - minor issue, degraded functionality",
    },
    LabelDefinition {
        name: "severity: low",
        color: "0e8a16",
        description: "Low severity - cosmetic issue, minor UI problems",
    },
    // Existing auto labels
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

/// All labels Rodgers should NOT conflict with (optional warning list).
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
    "rodgers:bug",
    "rodgers:question",
    "severity: critical",
    "severity: high",
    "severity: medium",
    "severity: low",
];

/// Returns true if the given label name is a Rodgers-reserved label.
pub fn is_rodgers_reserved(name: &str) -> bool {
    RODGERS_RESERVED_LABELS.contains(&name)
}
