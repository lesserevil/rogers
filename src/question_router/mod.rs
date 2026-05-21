//! Question Router module.
//!
//! Routes GitHub issues labeled `question` through the documentation and code
//! search workflow. Searches docs and source code for answers before filing
//! doc-gap beads.
//!
//! Plan: plans/question-routing-plan.md §Question Router Decision Tree

pub mod code_search;
pub mod doc_gap;
pub mod doc_search;
pub mod router;

pub use router::QuestionRouter;
