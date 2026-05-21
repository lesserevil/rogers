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
pub const RODGERS_AUTO_LABELS: &[LabelDefinition] = RODGERS_REQUIRED_LABELS;

/// Additional labels Rodgers monitors (but does not auto-create).
pub const RODGERS_BACKPORT_LABELS: &[LabelDefinition] = &[
    LabelDefinition {
        name: "backport-me",
        color: "0e8a16",
        description: "Human-requested backport to active release branches",
    },
    LabelDefinition {
        name: "backport-ready",
        color: "1d76db",
        description: "Backport is approved and ready for cherry-pick",
    },
    LabelDefinition {
        name: "backport-done",
        color: "6cc644",
        description: "Backport has been applied to all target branches",
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
    "security",
    "backport-me",
    "backport-ready",
    "backport-done",
    "rodgers:type=backport",
];

/// Returns true if the given label name is a Rodgers-reserved label.
pub fn is_rodgers_reserved(name: &str) -> bool {
    RODGERS_RESERVED_LABELS.contains(&name)
}

/// Checks if an issue is a backport candidate based on its labels.
///
/// Returns `Some(priority)` where priority is:
/// - `Some(1)` for security patches (highest priority)
/// - `Some(2)` for `backport-me` labeled issues
/// - `None` for issues without backport indicators
pub fn backport_candidate_priority(labels: &[String]) -> Option<u8> {
    // Security patches get highest priority (CRIT-12 from backport-plan)
    if labels.iter().any(|l| l == "security") {
        return Some(1);
    }
    // CVE pattern in any label
    for label in labels {
        if label.starts_with("CVE-") {
            return Some(1);
        }
    }
    // backport-me label triggers manual backport request
    if labels.iter().any(|l| l == "backport-me") {
        return Some(2);
    }
    None
}

/// Checks if a commit message indicates a security fix.
///
/// Detects:
/// 1. CVE patterns: CVE-YYYY-NNNNN
/// 2. GHSA references: GHSA-xxxx-xxxx-xxxx
pub fn is_security_commit_message(message: &str) -> bool {
    let msg = message.to_lowercase();
    // Check for CVE pattern (simple string search)
    if msg.contains("cve-") && msg.contains("cve-20") {
        return true;
    }
    // Check for GHSA reference
    let ghsa_pattern = "ghsa-";
    if msg.contains(ghsa_pattern) {
        return true;
    }
    false
}
